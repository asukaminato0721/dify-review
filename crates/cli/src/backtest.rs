//! `dify-review backtest`: ask Jev about every chunk (a file's hunks, as
//! review sends them) the reviewer commented on (positives, from labels) and
//! every chunk they let through (the final diff of the merged pull
//! requests), then report per rule how often it
//! would have matched the reviewer, and suggest a threshold.

use std::collections::{BTreeMap, BTreeSet, HashMap};
use std::fs::{self, File, OpenOptions};
use std::io::{BufRead, BufReader, BufWriter, Write};
use std::path::{Path, PathBuf};

use anyhow::{Context, Result, ensure};
use dify_review_core::backtest::{Label, Row, RuleReport, Sample, evaluate};
use dify_review_core::diff::{Chunk, Hunk, NewLine, parse_patch};
use dify_review_core::review::Answer;
use dify_review_core::rules::{Judge, OptionName, Rule, RuleId, RuleSet};
use dify_review_core::{PrState, Repo, ReviewThread};
use dify_review_github::{GitHub, pr};
use dify_review_jev::{Jev, MAX_PATCH_CHARS, chunk_state};
use futures::stream::{self, StreamExt};
use serde::{Deserialize, Serialize};

const USD_PER_INPUT_TOKEN: f64 = 0.042 / 1_000_000.0;

