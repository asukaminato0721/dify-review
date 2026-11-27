//! `dify-review review`: judge one pull request's hunks against the rules
//! and print, or post, the inline comments. `Reviewer` is shared with
//! `watch`, which reviews the pull requests someone mentioned the bot on.

use std::collections::{HashMap, HashSet};
use std::path::PathBuf;

use anyhow::Result;
use dify_review_core::Repo;
use dify_review_core::diff::Chunk;
use dify_review_core::review::{
    Finding, SplitAsk, SplitFacts, SummaryFacts, Verdict, decide_at, decide_by_hint,
    flags_problem, is_split_notice, posting_threshold, split_body, summary_body,
};
use dify_review_core::size::{REQUIRE_SPLIT, ReviewLines, SUGGEST_SPLIT, SizeTier};
use dify_review_core::rules::{Rule, RuleId, RuleSet};
use dify_review_github::GitHub;
use dify_review_github::pr::{self, InlineComment, PullRequest, ReviewEvent};
use dify_review_jev::{Jev, MAX_PATCH_CHARS, Usage, chunk_state};
use futures::stream::{self, StreamExt};

/// Chunks of one pull request judged at once.
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
    /// Failing CI jobs whose name contains this (case insensitive) are not
    /// flagged: flaky suites. Repeatable.
    #[arg(long = "ignore-ci", default_value = "E2E")]
    ignore_ci: Vec<String>,
    /// Over this many added lines of non-test code (pyrefly's rule of
    /// thumb), the author is asked once to consider splitting; still reviewed.
    #[arg(long, default_value_t = SUGGEST_SPLIT.0)]
    suggest_split: u64,
    /// Over this many (pyrefly's `size/xl`), the author is asked once to
    /// split, and the pull request is not judged until it is under.
    #[arg(long, default_value_t = REQUIRE_SPLIT.0)]
    require_split: u64,
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
        Ok(Self {
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
        // Judged before: a posted review names this head, or a status label
        // was set after it arrived.
        let seen = history.heads.contains(&pr.head.0)
            || (pr.open && pr::status_stamp(&self.gh, &self.repo, &pr).await?.judged());
        let asked = force || ask.is_some();
        match (pr.open, pr.draft && !asked) {
            (false, _) => {
                eprintln!("PR #{number}: not open, skipped");
                Ok(())
            }
            (true, true) => {
                eprintln!("PR #{number}: draft, skipped until ready for review");
                Ok(())
            }
            (true, false) => {
                let failing =
                    pr::failing_checks(&self.gh, &self.repo, &pr.head, &self.opts.ignore_ci)
                        .await?;
                let lines = pr::review_lines(&self.gh, &self.repo, number).await?;
                let tier = SizeTier::of(
                    lines,
                    ReviewLines(self.opts.suggest_split),
                    ReviewLines(self.opts.require_split),
                );
                match (tier, asked, seen) {
                    (SizeTier::RequireSplit, false, _) => {
                        self.require_split(&pr, &history, seen, &failing, lines)
                            .await?
                    }
                    (SizeTier::SuggestSplit, _, _) => {
                        self.suggest_split(&pr, &history, lines).await?;
                        self.judge_or_recheck(&pr, ask, &history, seen && !asked, &failing)
                            .await?
                    }
                    _ => {
                        self.judge_or_recheck(&pr, ask, &history, seen && !asked, &failing)
                            .await?
                    }
                }
                let clean = !pr.conflicted() && failing.is_empty();
                self.fold_notices(&pr, clean, tier == SizeTier::Fine).await
            }
        }
    }

    async fn judge_or_recheck(
        &self,
        pr: &PullRequest,
        ask: Option<Ask<'_>>,
        history: &pr::History,
        judged: bool,
        failing: &[pr::FailingCheck],
    ) -> Result<()> {
        match judged {
            true => self.recheck(pr, history, failing).await,
            false => self.judge_and_post(pr, ask, failing).await,
        }
    }

    /// Too large to judge well (and the costliest to ask Jev about): ask the
    /// author once to split it. Later heads only get the cheap checks and
    /// the status label, until it is small enough or someone asks.
    async fn require_split(
        &self,
        pr: &PullRequest,
        history: &pr::History,
        seen: bool,
        failing: &[pr::FailingCheck],
        lines: ReviewLines,
    ) -> Result<()> {
        match (history.splits.is_empty(), seen) {
            (true, _) => {
                eprintln!(
                    "PR #{}: {lines} added non-test lines, over {}: asking to split",
                    pr.number, self.opts.require_split
                );
                let body = split_body(&SplitFacts {
                    head: &pr.head.0,
                    lines,
                    limit: ReviewLines(self.opts.require_split),
                    tier: SplitAsk::Require,
                });
                self.post(pr, &body, &[], ReviewEvent::RequestChanges)
                    .await?;
                self.label(pr, pr::WAITING_FOR_AUTHOR).await
            }
            (false, true) => self.recheck(pr, history, failing).await,
            (false, false) => {
                eprintln!(
                    "PR #{}: still {lines} added non-test lines, split already asked; label only",
                    pr.number
                );
                self.label(pr, pr::WAITING_FOR_AUTHOR).await
            }
        }
    }

    /// Over the rule of thumb: suggest a split once (unless it was already
    /// asked for), as a comment that requests nothing. Review goes on.
    async fn suggest_split(
        &self,
        pr: &PullRequest,
        history: &pr::History,
        lines: ReviewLines,
    ) -> Result<()> {
        match history.splits.is_empty() && history.split_suggestions.is_empty() {
            false => Ok(()),
            true => {
                eprintln!(
                    "PR #{}: {lines} added non-test lines, over {}: suggesting a split",
                    pr.number, self.opts.suggest_split
                );
                let body = split_body(&SplitFacts {
                    head: &pr.head.0,
                    lines,
                    limit: ReviewLines(self.opts.suggest_split),
                    tier: SplitAsk::Suggest,
                });
                self.post(pr, &body, &[], ReviewEvent::Comment).await
            }
        }
    }

    /// Keep one bot notice in view: fold every earlier summary without
    /// inline suggestions, and the newest too once what it flags is gone.
    /// A split notice stays while the pull request is still too large.
    /// Folded, not deleted: the markers in them still count.
    async fn fold_notices(&self, pr: &PullRequest, clean: bool, size_ok: bool) -> Result<()> {
        let reviews = pr::bot_reviews(&self.gh, &self.repo, pr.number).await?;
        let stale = to_fold(&reviews, clean, size_ok);
        match (stale.is_empty(), self.opts.post) {
            (true, _) => Ok(()),
            (false, false) => {
                eprintln!("dry run: would fold {} earlier notice(s)", stale.len());
                Ok(())
            }
            (false, true) => {
                for r in &stale {
                    pr::minimize(&self.gh, &r.node_id).await?;
                }
                eprintln!("PR #{}: folded {} earlier notice(s)", pr.number, stale.len());
                // A split request folded because the size is fine no longer
                // stands in the way either.
                let lifted: Vec<u64> = stale
                    .iter()
                    .filter(|r| r.requests_changes && is_split_notice(&r.body))
                    .map(|r| r.id)
                    .collect();
                match lifted.is_empty() {
                    true => Ok(()),
                    false => {
                        let msg = "Small enough to review: the split request no longer applies.";
                        pr::dismiss_ids(&self.gh, &self.repo, pr.number, &lifted, msg).await?;
                        eprintln!("PR #{}: dismissed {} split request(s)", pr.number, lifted.len());
                        Ok(())
                    }
                }
            }
        }
    }

    /// A head already reviewed: only the cheap checks, no Jev. Since the
    /// review, the base may have moved on (a merge conflict) or CI may have
    /// finished and failed; each is flagged once per head.
    async fn recheck(
        &self,
        pr: &PullRequest,
        history: &pr::History,
        failing: &[pr::FailingCheck],
    ) -> Result<()> {
        let head = &pr.head.0;
        let new_conflict = pr.conflicted() && !history.conflicts.contains(head);
        let new_ci = !failing.is_empty() && !history.ci.contains(head);
        match new_conflict || new_ci {
            // Something the author must act on appeared since the review.
            true => {
                eprintln!(
                    "PR #{}: head {}{}{}",
                    pr.number,
                    short(head),
                    if new_conflict { " now conflicts" } else { "" },
                    if new_ci { ", CI failing" } else { "" }
                );
                let ci = ci_list(failing).filter(|_| new_ci);
                let facts = SummaryFacts {
                    head,
                    new: 0,
                    still_open: 0,
                    conflict_base: new_conflict.then_some(pr.base_ref.as_str()),
                    ci_failing: ci.as_deref(),
                    requester: None,
                    trigger: None,
                };
                self.post(pr, &self.summary(pr, &facts), &[], ReviewEvent::RequestChanges)
                    .await?;
                self.label(pr, pr::WAITING_FOR_AUTHOR).await
            }
            // Nothing new: keep the status label in step with the pull request.
            false => self.restatus(pr, failing).await,
        }
    }

    /// Whose turn it is on a head already judged: the author's while a
    /// thread is unresolved, it conflicts, or CI fails; else the reviewer's.
    /// When it turns to the reviewer and the bot still requests changes,
    /// say nothing is left and lift that request.
    async fn restatus(&self, pr: &PullRequest, failing: &[pr::FailingCheck]) -> Result<()> {
        let threads = pr::review_threads(&self.gh, &self.repo, pr.number).await?;
        let unresolved = threads
            .iter()
            .filter(|t| !t.resolved && !(t.bot && t.outdated))
            .count();
        let author_turn = unresolved > 0 || pr.conflicted() || !failing.is_empty();
        let status = match author_turn {
            true => pr::WAITING_FOR_AUTHOR,
            false => pr::WAITING_FOR_REVIEWER,
        };
        match pr.labels.iter().any(|l| l == status) {
            true => {
                eprintln!(
                    "PR #{}: head {} already reviewed, still {status}",
                    pr.number,
                    short(&pr.head.0)
                );
                Ok(())
            }
            false => {
                let standing = match author_turn {
                    true => Vec::new(),
                    false => pr::bot_change_requests(&self.gh, &self.repo, pr.number).await?,
                };
                if !standing.is_empty() {
                    let facts = SummaryFacts {
                        head: &pr.head.0,
                        new: 0,
                        still_open: 0,
                        conflict_base: None,
                        ci_failing: None,
                        requester: None,
                        trigger: None,
                    };
                    self.post(pr, &self.summary(pr, &facts), &[], ReviewEvent::Comment).await?;
                    if self.opts.post {
                        let msg = format!("Nothing left to flag at {}.", short(&pr.head.0));
                        pr::dismiss_reviews(&self.gh, &self.repo, pr.number, &standing, &msg)
                            .await?;
                        eprintln!("dismissed {} earlier change request(s)", standing.len());
                    }
                }
                self.label(pr, status).await
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
        // Hints alone have no size limit; Jev has.
        let limit = self.jev.as_ref().map_or(usize::MAX, |_| MAX_PATCH_CHARS);
        let jobs: Vec<(&str, Chunk, Vec<&Rule>)> = files
            .iter()
            .flat_map(|file| {
                let applicable: Vec<&Rule> = self
                    .rules
                    .rules()
                    .iter()
                    .filter(|r| r.applies_to(&file.path))
                    .collect();
                let packed = match applicable.is_empty() {
                    true => Default::default(),
                    false => Chunk::pack(file.hunks.iter().cloned(), limit),
                };
                packed.oversized.iter().for_each(|h| {
                    tracing::warn!(path = %file.path, header = %h.header, "hunk too large for Jev, skipped")
                });
                packed
                    .chunks
                    .into_iter()
                    .map(move |c| (file.path.as_str(), c, applicable.clone()))
            })
            .collect();
        let judged: Vec<_> = stream::iter(jobs)
            .map(|(path, chunk, rules)| async move {
                let (verdicts, usage, asked) =
                    judge(self.jev.as_ref(), pr, path, &chunk, &rules).await;
                let found: Vec<Finding> = verdicts
                    .into_iter()
                    .filter_map(|(rule, v)| Finding::new(rule, path, &chunk, v))
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
                self.post(pr, &self.summary(pr, &facts), &comments, ReviewEvent::RequestChanges)
                    .await?
            }
            // Everything was dealt with: say so, and lift the earlier change request.
            (false, _, true) => {
                self.post(pr, &self.summary(pr, &facts), &[], ReviewEvent::Comment).await?;
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
            }
            (false, true, false) => self.post(pr, &self.summary(pr, &facts), &[], ReviewEvent::Comment).await?,
            (false, false, false) => eprintln!(
                "PR #{}: nothing new to act on at {}{}; label only",
                pr.number,
                short(&pr.head.0),
                match still_open {
                    0 => String::new(),
                    n => format!(" ({n} unresolved thread(s) remain)"),
                }
            ),
        }
        // Whose turn it is; the label's time also records that this head was judged.
        let status = match act_on || !clear {
            true => pr::WAITING_FOR_AUTHOR,
            false => pr::WAITING_FOR_REVIEWER,
        };
        self.label(pr, status).await
    }

    async fn label(&self, pr: &PullRequest, status: &str) -> Result<()> {
        match self.opts.post {
            false => {
                eprintln!("dry run: would label PR #{} {status}", pr.number);
                Ok(())
            }
            true => {
                pr::set_status(&self.gh, &self.repo, pr, status).await?;
                eprintln!("labelled PR #{} {status}", pr.number);
                Ok(())
            }
        }
    }

    fn summary(&self, pr: &PullRequest, facts: &SummaryFacts<'_>) -> String {
        summary_body(&self.repo, pr.number, facts)
    }

    async fn post(
        &self,
        pr: &PullRequest,
        body: &str,
        comments: &[InlineComment],
        event: ReviewEvent,
    ) -> Result<()> {
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
                    pr::post_review(&self.gh, &self.repo, pr, body, comments, event).await?;
                eprintln!("\nposted ({event:?}): {}", review.html_url);
                Ok(())
            }
        }
    }
}

