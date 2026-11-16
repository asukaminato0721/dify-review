//! `dify-review feedback`: what became of the comments dify-review posted.
//! Reads every review thread a dify-review comment started on recently
//! updated pull requests (open ones too), asks Jev whether the replies
//! reject the suggestion, and writes one verdict per comment to
//! `backtest/feedback.toml`, which `backtest` then uses.

use std::collections::BTreeMap;
use std::fs::{self, File};
use std::io::{BufWriter, Write};
use std::path::{Path, PathBuf};

use anyhow::Result;
use dify_review_core::feedback::{DISAGREE_QUESTION, Outcome, bot_comment, human_replies, judge};
use dify_review_core::rules::{OptionName, RuleId};
use dify_review_core::since::parse_since;
use dify_review_core::{PrState, Repo, ReviewThread};
use dify_review_github::{GitHub, collect};
use dify_review_jev::{Jev, Usage};
use serde::{Deserialize, Serialize};
use serde_json::json;

#[derive(clap::Args)]
pub struct FeedbackArgs {
    #[arg(long, default_value = "langgenius/dify")]
    repo: Repo,
    /// Pull requests updated since: 30d, 12w or a date.
    #[arg(long, default_value = "30d")]
    since: String,
    #[arg(long, default_value = "backtest/feedback.toml")]
    out: PathBuf,
    /// The threads behind the verdicts, for backtest to find their hunks.
    #[arg(long, default_value = "data/feedback-threads.jsonl")]
    threads_out: PathBuf,
}

/// One verdict on one posted comment.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Entry {
    /// URL of the dify-review comment.
    pub comment: String,
    pub rule: RuleId,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub option: Option<OptionName>,
    pub outcome: Outcome,
    pub reason: String,
}

#[derive(Serialize, Deserialize, Default)]
pub struct FeedbackFile {
    #[serde(default)]
    pub feedback: Vec<Entry>,
}

pub fn load(path: &Path) -> Result<FeedbackFile> {
    match path.exists() {
        false => Ok(FeedbackFile::default()),
        true => Ok(toml::from_str(&fs::read_to_string(path)?)?),
    }
}

pub async fn run(a: FeedbackArgs) -> Result<()> {
    let since = parse_since(&a.since, jiff::Timestamp::now())?;
    let gh = GitHub::from_env().await?;
    let jev = Jev::from_env()?;
    if jev.is_none() {
        tracing::warn!("no Jev key: replies are not read, so only 👎 can mark a comment wrong");
    }
    let opts = collect::CollectOptions {
        repo: a.repo.clone(),
        since,
        states: vec![PrState::Open, PrState::Merged, PrState::Closed],
        max_prs: None,
    };
    let mut threads: Vec<ReviewThread> = Vec::new();
    collect::collect(&gh, &opts, |t| {
        if bot_comment(&t).is_some() {
            threads.push(t);
        }
        Ok(())
    })
    .await?;
    tracing::info!(threads = threads.len(), "dify-review comments found");

    let mut usage = Usage::default();
    let mut entries = Vec::new();
    for t in &threads {
        let replies = human_replies(t);
        let disagree = match (&jev, replies.is_empty()) {
            (Some(j), false) => {
                let suggestion = t.comments[0]
                    .body
                    .split("\n\n<sub>")
                    .next()
                    .unwrap_or_default();
                let state = json!({
                    "suggestion": suggestion,
                    "replies": replies.iter().map(|c| c.body.as_str()).collect::<Vec<_>>(),
                });
                match j.noul(&state, DISAGREE_QUESTION).await {
                    Ok((p, u)) => {
                        usage.input_tokens += u.input_tokens;
                        Some(p)
                    }
                    Err(e) => {
                        tracing::warn!(url = %t.comments[0].url, error = %e, "Jev failed on the replies");
                        None
                    }
                }
            }
            _ => None,
        };
        let verdict = judge(t, disagree);
        let bot = bot_comment(t).expect("filtered to bot comments above");
        entries.push(Entry {
            comment: t.comments[0].url.clone(),
            rule: bot.rule,
            option: bot.option,
            outcome: verdict.outcome,
            reason: verdict.reason,
        });
    }
    entries.sort_by(|x, y| x.comment.cmp(&y.comment));

    write_threads(&a.threads_out, &threads)?;
    if let Some(dir) = a.out.parent().filter(|d| !d.as_os_str().is_empty()) {
        fs::create_dir_all(dir)?;
    }
    let header = "# What became of each comment dify-review posted. Written by\n# `dify-review feedback`; `backtest` reads `right` as a positive and\n# `wrong` as an explicit negative. Do not edit: rerun it.\n\n";
    fs::write(
        &a.out,
        format!(
            "{header}{}",
            toml::to_string(&FeedbackFile {
                feedback: entries.clone()
            })?
        ),
    )?;

    summarize(&entries);
    eprintln!(
        "\n{} comment(s) -> {}; {} input tokens on replies (~${:.4})",
        entries.len(),
        a.out.display(),
        usage.input_tokens,
        usage.input_tokens as f64 * 0.042 / 1_000_000.0
    );
    Ok(())
}

fn write_threads(path: &Path, threads: &[ReviewThread]) -> Result<()> {
    if let Some(dir) = path.parent().filter(|d| !d.as_os_str().is_empty()) {
        fs::create_dir_all(dir)?;
    }
    let mut w = BufWriter::new(File::create(path)?);
    threads.iter().try_for_each(|t| -> Result<()> {
        serde_json::to_writer(&mut w, t)?;
        w.write_all(b"\n")?;
        Ok(())
    })?;
    w.flush()?;
    Ok(())
}

fn summarize(entries: &[Entry]) {
    let mut per_rule: BTreeMap<&RuleId, [usize; 4]> = BTreeMap::new();
    entries.iter().for_each(|e| {
        let slot = match e.outcome {
            Outcome::Right => 0,
            Outcome::Wrong => 1,
            Outcome::Ignored => 2,
            Outcome::Pending => 3,
        };
        per_rule.entry(&e.rule).or_default()[slot] += 1;
    });
    println!(
        "{:<26} {:>6} {:>6} {:>8} {:>8}",
        "rule", "right", "wrong", "ignored", "pending"
    );
    per_rule.iter().for_each(|(rule, [r, w, i, p])| {
        println!("{rule:<26} {r:>6} {w:>6} {i:>8} {p:>8}");
    });
}
