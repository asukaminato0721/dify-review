//! `dify-review review`: judge one pull request's hunks against the rules
//! and print, or post, the inline comments.

use std::collections::HashSet;
use std::path::PathBuf;

use anyhow::Result;
use dify_review_core::Repo;
use dify_review_core::diff::Hunk;
use dify_review_core::review::{Finding, Verdict, decide, decide_by_hint};
use dify_review_core::rules::{Rule, RuleSet};
use dify_review_github::GitHub;
use dify_review_github::pr::{self, InlineComment, PullRequest};
use dify_review_jev::{Jev, Usage};
use serde_json::json;

/// Jev's input limit is about 32k tokens for state plus one question; a
/// hunk this large is skipped rather than truncated into a wrong answer.
const MAX_HUNK_CHARS: usize = 60_000;
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
    let rules = RuleSet::load_dir(&a.rules)?;
    let jev = match a.hints_only {
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
    let pr = pr::pull_request(&gh, &a.repo, a.pr).await?;
    let files = pr::changed_files(&gh, &a.repo, a.pr).await?;
    tracing::info!(pr = pr.number, title = %pr.title, files = files.len(), "reviewing");

    let mut findings = Vec::new();
    let mut usage = Usage::default();
    let mut asked = 0usize;
    for file in &files {
        let applicable: Vec<&Rule> = rules
            .rules()
            .iter()
            .filter(|r| r.applies_to(&file.path))
            .collect();
        let hunks = file
            .hunks
            .iter()
            .filter(|h| h.has_additions() && !applicable.is_empty());
        for hunk in hunks {
            let verdicts = judge(
                jev.as_ref(),
                &pr,
                &file.path,
                hunk,
                &applicable,
                &mut usage,
                &mut asked,
            )
            .await;
            findings.extend(
                verdicts
                    .into_iter()
                    .filter_map(|(rule, v)| Finding::new(rule, &file.path, hunk, v)),
            );
        }
    }

    let posted = pr::posted_keys(&gh, &a.repo, a.pr).await?;
    let selected = select(findings, &posted, a.max_comments);
    report(&pr, &selected, &posted, asked, usage);

    match (a.post, selected.is_empty()) {
        (false, _) => {
            eprintln!("\ndry run: nothing posted (add --post to post these as one review)")
        }
        (true, true) => eprintln!("\nnothing new to post"),
        (true, false) => {
            let comments: Vec<_> = selected
                .iter()
                .map(|f| InlineComment {
                    path: f.path.clone(),
                    line: f.line,
                    body: f.body(),
                })
                .collect();
            let summary = format!(
                "dify-review left {} inline suggestion(s), each from a rule learned from earlier reviews in this repository. Reply on a comment if it is wrong.",
                comments.len()
            );
            let review = pr::post_review(&gh, &a.repo, &pr, &summary, &comments).await?;
            eprintln!("\nposted: {}", review.html_url);
        }
    }
    Ok(())
}

/// One Jev request per hunk carrying every applicable rule; hints without
/// Jev. A failed request is logged and the hunk gets no comment.
async fn judge<'r>(
    jev: Option<&Jev>,
    pr: &PullRequest,
    path: &str,
    hunk: &Hunk,
    rules: &[&'r Rule],
    usage: &mut Usage,
    asked: &mut usize,
) -> Vec<(&'r Rule, Verdict)> {
    let patch = hunk.render();
    match jev {
        None => rules
            .iter()
            .filter_map(|r| decide_by_hint(r, hunk).map(|v| (*r, v)))
            .collect(),
        Some(_) if patch.len() > MAX_HUNK_CHARS => {
            tracing::warn!(path, header = %hunk.header, chars = patch.len(), "hunk too large for Jev, skipped");
            Vec::new()
        }
        Some(j) => {
            let state = json!({ "title": pr.title, "path": path, "patch": patch });
            *asked += 1;
            match j.ask(&state, rules).await {
                Ok((answers, u)) => {
                    usage.input_tokens += u.input_tokens;
                    usage.output_tokens += u.output_tokens;
                    rules
                        .iter()
                        .filter_map(|r| {
                            answers
                                .get(&r.id.0)
                                .and_then(|ans| decide(r, ans))
                                .map(|v| (*r, v))
                        })
                        .collect()
                }
                Err(e) => {
                    tracing::warn!(path, header = %hunk.header, error = %e, "Jev failed, hunk skipped");
                    Vec::new()
                }
            }
        }
    }
}

/// Drop what was already posted and duplicates, keep the highest scores.
fn select(mut findings: Vec<Finding>, posted: &HashSet<String>, max: usize) -> Vec<Finding> {
    findings.sort_by(|x, y| {
        y.verdict
            .score
            .total_cmp(&x.verdict.score)
            .then(y.severity.cmp(&x.severity))
    });
    let mut seen = HashSet::new();
    findings
        .into_iter()
        .filter(|f| !posted.contains(&f.key()) && seen.insert(f.key()))
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
        &pr.head.0[..pr.head.0.len().min(10)]
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