#[derive(clap::Args)]
pub struct BacktestArgs {
    /// Collected review threads (`dify-review collect`).
    #[arg(long, default_value = "data/threads.jsonl")]
    threads: PathBuf,
    /// Which review comment is an instance of which rule.
    #[arg(long, default_value = "backtest/labels.toml")]
    labels: PathBuf,
    #[arg(long, default_value = "rules")]
    rules: PathBuf,
    /// The file diff each labelled comment was left on, fetched once.
    #[arg(long, default_value = "data/backtest-files.jsonl")]
    files: PathBuf,
    /// Verdicts on live comments (`dify-review feedback`).
    #[arg(long, default_value = "backtest/feedback.toml")]
    feedback: PathBuf,
    /// The threads behind those verdicts.
    #[arg(long, default_value = "data/feedback-threads.jsonl")]
    feedback_threads: PathBuf,
    /// Jev answers by request fingerprint; reruns only ask what changed.
    #[arg(long, default_value = "data/backtest-answers.jsonl")]
    cache: PathBuf,
    /// Jev requests in flight at once.
    #[arg(long, default_value_t = 8)]
    concurrency: usize,
    #[arg(long, default_value = "backtest/report.md")]
    report: PathBuf,
    /// Write each suggested threshold into its rule file.
    #[arg(long)]
    apply: bool,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct LabelsFile {
    label: Vec<RawLabel>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct RawLabel {
    /// URL of the review comment that started the thread.
    comment: String,
    rule: RuleId,
    #[serde(default)]
    option: Option<OptionName>,
}

/// A chunk to judge, before Jev has answered.
struct Pending {
    origin: String,
    title: String,
    path: String,
    chunk: Chunk,
    labels: Vec<Label>,
    rejected: Vec<RuleId>,
}

/// What is known about one review comment: the rules it is an instance of
/// (labels, live comments judged right) and the rules judged wrong there.
#[derive(Default, Clone)]
struct Marks {
    labels: Vec<Label>,
    rejected: Vec<RuleId>,
}

#[derive(Serialize, Deserialize)]
struct CacheLine {
    key: String,
    answers: BTreeMap<String, Answer>,
}

pub async fn run(a: BacktestArgs) -> Result<()> {
    let rules = RuleSet::load_dir(&a.rules)?;
    let jev = Jev::from_env()?.context("backtest needs Jev: set TYPESAFE_API_KEY")?;
    let labels = load_labels(&a.labels, &rules)?;
    let mut threads = load_threads(&a.threads)?;
    if a.feedback_threads.exists() {
        threads.extend(load_threads(&a.feedback_threads)?);
    }
    let marks = with_feedback(&labels, &crate::feedback::load(&a.feedback)?, &rules);

    let gh = GitHub::from_env().await?;
    let positives = positive_samples(&gh, &threads, &marks, &a.files).await?;
    let negatives = negative_samples(&gh, &threads, &labels).await?;
    tracing::info!(
        positives = positives.len(),
        negatives = negatives.len(),
        "samples"
    );

    let mut cache = load_cache(&a.cache)?;
    let positives = merge_same_chunk(positives);
    let pending: Vec<Pending> = positives.into_iter().chain(negatives).collect();
    let (samples, asked, tokens) =
        judge_all(&jev, &rules, pending, &mut cache, &a.cache, a.concurrency).await?;

    let reports: Vec<RuleReport> = rules
        .rules()
        .iter()
        .map(|r| evaluate(r, &samples))
        .collect();
    print_summary(&reports);
    write_report(&a.report, &reports, samples.len())?;
    eprintln!(
        "\n{} samples; {} new Jev request(s), {} input tokens (~${:.4}); full grid in {}",
        samples.len(),
        asked,
        tokens,
        tokens as f64 * USD_PER_INPUT_TOKEN,
        a.report.display()
    );
    match a.apply {
        true => apply_thresholds(&a.rules, &reports),
        false => {
            eprintln!("thresholds unchanged (add --apply to write the suggested ones into rules/)");
            Ok(())
        }
    }
}

fn load_labels(path: &Path, rules: &RuleSet) -> Result<BTreeMap<String, Vec<Label>>> {
    let file: LabelsFile = toml::from_str(&fs::read_to_string(path)?)
        .with_context(|| format!("parsing {}", path.display()))?;
    let by_id: HashMap<&RuleId, &Rule> = rules.rules().iter().map(|r| (&r.id, r)).collect();
    file.label.into_iter().try_fold(
        BTreeMap::new(),
        |mut acc: BTreeMap<String, Vec<Label>>, l| {
            let rule = by_id
                .get(&l.rule)
                .with_context(|| format!("label for {}: unknown rule {}", l.comment, l.rule))?;
            let option_ok = match (&rule.judge, &l.option) {
                (Judge::Noul { .. }, None) => true,
                (Judge::Choice { options, .. }, Some(o)) => options
                    .iter()
                    .any(|opt| &opt.name == o && opt.comment.is_some()),
                _ => false,
            };
            ensure!(
                option_ok,
                "label for {}: rule {} needs {}",
                l.comment,
                l.rule,
                match rule.judge {
                    Judge::Noul { .. } => "no option",
                    Judge::Choice { .. } => "an option that has a comment",
                }
            );
            acc.entry(l.comment).or_default().push(Label {
                rule: l.rule,
                option: l.option,
            });
            Ok(acc)
        },
    )
}

/// Labels plus live verdicts: a comment judged right is a positive with the
/// option it used; one judged wrong is an explicit negative for its rule.
/// Verdicts for rules that no longer exist, or right verdicts on a choice
/// rule without a commenting option, are left out.
fn with_feedback(
    labels: &BTreeMap<String, Vec<Label>>,
    feedback: &crate::feedback::FeedbackFile,
    rules: &RuleSet,
) -> BTreeMap<String, Marks> {
    use dify_review_core::feedback::Outcome;
    let mut marks: BTreeMap<String, Marks> = labels
        .iter()
        .map(|(url, ls)| {
            (
                url.clone(),
                Marks {
                    labels: ls.clone(),
                    rejected: Vec::new(),
                },
            )
        })
        .collect();
    let by_id: HashMap<&RuleId, &Rule> = rules.rules().iter().map(|r| (&r.id, r)).collect();
    let usable = |e: &crate::feedback::Entry| match by_id.get(&e.rule).map(|r| &r.judge) {
        None => false,
        Some(Judge::Noul { .. }) => true,
        Some(Judge::Choice { options, .. }) => {
            e.outcome == Outcome::Wrong
                || options
                    .iter()
                    .any(|o| Some(&o.name) == e.option.as_ref() && o.comment.is_some())
        }
    };
    let (used, skipped): (Vec<_>, Vec<_>) = feedback
        .feedback
        .iter()
        .filter(|e| matches!(e.outcome, Outcome::Right | Outcome::Wrong))
        .partition(|e| usable(e));
    if !skipped.is_empty() {
        tracing::warn!(
            count = skipped.len(),
            "feedback for unknown rules or options left out"
        );
    }
    used.into_iter().for_each(|e| {
        let m = marks.entry(e.comment.clone()).or_default();
        match e.outcome {
            Outcome::Right => m.labels.push(Label {
                rule: e.rule.clone(),
                option: e.option.clone(),
            }),
            _ => m.rejected.push(e.rule.clone()),
        }
    });
    marks
}

/// Threads keyed by the URL of their first comment.
fn load_threads(path: &Path) -> Result<HashMap<String, ReviewThread>> {
    let lines =
        BufReader::new(File::open(path).with_context(|| format!("opening {}", path.display()))?)
            .lines()
            .collect::<std::io::Result<Vec<_>>>()?;
    lines
        .iter()
        .filter(|l| !l.trim().is_empty())
        .map(|l| {
            let t: ReviewThread = serde_json::from_str(l)?;
            let url = t
                .comments
                .first()
                .map(|c| c.url.clone())
                .unwrap_or_default();
            Ok((url, t))
        })
        .collect()
}

/// The chunk each labelled comment was left on, as it was when the comment
/// was written: the file's diff at the comment's commit, packed as review
/// packs it, and the chunk holding the commented hunk. When the file diff
/// cannot be fetched, GitHub's own `diff_hunk` is used; it stops at the
/// commented line, hiding what follows it (a fixture's body under its `def`).
async fn positive_samples(
    gh: &GitHub,
    threads: &HashMap<String, ReviewThread>,
    marks: &BTreeMap<String, Marks>,
    file_cache: &Path,
) -> Result<Vec<Pending>> {
    let wanted = marks
        .iter()
        .map(|(url, ls)| {
            threads
                .get(url)
                .map(|t| (url, ls, t))
                .with_context(|| format!("labelled comment {url} is not in the threads file"))
        })
        .collect::<Result<Vec<_>>>()?;
    let mut cache = load_file_cache(file_cache)?;
    let missing: Vec<_> = wanted
        .iter()
        .filter(|(url, _, _)| !cache.contains_key(*url))
        .map(|(url, _, t)| ((*url).clone(), *t))
        .collect();
    tracing::info!(cached = cache.len(), to_fetch = missing.len(), "file diffs");
    let fetched = stream::iter(missing)
        .map(|(url, t)| async move {
            let res = file_patch(gh, t, &url).await;
            (url, res)
        })
        .buffer_unordered(4)
        .collect::<Vec<_>>()
        .await;
    let mut out = BufWriter::new(
        OpenOptions::new()
            .create(true)
            .append(true)
            .open(file_cache)?,
    );
    fetched.into_iter().try_for_each(|(url, res)| -> Result<()> {
        match res {
            Ok(patch) => {
                serde_json::to_writer(&mut out, &FileLine { comment: url.clone(), patch: patch.clone() })?;
                out.write_all(b"\n")?;
                cache.insert(url, patch);
            }
            Err(e) => tracing::warn!(%url, error = %e, "file diff unavailable, using the truncated hunk"),
        }
        Ok(())
    })?;
    out.flush()?;

    let mut full = 0usize;
    let all = wanted
        .into_iter()
        .map(|(url, ls, t)| {
            let fetched = cache.get(url).cloned().flatten();
            full += usize::from(fetched.is_some());
            let text = fetched.unwrap_or_else(|| t.diff_hunk.clone());
            let hunks = parse_patch(&text).with_context(|| format!("diff of {url}"))?;
            let chunk = commented_chunk(t, hunks);
            // A file-level comment, or one on a hunk without additions or
            // too large for Jev, has no chunk to judge.
            if chunk.is_none() {
                tracing::warn!(%url, "labelled comment has no chunk to judge, left out");
            }
            Ok(chunk.map(|chunk| Pending {
                origin: url.clone(),
                title: t.pr.title.clone(),
                path: t.path.clone(),
                chunk,
                labels: ls.labels.clone(),
                rejected: ls.rejected.clone(),
            }))
        })
        .collect::<Result<Vec<_>>>()?;
    tracing::info!(full, total = all.len(), "positives with their file diff");
    Ok(all.into_iter().flatten().collect())
}

/// The chunk holding the hunk the comment was left on: the one its
/// `diff_hunk` starts (same header), else the one holding its line.
fn commented_chunk(t: &ReviewThread, hunks: Vec<Hunk>) -> Option<Chunk> {
    let header = t.diff_hunk.lines().next().unwrap_or_default();
    let line = t.original_line.map(NewLine);
    let commented = |h: &Hunk| {
        h.header == header || (line.is_some() && h.lines.iter().any(|l| l.new_line == line))
    };
    Chunk::pack(hunks, MAX_PATCH_CHARS)
        .chunks
        .into_iter()
        .find(|c| c.hunks().iter().any(commented))
}

/// One sample per chunk: comments left on the same file at the same commit
/// share one chunk, which Jev judges in one decision. Counting it once per
/// comment would weigh it several times, and its cached answer (one per
/// request) would stand in for answers that differed. Labels and rejections
/// are merged; the first comment's URL names it.
fn merge_same_chunk(positives: Vec<Pending>) -> Vec<Pending> {
    let mut merged: Vec<Pending> = Vec::new();
    let mut index: HashMap<(String, String, String), usize> = HashMap::new();
    positives.into_iter().for_each(|p| {
        let key = (p.title.clone(), p.path.clone(), p.chunk.render());
        match index.get(&key) {
            Some(&i) => {
                let m = &mut merged[i];
                p.labels.into_iter().for_each(|l| {
                    if !m.labels.contains(&l) {
                        m.labels.push(l);
                    }
                });
                p.rejected.into_iter().for_each(|r| {
                    if !m.rejected.contains(&r) {
                        m.rejected.push(r);
                    }
                });
            }
            None => {
                index.insert(key, merged.len());
                merged.push(p);
            }
        }
    });
    merged
}

/// One fetched file diff; `patch: None` records that it could not be
/// found, so it is not asked again.
#[derive(Serialize, Deserialize)]
struct FileLine {
    comment: String,
    patch: Option<String>,
}

fn load_file_cache(path: &Path) -> Result<HashMap<String, Option<String>>> {
    match path.exists() {
        false => Ok(HashMap::new()),
        true => BufReader::new(File::open(path)?)
            .lines()
            .collect::<std::io::Result<Vec<_>>>()?
            .iter()
            .filter(|l| !l.trim().is_empty())
            .map(|l| {
                let h: FileLine = serde_json::from_str(l)?;
                Ok((h.comment, h.patch))
            })
            .collect(),
    }
}

/// The commented file's diff in the pull request at the comment's commit.
/// Ok(None): the file is not in that diff.
async fn file_patch(gh: &GitHub, t: &ReviewThread, url: &str) -> Result<Option<String>> {
    let id: u64 = url
        .rsplit_once("#discussion_r")
        .and_then(|(_, id)| id.parse().ok())
        .with_context(|| format!("no comment id in {url}"))?;
    let commit = pr::review_comment_commit(gh, &t.repo, id).await?;
    let base = pr::pull_request(gh, &t.repo, t.pr.number).await?.base_ref;
    let files = pr::compare_files(gh, &t.repo, &base, &commit).await?;
    Ok(files.into_iter().find(|f| f.path == t.path).map(|f| {
        f.hunks
            .iter()
            .map(Hunk::render)
            .collect::<Vec<_>>()
            .join("\n")
    }))
}

/// Every chunk in the final diff of the merged pull requests the labels
/// come from: code the reviewer let through. Unmerged pull requests are left
/// out; their last diff may still hold what was objected to.
async fn negative_samples(
    gh: &GitHub,
    threads: &HashMap<String, ReviewThread>,
    labels: &BTreeMap<String, Vec<Label>>,
) -> Result<Vec<Pending>> {
    let prs: BTreeMap<(String, u64), (Repo, String, String)> = labels
        .keys()
        .filter_map(|url| threads.get(url))
        .filter(|t| t.pr.state == PrState::Merged)
        .map(|t| {
            (
                (t.repo.to_string(), t.pr.number),
                (t.repo.clone(), t.pr.title.clone(), t.pr.url.clone()),
            )
        })
        .collect();
    let per_pr = stream::iter(prs)
        .map(|((_, number), (repo, title, pr_url))| async move {
            let files = pr::changed_files(gh, &repo, number).await?;
            Ok::<_, anyhow::Error>(
                files
                    .into_iter()
                    .flat_map(|f| {
                        let title = title.clone();
                        let pr_url = pr_url.clone();
                        let packed = Chunk::pack(f.hunks, MAX_PATCH_CHARS);
                        packed.oversized.iter().for_each(|h| {
                            tracing::warn!(path = %f.path, header = %h.header, "hunk too large, left out")
                        });
                        packed
                            .chunks
                            .into_iter()
                            .map(move |c| Pending {
                                origin: format!("{pr_url}/files {} `{}`", f.path, c.first().header),
                                title: title.clone(),
                                path: f.path.clone(),
                                chunk: c,
                                labels: Vec::new(),
                                rejected: Vec::new(),
                            })
                            .collect::<Vec<_>>()
                    })
                    .collect::<Vec<_>>(),
            )
        })
        .buffer_unordered(4)
        .collect::<Vec<_>>()
        .await;
    per_pr
        .into_iter()
        .collect::<Result<Vec<_>>>()
        .map(|v| v.into_iter().flatten().collect())
}

fn load_cache(path: &Path) -> Result<HashMap<String, BTreeMap<String, Answer>>> {
    match path.exists() {
        false => Ok(HashMap::new()),
        true => BufReader::new(File::open(path)?)
            .lines()
            .collect::<std::io::Result<Vec<_>>>()?
            .iter()
            .filter(|l| !l.trim().is_empty())
            .map(|l| {
                let c: CacheLine = serde_json::from_str(l)?;
                Ok((c.key, c.answers))
            })
            .collect(),
    }
}

/// Answer every sample, from the cache or from Jev. Answers are cached per
/// rule (keyed by the chunk and that rule's question), so editing one rule
/// only re-asks that rule; a chunk's missing rules go out in one request.
/// A failed sample is dropped from the evaluation instead of counting as
/// "no comment".
async fn judge_all(
    jev: &Jev,
    rules: &RuleSet,
    pending: Vec<Pending>,
    cache: &mut HashMap<String, BTreeMap<String, Answer>>,
    cache_path: &Path,
    concurrency: usize,
) -> Result<(Vec<Sample>, usize, u64)> {
    if let Some(dir) = cache_path.parent().filter(|d| !d.as_os_str().is_empty()) {
        fs::create_dir_all(dir)?;
    }
    let mut out = BufWriter::new(
        OpenOptions::new()
            .create(true)
            .append(true)
            .open(cache_path)?,
    );
    let jobs: Vec<Job> = pending
        .into_iter()
        .map(|p| {
            let state = chunk_state(&p.title, &p.path, &p.chunk);
            let keyed = rules
                .rules()
                .iter()
                .filter(|r| r.applies_to(&p.path))
                .map(|r| Ok((r, jev.fingerprint(&state, &[r])?)))
                .collect::<Result<Vec<_>>>()?;
            let cached_answer =
                |r: &Rule, key: &str| cache.get(key).and_then(|m| m.get(&r.id.0)).cloned();
            let known: BTreeMap<String, Answer> = keyed
                .iter()
                .filter_map(|(r, key)| cached_answer(r, key).map(|a| (r.id.0.clone(), a)))
                .collect();
            let missing: Vec<(&Rule, String)> = keyed
                .into_iter()
                .filter(|(r, _)| !known.contains_key(&r.id.0))
                .collect();
            Ok(Job {
                pending: p,
                state,
                known,
                missing,
            })
        })
        .collect::<Result<_>>()?;

    let (complete, to_ask): (Vec<Job>, Vec<Job>) =
        jobs.into_iter().partition(|j| j.missing.is_empty());
    let total = to_ask.len();
    tracing::info!(cached = complete.len(), to_ask = total, "asking Jev");

    let mut samples: Vec<Sample> = complete
        .into_iter()
        .map(|j| j.into_sample(BTreeMap::new()))
        .collect();
    let mut answered = stream::iter(to_ask)
        .map(|j| async move {
            let ask: Vec<&Rule> = j.missing.iter().map(|(r, _)| *r).collect();
            let res = jev.ask(&j.state, &ask).await;
            (j, res)
        })
        .buffer_unordered(concurrency.max(1));
    let mut done = 0usize;
    let mut tokens = 0u64;
    while let Some((j, res)) = answered.next().await {
        done += 1;
        match res {
            Ok((answers, usage)) => {
                tokens += usage.input_tokens;
                j.missing.iter().try_for_each(|(r, key)| -> Result<()> {
                    let one: BTreeMap<String, Answer> = answers
                        .get(&r.id.0)
                        .map(|a| (r.id.0.clone(), a.clone()))
                        .into_iter()
                        .collect();
                    serde_json::to_writer(
                        &mut out,
                        &CacheLine {
                            key: key.clone(),
                            answers: one.clone(),
                        },
                    )?;
                    out.write_all(b"\n")?;
                    cache.insert(key.clone(), one);
                    Ok(())
                })?;
                samples.push(j.into_sample(answers));
            }
            Err(e) => {
                tracing::warn!(path = %j.pending.path, error = %e, "Jev failed, sample left out")
            }
        }
        if done.is_multiple_of(100) {
            out.flush()?;
            tracing::info!(done, total, "progress");
        }
    }
    out.flush()?;
    Ok((samples, total, tokens))
}

/// A sample with the answers already cached and the rules still to ask.
struct Job<'r> {
    pending: Pending,
    state: serde_json::Value,
    known: BTreeMap<String, Answer>,
    /// Rules without a cached answer, with their cache keys.
    missing: Vec<(&'r Rule, String)>,
}

impl Job<'_> {
    fn into_sample(self, fresh: BTreeMap<String, Answer>) -> Sample {
        let mut answers = self.known;
        answers.extend(fresh);
        Sample {
            origin: self.pending.origin,
            path: self.pending.path,
            chunk: self.pending.chunk,
            labels: self.pending.labels,
            rejected: self.pending.rejected,
            answers,
        }
    }
}

/// The row at the suggested threshold, else at the rule's current one.
fn chosen(r: &RuleReport) -> Option<&Row> {
    let t = r.suggested.unwrap_or(r.current);
    r.rows
        .iter()
        .min_by(|a, b| (a.threshold - t).abs().total_cmp(&(b.threshold - t).abs()))
}

fn pct(n: usize, d: usize) -> String {
    match d {
        0 => "-".to_owned(),
        _ => format!("{:.0}%", 100.0 * n as f64 / d as f64),
    }
}

fn print_summary(reports: &[RuleReport]) {
    println!(
        "{:<26} {:>4} {:>5} {:>5} {:>5}  {:>9} {:>6} {:>5} {:>5} {:>9}",
        "rule", "pos", "neg", "now", "best", "hits", "noise", "prec", "fix", "tolerated"
    );
    reports.iter().for_each(|r| {
        let row = chosen(r);
        let best = r
            .suggested
            .map_or("never".to_owned(), |t| format!("{t:.2}"));
        let (hits, noise, prec, fix, tolerated) = row.map_or(
            ("-".into(), "-".into(), "-".into(), "-".into(), "-".into()),
            |row| {
                (
                    format!(
                        "{}/{} {}",
                        row.hits,
                        r.positives,
                        pct(row.hits, r.positives)
                    ),
                    row.noise.to_string(),
                    row.precision()
                        .map_or("-".to_owned(), |p| format!("{:.0}%", 100.0 * p)),
                    pct(row.right_fix, row.hits),
                    row.tolerated.to_string(),
                )
            },
        );
        println!(
            "{:<26} {:>4} {:>5} {:>5.2} {:>5}  {:>9} {:>6} {:>5} {:>5} {:>9}",
            r.rule, r.positives, r.negatives, r.current, best, hits, noise, prec, fix, tolerated
        );
    });
}

fn write_report(path: &Path, reports: &[RuleReport], samples: usize) -> Result<()> {
    if let Some(dir) = path.parent().filter(|d| !d.as_os_str().is_empty()) {
        fs::create_dir_all(dir)?;
    }
    let mut md = vec![
        "# Backtest".to_owned(),
        String::new(),
        format!(
            "{samples} chunks (a file's hunks, judged together as review does). Positives: chunks the reviewer commented on, labelled in `backtest/labels.toml`. \
             Negatives: chunks in the final diff of the merged pull requests those comments come from. \
             *hits*: positives the rule would have commented on; *fix*: of those, with the labelled option; \
             *noise*: negatives without the rule's hint pattern it would have commented on; \
             *tolerated*: negatives that do contain the pattern (the reviewer let it through at the time) it would have commented on. \
             Precision and the suggestion (max F0.5) count noise only."
        ),
    ];
    reports.iter().for_each(|r| {
        md.push(String::new());
        md.push(format!(
            "## {}\n\n{} positives ({} outside the rule's paths), {} negatives, {} live comments judged wrong; threshold now {:.2}, suggested {}.\n",
            r.rule,
            r.positives,
            r.outside_paths,
            r.negatives,
            r.rejected,
            r.current,
            r.suggested.map_or("none (no threshold reaches 50% precision)".to_owned(), |t| format!("{t:.2}"))
        ));
        md.push("| threshold | hits | fix | noise | precision | tolerated |".to_owned());
        md.push("|---|---|---|---|---|---|".to_owned());
        r.rows.iter().for_each(|row| {
            md.push(format!(
                "| {:.2} | {}/{} | {} | {} | {} | {} |",
                row.threshold,
                row.hits,
                r.positives,
                pct(row.right_fix, row.hits),
                row.noise,
                row.precision().map_or("-".to_owned(), |p| format!("{:.0}%", 100.0 * p)),
                row.tolerated
            ));
        });
        let list = |title: &str, items: &[String]| match items.is_empty() {
            true => Vec::new(),
            false => std::iter::once(format!("\n{title} ({}):\n", items.len()))
                .chain(items.iter().take(40).map(|o| format!("- {o}")))
                .collect(),
        };
        md.extend(list("Missed comments", &r.missed));
        md.extend(list("Noise: would comment where the pattern is absent", &r.noise));
        md.extend(list("Tolerated: pattern present, reviewer let it through", &r.tolerated));
    });
    fs::write(path, md.join("\n") + "\n")?;
    Ok(())
}

/// Set `threshold = t` in each rule file whose suggestion differs.
fn apply_thresholds(dir: &Path, reports: &[RuleReport]) -> Result<()> {
    let changed: BTreeSet<_> = reports
        .iter()
        .filter_map(|r| {
            r.suggested
                .filter(|t| (t - r.current).abs() > 1e-9)
                .map(|t| (&r.rule, t))
        })
        .collect::<Vec<_>>()
        .into_iter()
        .map(|(id, t)| {
            let path = dir.join(format!("{id}.toml"));
            let text = fs::read_to_string(&path)?;
            fs::write(&path, set_threshold(&text, t))?;
            eprintln!("{id}: threshold -> {t:.2}");
            Ok::<_, anyhow::Error>(id.0.clone())
        })
        .collect::<Result<_>>()?;
    reports
        .iter()
        .filter(|r| r.suggested.is_none())
        .for_each(|r| {
            eprintln!(
                "{}: no threshold reaches 50% precision; rewrite its question or drop it",
                r.rule
            )
        });
    eprintln!("{} rule file(s) updated", changed.len());
    Ok(())
}

/// Replace the top-level `threshold` line, or add one after `severity`.
fn set_threshold(text: &str, t: f64) -> String {
    let line = format!("threshold = {t:.2}");
    let has = text.lines().any(|l| l.starts_with("threshold = "));
    text.lines()
        .flat_map(|l| {
            match (
                has,
                l.starts_with("threshold = "),
                l.starts_with("severity = "),
            ) {
                (true, true, _) => vec![line.clone()],
                (false, _, true) => vec![l.to_owned(), line.clone()],
                _ => vec![l.to_owned()],
            }
        })
        .collect::<Vec<_>>()
        .join("\n")
        + "\n"
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sets_or_replaces_threshold() {
        let fresh = "# c\nseverity = 'nit'\npaths = ['**']\n";
        assert_eq!(
            set_threshold(fresh, 0.8),
            "# c\nseverity = 'nit'\nthreshold = 0.80\npaths = ['**']\n"
        );
        let again = set_threshold(&set_threshold(fresh, 0.8), 0.65);
        assert_eq!(
            again,
            "# c\nseverity = 'nit'\nthreshold = 0.65\npaths = ['**']\n"
        );
    }
}
