//! `dify-review review`: judge one pull request's hunks against the rules
//! and print, or post, the inline comments. `Reviewer` is shared with
//! `watch`, which reviews the pull requests someone mentioned the bot on.

use std::collections::{HashMap, HashSet};
use std::path::PathBuf;

use anyhow::Result;
use dify_review_core::Repo;
use dify_review_core::diff::Hunk;
use dify_review_core::review::{
    Finding, Verdict, decide_at, decide_by_hint, posting_threshold, summary_marker,
};
use dify_review_core::rules::{Rule, RuleId, RuleSet};
use dify_review_github::GitHub;
use dify_review_github::pr::{self, InlineComment, PullRequest};
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
        let seen = history.heads.contains(&pr.head.0);
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
            (true, false) => {
                eprintln!(
                    "PR #{number}: head {} already reviewed (use --force to review again)",
                    short(&pr.head.0)
                );
                Ok(())
            }
            _ => self.judge_and_post(&pr, ask).await,
        }
    }

    async fn judge_and_post(&self, pr: &PullRequest, ask: Option<Ask<'_>>) -> Result<()> {
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

        let posted = pr::posted_keys(&self.gh, &self.repo, pr.number).await?;
        let selected = select(findings, &posted, self.opts.max_comments);
        report(pr, &selected, &posted, asked, usage);

        // An explicit ask always gets an answer, even "nothing to flag".
        match (self.opts.post, selected.is_empty() && ask.is_none()) {
            (false, _) => {
                eprintln!("\ndry run: nothing posted (add --post to post)");
                Ok(())
            }
            (true, true) => {
                eprintln!("\nnothing new to post");
                Ok(())
            }
            (true, false) => {
                let comments: Vec<_> = selected
                    .iter()
                    .map(|f| InlineComment {
                        path: f.path.clone(),
                        line: f.line,
                        body: f.body(),
                    })
                    .collect();
                let summary = summary(pr, comments.len(), ask.as_ref());
                let review = pr::post_review(&self.gh, &self.repo, pr, &summary, &comments).await?;
                eprintln!("\nposted: {}", review.html_url);
                Ok(())
            }
        }
    }
}

fn short(sha: &str) -> &str {
    &sha[..sha.len().min(10)]
}

fn summary(pr: &PullRequest, n: usize, ask: Option<&Ask<'_>>) -> String {
    let who = ask
        .map(|a| format!("Requested by @{}. ", a.author))
        .unwrap_or_default();
    let what = match n {
        0 => format!("Nothing to flag at `{}`.", short(&pr.head.0)),
        n => format!(
            "{n} inline suggestion(s) at `{}`, each from a rule learned from earlier reviews in this repository. Reply on a comment if it is wrong.",
            short(&pr.head.0)
        ),
    };
    format!(
        "{who}{what}\n\n<sub>dify-codereview</sub>\n{}",
        summary_marker(&pr.head.0, ask.map(|a| a.comment_id))
    )
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
