//! `dify-review refresh`: bring the comments dify-review already posted in
//! line with the current rule wording. Only the text above the footer
//! changes; footer and marker stay, so dedupe and feedback keep matching.

use anyhow::Result;
use dify_review_core::Repo;
use std::collections::BTreeSet;

use dify_review_core::review::{refreshed_body, refreshed_summary};
use dify_review_core::rules::RuleSet;
use dify_review_core::since::parse_since;
use dify_review_github::{GitHub, pr};
use std::path::PathBuf;

#[derive(clap::Args)]
pub struct RefreshArgs {
    #[arg(long, default_value = "langgenius/dify")]
    repo: Repo,
    /// Comments updated since: 30d, 12w or a date.
    #[arg(long, default_value = "30d")]
    since: String,
    #[arg(long, default_value = "rules")]
    rules: PathBuf,
    /// Edit the comments. Without this flag only list what would change.
    #[arg(long)]
    apply: bool,
}

pub async fn run(a: RefreshArgs) -> Result<()> {
    let rules = RuleSet::load_dir(&a.rules)?;
    let since = parse_since(&a.since, jiff::Timestamp::now())?;
    let gh = GitHub::from_env().await?;
    let posted = pr::bot_comments_since(&gh, &a.repo, since).await?;
    let stale: Vec<_> = posted
        .iter()
        .filter_map(|c| refreshed_body(&c.body, rules.rules()).map(|b| (c, b)))
        .collect();
    eprintln!(
        "{} dify-review comment(s) since {since}, {} with outdated wording",
        posted.len(),
        stale.len()
    );
    for (c, body) in &stale {
        let rule = body
            .split("rule=")
            .nth(1)
            .and_then(|r| r.split([';', ' ']).next())
            .unwrap_or("?");
        println!("{} [{rule}]", c.url);
        if a.apply {
            pr::edit_comment(&gh, &a.repo, c.id, body).await?;
        }
    }
    // Review summaries on the same pull requests.
    let prs: BTreeSet<u64> = posted.iter().map(|c| c.pr).collect();
    let mut summaries = 0usize;
    for number in &prs {
        let reviews = pr::reviews(&gh, &a.repo, *number).await?;
        let stale: Vec<_> = reviews
            .iter()
            .filter_map(|r| {
                let body = r.body.as_deref()?;
                refreshed_summary(body, &a.repo, *number).map(|b| (r, b))
            })
            .collect();
        summaries += stale.len();
        for (r, body) in &stale {
            println!("{} [summary]", r.html_url);
            if a.apply {
                pr::edit_review(&gh, &a.repo, *number, r.id, body).await?;
            }
        }
    }
    eprintln!(
        "{summaries} review summary(ies) in an older format on {} pull request(s)",
        prs.len()
    );
    eprintln!(
        "{}",
        match a.apply {
            true => "edited",
            false => "dry run: nothing edited (add --apply)",
        }
    );
    Ok(())
}
