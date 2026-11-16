//! `dify-review review`: judge one pull request's hunks against the rules
//! and print, or post, the inline comments. `Reviewer` is shared with
//! `watch`, which reviews the pull requests someone mentioned the bot on.

use std::collections::{HashMap, HashSet};
use std::fs::{self, OpenOptions};
use std::io::Write;
use std::path::PathBuf;
use std::sync::Mutex;

use anyhow::Result;
use dify_review_core::Repo;
use dify_review_core::diff::Hunk;
use dify_review_core::review::{
    Finding, SummaryFacts, Verdict, decide_at, decide_by_hint, posting_threshold, summary_body,
};
use dify_review_core::rules::{Rule, RuleId, RuleSet};
use dify_review_github::GitHub;
use dify_review_github::pr::{self, InlineComment, PullRequest, ReviewEvent};
use dify_review_jev::{Jev, MAX_PATCH_CHARS, Usage, hunk_state};
use futures::stream::{self, StreamExt};

/// Hunks of one pull request judged at once.
const JEV_CONCURRENCY: usize = 8;
/// Comments one rule may leave on one pull request.
const MAX_PER_RULE: usize = 2;
/// Jev price per input token, for the cost line (output is free).
const USD_PER_INPUT_TOKEN: f64 = 0.042 / 1_000_000.0;

#[derive(clap::Args)]
pub struct ReviewArgs {
    /// Repository as OWNER/NAME.
    #[arg(long, default_value = "langgenius/dify")]
    repo: Repo,
    /// Pull request number.
    #[arg(long)]
    pr: u64,
    #[command(flatten)]
    opts: ReviewOpts,
    /// Review even if this head was already reviewed.
    #[arg(long)]
    force: bool,
}

#[derive(clap::Args, Clone)]
pub struct ReviewOpts {
    /// Directory of rule files (`<id>.toml`).
    #[arg(long, default_value = "rules")]
    rules: PathBuf,
    /// Post at most this many comments, highest score first.
    #[arg(long, default_value_t = 15)]
    max_comments: usize,
    /// Judge by each rule's hint regex only, even with a Jev key.
    #[arg(long)]
    hints_only: bool,
    /// Post the review. Without this flag nothing is written to GitHub.
    #[arg(long)]
    post: bool,
    /// Heads reviewed without posting anything, so they are not judged again.
    #[arg(long, default_value = "data/reviewed-heads.txt")]
    state: PathBuf,
}

pub async fn run(a: ReviewArgs) -> Result<()> {
    let reviewer = Reviewer::new(a.repo, a.opts).await?;
    reviewer.review(a.pr, None, a.force).await
}

pub struct Reviewer {
    pub gh: GitHub,
    pub repo: Repo,
    jev: Option<Jev>,
    rules: RuleSet,
    opts: ReviewOpts,
    /// Heads reviewed without a posted review (see `remember`).
    reviewed: Mutex<HashSet<String>>,
}

/// Who asked for a review, if someone did.
pub struct Ask<'a> {
    pub comment_id: u64,
    pub author: &'a str,
}

impl Reviewer {
    pub async fn new(repo: Repo, opts: ReviewOpts) -> Result<Self> {
        let rules = RuleSet::load_dir(&opts.rules)?;
        let jev = match opts.hints_only {
            true => None,
            false => Jev::from_env()?,
        };
        match &jev {
            Some(j) => tracing::info!(
                base = j.base_url(),
                rules = rules.rules().len(),
                "judging with Jev"
            ),
            None => tracing::warn!(
                "no Jev (TYPESAFE_API_KEY unset or --hints-only): judging by hint regexes; choice rules stay silent"
            ),
        }
        let gh = GitHub::from_env().await?;
        let reviewed = match opts.state.exists() {
            true => fs::read_to_string(&opts.state)?
                .lines()
                .map(|l| l.trim().to_owned())
                .filter(|l| !l.is_empty())
                .collect(),
            false => HashSet::new(),
        };
        if let Some(dir) = opts.state.parent().filter(|d| !d.as_os_str().is_empty()) {
            fs::create_dir_all(dir)?;
        }
        Ok(Self {
            reviewed: Mutex::new(reviewed),
            gh,
            repo,
            jev,
            rules,
            opts,
        })
    }

