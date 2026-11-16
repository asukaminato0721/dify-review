//! `dify-review watch`: one pass over the pull requests the bot should look
//! at; run it on a schedule.
//!
//! - Open pull requests assigned to `--assignee` and updated since `--since`
//!   (all of them with `--all-assigned`): each head is reviewed once, so a
//!   new push gets a new review and an unchanged head is left alone, except
//!   that a merge conflict appearing later is still flagged. Drafts wait
//!   until they are ready.
//! - Pull requests where a member mentions `@dify-codereview`: answered once
//!   per mention, even if the head was already reviewed.

use std::collections::BTreeMap;

use anyhow::Result;
use dify_review_core::Repo;
use dify_review_core::since::parse_since;
use dify_review_github::pr::{self, Trigger};

use crate::review::{Ask, ReviewOpts, Reviewer};

/// The bot's handle, after Meta's `meta-codesync`.
pub const DEFAULT_HANDLE: &str = "@dify-codereview";

#[derive(clap::Args)]
pub struct WatchArgs {
    #[arg(long, default_value = "langgenius/dify")]
    repo: Repo,
    /// Consider activity since: 2h, 1d or a date. Overlap between passes is
    /// harmless: heads and mentions are only handled once.
    #[arg(long, default_value = "2h")]
    since: String,
    /// Review open pull requests assigned to this login (updated since --since).
    #[arg(long)]
    assignee: Option<String>,
    /// Look at every open assigned pull request, not only recently updated
    /// ones: for a cold start, or a periodic sweep for merge conflicts that
    /// appear when the base moves (that does not update the pull request).
    #[arg(long)]
    all_assigned: bool,
    /// Mention that asks for a review.
    #[arg(long, default_value = DEFAULT_HANDLE)]
    handle: String,
    #[command(flatten)]
    opts: ReviewOpts,
}

pub async fn run(a: WatchArgs) -> Result<()> {
    let since = parse_since(&a.since, jiff::Timestamp::now())?;
    let reviewer = Reviewer::new(a.repo.clone(), a.opts).await?;

    let triggers = pr::triggers_since(&reviewer.gh, &a.repo, since, &a.handle).await?;
    // The latest mention per pull request stands for all of them.
    let mentioned: BTreeMap<u64, Trigger> = triggers.into_iter().map(|t| (t.pr, t)).collect();
    let assigned: Vec<u64> = match &a.assignee {
        Some(login) => {
            let window = (!a.all_assigned).then_some(since);
            pr::assigned_open(&reviewer.gh, &a.repo, login, window).await?
        }
        None => Vec::new(),
    };
    eprintln!(
        "{} open assigned to {} ({}); {} mentioning {} since {since}",
        assigned.len(),
        a.assignee.as_deref().unwrap_or("-"),
        if a.all_assigned {
            "all"
        } else {
            "updated in the window"
        },
        mentioned.len(),
        a.handle
    );

    for t in mentioned.values() {
        let history = pr::history(&reviewer.gh, &a.repo, t.pr).await?;
        match history.triggers.contains(&t.comment_id) {
            true => eprintln!("PR #{}: mention {} already answered", t.pr, t.url),
            false => {
                eprintln!("PR #{}: asked by @{} in {}", t.pr, t.author, t.url);
                let ask = Ask {
                    comment_id: t.comment_id,
                    author: &t.author,
                };
                report_failure(t.pr, reviewer.review(t.pr, Some(ask), true).await);
            }
        }
    }
    // A mention just answered already covers that pull request's head.
    for number in assigned.iter().filter(|n| !mentioned.contains_key(n)) {
        report_failure(*number, reviewer.review(*number, None, false).await);
    }
    Ok(())
}

/// One failing pull request must not stop the others.
fn report_failure(number: u64, res: Result<()>) {
    if let Err(e) = res {
        tracing::warn!(pr = number, error = %e, "review failed");
    }
}
