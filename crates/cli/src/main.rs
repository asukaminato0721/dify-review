use std::collections::{BTreeMap, HashSet};
use std::fs::{self, File, OpenOptions};
use std::io::{BufRead, BufReader, BufWriter, Write};
use std::path::{Path, PathBuf};

use anyhow::{Context, Result};
use clap::{Parser, Subcommand};
use dify_review_core::{Login, PrState, Repo, ReviewThread, ThreadId, since::parse_since};
use dify_review_github::{GitHub, collect};

#[derive(Parser)]
#[command(
    version,
    about = "Learn review rules from PR history and apply them with Jev"
)]
struct Cli {
    #[command(subcommand)]
    cmd: Cmd,
}

#[derive(Subcommand)]
enum Cmd {
    /// Download inline review threads as JSONL, one thread per line.
    Collect(CollectArgs),
}

#[derive(clap::Args)]
struct CollectArgs {
    /// Repository as OWNER/NAME.
    #[arg(long, default_value = "langgenius/dify")]
    repo: Repo,
    /// Only pull requests updated since: 180d, 12w, 36h or 2026-05-01.
    #[arg(long, default_value = "180d")]
    since: String,
    /// Pull request states to include (merged, closed, open).
    #[arg(long, value_delimiter = ',', default_value = "merged,closed")]
    states: Vec<PrState>,
    /// Stop after this many pull requests (for trying things out).
    #[arg(long)]
    max_prs: Option<usize>,
    #[arg(long, default_value = "data/threads.jsonl")]
    out: PathBuf,
    /// Keep the existing file and add only threads not already in it.
    #[arg(long)]
    append: bool,
}

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env().unwrap_or_else(|_| "info".into()),
        )
        .with_writer(std::io::stderr)
        .init();

    match Cli::parse().cmd {
        Cmd::Collect(args) => run_collect(args).await,
    }
}

async fn run_collect(a: CollectArgs) -> Result<()> {
    let since = parse_since(&a.since, chrono::Utc::now())?;
    let seen = if a.append && a.out.exists() {
        existing_ids(&a.out)?
    } else {
        HashSet::new()
    };
    if let Some(dir) = a.out.parent().filter(|d| !d.as_os_str().is_empty()) {
        fs::create_dir_all(dir)?;
    }
    let file = if a.append {
        OpenOptions::new().create(true).append(true).open(&a.out)?
    } else {
        File::create(&a.out)?
    };
    let mut w = BufWriter::new(file);

    let gh = GitHub::from_env().await?;
    tracing::info!(repo = %a.repo, since = %since.to_rfc3339(), out = %a.out.display(), "collecting");
    let opts = collect::CollectOptions {
        repo: a.repo,
        since,
        states: a.states,
        max_prs: a.max_prs,
    };

    let mut reviewers: BTreeMap<Login, usize> = BTreeMap::new();
    let mut written = 0usize;
    let stats = collect::collect(&gh, &opts, |t: ReviewThread| {
        if !seen.contains(&t.id) {
            if let Some(r) = &t.flags.reviewer {
                *reviewers.entry(r.clone()).or_default() += 1;
            }
            serde_json::to_writer(&mut w, &t)?;
            w.write_all(b"\n")?;
            written += 1;
        }
        Ok(())
    })
    .await?;
    w.flush()?;

    eprintln!(
        "\n{} PRs scanned, {} with threads; {} threads ({} human review), {} newly written to {}; GraphQL cost {}",
        stats.prs_seen,
        stats.prs_with_threads,
        stats.threads,
        stats.human_threads,
        written,
        a.out.display(),
        stats.api_cost
    );
    let mut top: Vec<_> = reviewers.into_iter().collect();
    top.sort_by(|x, y| y.1.cmp(&x.1).then_with(|| x.0.cmp(&y.0)));
    if !top.is_empty() {
        eprintln!("human reviewers in newly written threads:");
        top.iter()
            .take(15)
            .for_each(|(login, n)| eprintln!("  {n:>6}  {login}"));
    }
    Ok(())
}

fn existing_ids(path: &Path) -> Result<HashSet<ThreadId>> {
    #[derive(serde::Deserialize)]
    struct Id {
        id: ThreadId,
    }
    let lines = BufReader::new(File::open(path)?)
        .lines()
        .collect::<std::io::Result<Vec<_>>>()?;
    lines
        .iter()
        .enumerate()
        .filter(|(_, line)| !line.trim().is_empty())
        .map(|(i, line)| {
            serde_json::from_str::<Id>(line)
                .map(|Id { id }| id)
                .with_context(|| format!("{}:{}: not a thread record", path.display(), i + 1))
        })
        .collect()
}