/// Bot notices to fold. A split notice: once the size is fine (`size_ok`).
/// Any other summary without inline suggestions: when it is not the newest
/// bot review, or it is and flags a conflict or failing CI that is gone
/// (`clean`). Suggestions are never folded.
fn to_fold(reviews: &[pr::BotReview], clean: bool, size_ok: bool) -> Vec<&pr::BotReview> {
    let newest = reviews.len().saturating_sub(1);
    reviews
        .iter()
        .enumerate()
        .filter(|(_, r)| !r.minimized && !r.has_inline)
        .filter(|(i, r)| match is_split_notice(&r.body) {
            true => size_ok,
            false => *i != newest || (clean && flags_problem(&r.body)),
        })
        .map(|(_, r)| r)
        .collect()
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

/// One Jev request per chunk carrying every applicable rule; hints without
/// Jev. A new comment needs a margin above the rule's threshold. A failed
/// request is logged and the chunk gets no comment.
async fn judge<'r>(
    jev: Option<&Jev>,
    pr: &PullRequest,
    path: &str,
    chunk: &Chunk,
    rules: &[&'r Rule],
) -> (Vec<(&'r Rule, Verdict)>, Usage, usize) {
    match jev {
        None => (
            rules
                .iter()
                .filter_map(|r| decide_by_hint(r, chunk).map(|v| (*r, v)))
                .collect(),
            Usage::default(),
            0,
        ),
        Some(j) => {
            let state = chunk_state(&pr.title, path, chunk);
            match j.ask(&state, rules).await {
                Ok((answers, usage)) => (
                    rules
                        .iter()
                        .filter_map(|r| {
                            answers
                                .get(&r.id.0)
                                .and_then(|ans| decide_at(r, chunk, ans, posting_threshold(r)))
                                .map(|v| (*r, v))
                        })
                        .collect(),
                    usage,
                    1,
                ),
                Err(e) => {
                    tracing::warn!(path, header = %chunk.first().header, error = %e, "Jev failed, chunk skipped");
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

#[cfg(test)]
mod tests {
    use super::*;

    fn review(id: &str, body: &str, has_inline: bool, minimized: bool) -> pr::BotReview {
        pr::BotReview {
            node_id: id.to_owned(),
            id: 0,
            requests_changes: false,
            body: body.to_owned(),
            has_inline,
            minimized,
        }
    }

    #[test]
    fn folds_earlier_notices_and_a_stale_newest_one() {
        let conflict = "x <!-- dify-review:head=a;conflict=a -->";
        let plain = "x <!-- dify-review:head=b -->";
        let reviews = [
            review("found", plain, true, false),
            review("old-conflict", conflict, false, false),
            review("folded", conflict, false, true),
            review("newest", conflict, false, false),
        ];
        let ids = |clean| -> Vec<&str> {
            to_fold(&reviews, clean, true)
                .iter()
                .map(|r| r.node_id.as_str())
                .collect()
        };
        // Suggestions are never folded; the newest notice stays while it holds.
        assert_eq!(ids(false), ["old-conflict"]);
        assert_eq!(ids(true), ["old-conflict", "newest"]);
        // A newest summary that flags nothing stays even when clean.
        let quiet = [review("q", plain, false, false)];
        assert!(to_fold(&quiet, true, true).is_empty());
    }

    #[test]
    fn split_notice_stays_while_the_pull_request_is_too_large() {
        let split = "x <!-- dify-review:head=a;suggest_split=a -->";
        let found = "x <!-- dify-review:head=a -->";
        // The split notice is older than the review that followed it.
        let reviews = [review("split", split, false, false), review("found", found, true, false)];
        assert!(to_fold(&reviews, true, false).is_empty());
        let ids: Vec<&str> = to_fold(&reviews, true, true)
            .iter()
            .map(|r| r.node_id.as_str())
            .collect();
        assert_eq!(ids, ["split"]);
    }
}