    /// Review one pull request. Each head is judged once: Jev's scores move a
    /// little between calls, so judging an unchanged head again could add a
    /// comment nobody's change caused. `force` (or an explicit ask) overrides.
    pub async fn review(&self, number: u64, ask: Option<Ask<'_>>, force: bool) -> Result<()> {
        let pr = pr::pull_request(&self.gh, &self.repo, number).await?;
        let history = pr::history(&self.gh, &self.repo, number).await?;
        let seen = history.heads.contains(&pr.head.0)
            || self
                .reviewed
                .lock()
                .expect("reviewed-heads lock")
                .contains(&pr.head.0);
        let asked = force || ask.is_some();
        match (seen, asked) {
            _ if !pr.open => {
                eprintln!("PR #{number}: not open, skipped");
                Ok(())
            }
            _ if pr.draft && !asked => {
                eprintln!("PR #{number}: draft, skipped until ready for review");
                Ok(())
            }
            (true, false) => self.recheck(&pr, &history).await,
            _ => {
                let failing = pr::failing_checks(&self.gh, &self.repo, &pr.head).await?;
                self.judge_and_post(&pr, ask, &failing).await
            }
        }
    }

    /// A head already reviewed: only the cheap checks, no Jev. Since the
    /// review, the base may have moved on (a merge conflict) or CI may have
    /// finished and failed; each is flagged once per head.
    async fn recheck(&self, pr: &PullRequest, history: &pr::History) -> Result<()> {
        let head = &pr.head.0;
        let conflict = pr.conflicted() && !history.conflicts.contains(head);
        let failing = match history.ci.contains(head) {
            true => Vec::new(),
            false => pr::failing_checks(&self.gh, &self.repo, &pr.head).await?,
        };
        match (conflict, failing.is_empty()) {
            (false, true) => {
                eprintln!(
                    "PR #{}: head {} already reviewed, nothing new",
                    pr.number,
                    short(head)
                );
                Ok(())
            }
            _ => {
                eprintln!(
                    "PR #{}: head {}{}{}",
                    pr.number,
                    short(head),
                    if conflict { " now conflicts" } else { "" },
                    if failing.is_empty() {
                        ""
                    } else {
                        ", CI failing"
                    }
                );
                let ci = ci_list(&failing);
                let facts = SummaryFacts {
                    head,
                    new: 0,
                    still_open: 0,
                    conflict_base: conflict.then_some(pr.base_ref.as_str()),
                    ci_failing: ci.as_deref(),
                    requester: None,
                    trigger: None,
                };
                self.post(pr, &facts, &[], ReviewEvent::RequestChanges)
                    .await
            }
        }
    }

    /// Review a head: resolve the bot's comments the push made outdated,
    /// judge the hunks, and post one review. It requests changes when there
    /// is a new suggestion, an earlier one still open, or a merge conflict;
    /// otherwise it says nothing is left and dismisses the bot's earlier
    /// change requests.
    async fn judge_and_post(
        &self,
        pr: &PullRequest,
        ask: Option<Ask<'_>>,
        failing: &[pr::FailingCheck],
    ) -> Result<()> {
        let threads = pr::review_threads(&self.gh, &self.repo, pr.number).await?;
        let outdated: Vec<_> = threads
            .iter()
            .filter(|t| t.bot && t.outdated && !t.resolved)
            .collect();
        if self.opts.post {
            for t in &outdated {
                pr::resolve_thread(&self.gh, &t.id).await?;
            }
        }
        if !outdated.is_empty() {
            eprintln!(
                "PR #{}: {} earlier comment(s) outdated by the push {}",
                pr.number,
                outdated.len(),
                if self.opts.post {
                    "resolved"
                } else {
                    "would be resolved"
                }
            );
        }
        // Dedupe only against comments still standing on current lines: a
        // problem that survived a push under an outdated comment is raised again.
        let posted: HashSet<String> = threads
            .iter()
            .filter(|t| t.bot && !t.outdated)
            .filter_map(|t| t.key.clone())
            .collect();
        // Unresolved threads by anyone keep the review open; the bot's own
        // outdated ones were just resolved above.
        let still_open = threads
            .iter()
            .filter(|t| !t.resolved && !(t.bot && t.outdated))
            .count();

        let files = pr::changed_files(&self.gh, &self.repo, pr.number).await?;
        tracing::info!(pr = pr.number, title = %pr.title, files = files.len(), "reviewing");
        let jobs: Vec<(&str, &Hunk, Vec<&Rule>)> = files
            .iter()
            .flat_map(|file| {
                let applicable: Vec<&Rule> = self
                    .rules
                    .rules()
                    .iter()
                    .filter(|r| r.applies_to(&file.path))
                    .collect();
                let any_rule = !applicable.is_empty();
                file.hunks
                    .iter()
                    .filter(move |h| h.has_additions() && any_rule)
                    .map(move |h| (file.path.as_str(), h, applicable.clone()))
            })
            .collect();
        let judged: Vec<_> = stream::iter(jobs)
            .map(|(path, hunk, rules)| async move {
                let (verdicts, usage, asked) =
                    judge(self.jev.as_ref(), pr, path, hunk, &rules).await;
                let found: Vec<Finding> = verdicts
                    .into_iter()
                    .filter_map(|(rule, v)| Finding::new(rule, path, hunk, v))
                    .collect();
                (found, usage, asked)
            })
            .buffer_unordered(JEV_CONCURRENCY)
            .collect()
            .await;
        let (findings, usage, asked) = judged.into_iter().fold(
            (Vec::new(), Usage::default(), 0usize),
            |(mut all, mut total, n), (found, u, a)| {
                all.extend(found);
                total.input_tokens += u.input_tokens;
                total.output_tokens += u.output_tokens;
                (all, total, n + a)
            },
        );
        let selected = select(findings, &posted, self.opts.max_comments);
        report(pr, &selected, &posted, asked, usage);

        let conflict = pr.conflicted();
        let ci = ci_list(failing);
        let facts = SummaryFacts {
            head: &pr.head.0,
            new: selected.len(),
            still_open,
            conflict_base: conflict.then_some(pr.base_ref.as_str()),
            ci_failing: ci.as_deref(),
            requester: ask.as_ref().map(|a| a.author),
            trigger: ask.as_ref().map(|a| a.comment_id),
        };
        let comments: Vec<_> = selected
            .iter()
            .map(|f| InlineComment {
                path: f.path.clone(),
                line: f.line,
                body: f.body(),
            })
            .collect();
        // Post only what the author must act on: new suggestions, a conflict,
        // failing CI. Unresolved threads alone are already visible on the pull
        // request. An explicit ask always gets an answer.
        let act_on = facts.new > 0 || conflict || ci.is_some();
        let standing = match act_on {
            true => Vec::new(),
            false => pr::bot_change_requests(&self.gh, &self.repo, pr.number).await?,
        };
        let clear = still_open == 0;
        match (act_on, ask.is_some(), clear && !standing.is_empty()) {
            (true, _, _) => {
                self.post(pr, &facts, &comments, ReviewEvent::RequestChanges)
                    .await
            }
            // Everything was dealt with: say so, and lift the earlier change request.
            (false, _, true) => {
                self.post(pr, &facts, &[], ReviewEvent::Comment).await?;
                if self.opts.post {
                    let msg = format!("Nothing left to flag at {}.", short(&pr.head.0));
                    match pr::dismiss_reviews(&self.gh, &self.repo, pr.number, &standing, &msg)
                        .await
                    {
                        Ok(()) => {
                            eprintln!("dismissed {} earlier change request(s)", standing.len())
                        }
                        Err(e) => {
                            tracing::warn!(pr = pr.number, error = %e, "could not dismiss earlier change requests")
                        }
                    }
                }
                Ok(())
            }
            (false, true, false) => self.post(pr, &facts, &[], ReviewEvent::Comment).await,
            (false, false, false) => {
                eprintln!(
                    "PR #{}: nothing to act on at {}{}; not posting",
                    pr.number,
                    short(&pr.head.0),
                    match still_open {
                        0 => String::new(),
                        n => format!(" ({n} unresolved thread(s) remain)"),
                    }
                );
                self.remember(&pr.head.0)
            }
        }
    }

    /// Record a head reviewed without posting, so the next pass does not
    /// judge it again. Heads with a posted review are recorded in its marker.
    fn remember(&self, head: &str) -> Result<()> {
        match self.opts.post {
            false => Ok(()),
            true => {
                self.reviewed
                    .lock()
                    .expect("reviewed-heads lock")
                    .insert(head.to_owned());
                let mut f = OpenOptions::new()
                    .create(true)
                    .append(true)
                    .open(&self.opts.state)?;
                writeln!(f, "{head}")?;
                Ok(())
            }
        }
    }

    async fn post(
        &self,
        pr: &PullRequest,
        facts: &SummaryFacts<'_>,
        comments: &[InlineComment],
        event: ReviewEvent,
    ) -> Result<()> {
        let body = summary_body(&self.repo, pr.number, facts);
        match self.opts.post {
            false => {
                eprintln!(
                    "\ndry run: would post a {event:?} review: {}",
                    body.lines().next().unwrap_or_default()
                );
                Ok(())
            }
            true => {
                let review =
                    pr::post_review(&self.gh, &self.repo, pr, &body, comments, event).await?;
                eprintln!("\nposted ({event:?}): {}", review.html_url);
                Ok(())
            }
        }
    }
}

/// Failing checks as markdown links, or None when CI is not failing.
fn ci_list(failing: &[pr::FailingCheck]) -> Option<String> {
    (!failing.is_empty()).then(|| {
        failing
            .iter()
            .map(|c| match c.url.is_empty() {
                true => c.name.clone(),
                false => format!("[{}]({})", c.name, c.url),
            })
            .collect::<Vec<_>>()
            .join(", ")
    })
}

fn short(sha: &str) -> &str {
    &sha[..sha.len().min(10)]
}

/// One Jev request per hunk carrying every applicable rule; hints without
/// Jev. A new comment needs a margin above the rule's threshold. A failed
/// request is logged and the hunk gets no comment.
async fn judge<'r>(
    jev: Option<&Jev>,
    pr: &PullRequest,
    path: &str,
    hunk: &Hunk,
    rules: &[&'r Rule],
) -> (Vec<(&'r Rule, Verdict)>, Usage, usize) {
    let patch = hunk.render();
    match jev {
        None => (
            rules
                .iter()
                .filter_map(|r| decide_by_hint(r, hunk).map(|v| (*r, v)))
                .collect(),
            Usage::default(),
            0,
        ),
        Some(_) if patch.len() > MAX_PATCH_CHARS => {
            tracing::warn!(path, header = %hunk.header, chars = patch.len(), "hunk too large for Jev, skipped");
            (Vec::new(), Usage::default(), 0)
        }
        Some(j) => {
            let state = hunk_state(&pr.title, path, &patch);
            match j.ask(&state, rules).await {
                Ok((answers, usage)) => (
                    rules
                        .iter()
                        .filter_map(|r| {
                            answers
                                .get(&r.id.0)
                                .and_then(|ans| decide_at(r, hunk, ans, posting_threshold(r)))
                                .map(|v| (*r, v))
                        })
                        .collect(),
                    usage,
                    1,
                ),
                Err(e) => {
                    tracing::warn!(path, header = %hunk.header, error = %e, "Jev failed, hunk skipped");
                    (Vec::new(), Usage::default(), 1)
                }
            }
        }
    }
}

/// Drop what was already posted and duplicates, keep the highest scores,
/// at most MAX_PER_RULE per rule: one rule repeating itself down a pull
/// request is noise; the author gets the point from the first two.
fn select(mut findings: Vec<Finding>, posted: &HashSet<String>, max: usize) -> Vec<Finding> {
    findings.sort_by(|x, y| {
        y.verdict
            .score
            .total_cmp(&x.verdict.score)
            .then(y.severity.cmp(&x.severity))
    });
    let mut seen = HashSet::new();
    let mut per_rule: HashMap<RuleId, usize> = HashMap::new();
    findings
        .into_iter()
        .filter(|f| !posted.contains(&f.key()) && seen.insert(f.key()))
        .filter(|f| {
            let n = per_rule.entry(f.rule.clone()).or_default();
            *n += 1;
            *n <= MAX_PER_RULE
        })
        .take(max)
        .collect()
}

fn report(
    pr: &PullRequest,
    selected: &[Finding],
    posted: &HashSet<String>,
    asked: usize,
    usage: Usage,
) {
    println!(
        "PR #{} {}  (head {})",
        pr.number,
        pr.title,
        short(&pr.head.0)
    );
    selected.iter().for_each(|f| {
        let option = f
            .verdict
            .option
            .as_ref()
            .map(|o| format!("/{o}"))
            .unwrap_or_default();
        println!(
            "\n{}:{}  [{}{}] score {:.2}\n  > {}\n  {}",
            f.path,
            f.line,
            f.rule,
            option,
            f.verdict.score,
            f.line_text.trim(),
            f.verdict.comment
        );
    });
    eprintln!(
        "\n{} comment(s); {} already posted earlier; {} Jev request(s), {} input tokens (~${:.4})",
        selected.len(),
        posted.len(),
        asked,
        usage.input_tokens,
        usage.input_tokens as f64 * USD_PER_INPUT_TOKEN
    );
}
