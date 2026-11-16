//! One pull request over REST: its head, its file patches, the markers of
//! comments dify-review already left, and posting a COMMENT review.

use std::collections::HashSet;

use anyhow::{Context, Result};
use dify_review_core::Repo;
use dify_review_core::diff::{Hunk, NewLine, parse_patch};
use dify_review_core::review::marker_keys;
use serde::{Deserialize, Serialize};
use serde_json::json;

use crate::client::GitHub;

/// A commit SHA as GitHub reports it.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(transparent)]
pub struct CommitSha(pub String);

#[derive(Debug, Clone)]
pub struct PullRequest {
    pub number: u64,
    pub title: String,
    pub head: CommitSha,
    /// The branch it merges into, e.g. `main`.
    pub base_ref: String,
    pub open: bool,
    pub draft: bool,
}

#[derive(Debug, Clone)]
pub struct ChangedFile {
    pub path: String,
    pub hunks: Vec<Hunk>,
}

#[derive(Deserialize)]
struct RawPr {
    number: u64,
    title: String,
    state: String,
    #[serde(default)]
    draft: bool,
    head: RawHead,
    base: RawBase,
}

#[derive(Deserialize)]
struct RawHead {
    sha: CommitSha,
}

#[derive(Deserialize)]
struct RawBase {
    #[serde(rename = "ref")]
    branch: String,
}

#[derive(Deserialize)]
struct RawFile {
    filename: String,
    status: String,
    /// Absent for binary files and for patches GitHub considers too large.
    #[serde(default)]
    patch: Option<String>,
}

#[derive(Deserialize)]
struct RawComment {
    body: String,
}

pub async fn pull_request(gh: &GitHub, repo: &Repo, number: u64) -> Result<PullRequest> {
    let pr: RawPr = gh.get(&format!("/repos/{repo}/pulls/{number}")).await?;
    Ok(PullRequest {
        number: pr.number,
        title: pr.title,
        head: pr.head.sha,
        base_ref: pr.base.branch,
        open: pr.state == "open",
        draft: pr.draft,
    })
}

/// Files with a reviewable patch. Removed files and files without a patch
/// (binary or too large) are skipped: there is no added line to comment on.
pub async fn changed_files(gh: &GitHub, repo: &Repo, number: u64) -> Result<Vec<ChangedFile>> {
    let files: Vec<RawFile> = gh
        .get_all(&format!("/repos/{repo}/pulls/{number}/files"))
        .await?;
    reviewable(files)
}

#[derive(Deserialize)]
struct RawCompare {
    #[serde(default)]
    files: Vec<RawFile>,
}

/// The pull request's diff as of `head`: what `base...head` shows, the
/// changes since the branch left `base`.
pub async fn compare_files(
    gh: &GitHub,
    repo: &Repo,
    base: &str,
    head: &CommitSha,
) -> Result<Vec<ChangedFile>> {
    let cmp: RawCompare = gh
        .get(&format!("/repos/{repo}/compare/{base}...{}", head.0))
        .await?;
    reviewable(cmp.files)
}

#[derive(Deserialize)]
struct RawReviewComment {
    original_commit_id: CommitSha,
}

/// The commit a review comment was written against.
pub async fn review_comment_commit(gh: &GitHub, repo: &Repo, comment_id: u64) -> Result<CommitSha> {
    let c: RawReviewComment = gh
        .get(&format!("/repos/{repo}/pulls/comments/{comment_id}"))
        .await?;
    Ok(c.original_commit_id)
}

fn reviewable(files: Vec<RawFile>) -> Result<Vec<ChangedFile>> {
    files
        .into_iter()
        .filter(|f| f.status != "removed")
        .filter_map(|f| {
            let skipped = f.patch.is_none();
            if skipped {
                tracing::info!(path = %f.filename, "no patch (binary or too large), skipped");
            }
            f.patch.map(|p| (f.filename, p))
        })
        .map(|(path, patch)| {
            let hunks = parse_patch(&patch).with_context(|| format!("parsing patch of {path}"))?;
            Ok(ChangedFile { path, hunks })
        })
        .collect()
}

/// Dedupe keys of every inline comment dify-review already left on the PR.
pub async fn posted_keys(gh: &GitHub, repo: &Repo, number: u64) -> Result<HashSet<String>> {
    let comments: Vec<RawComment> = gh
        .get_all(&format!("/repos/{repo}/pulls/{number}/comments"))
        .await?;
    Ok(comments.iter().flat_map(|c| marker_keys(&c.body)).collect())
}

#[derive(Debug, Clone, Serialize)]
pub struct InlineComment {
    pub path: String,
    pub line: NewLine,
    pub body: String,
}

#[derive(Deserialize)]
pub struct PostedReview {
    pub html_url: String,
}

/// One review with event COMMENT: never approves, never requests changes.
pub async fn post_review(
    gh: &GitHub,
    repo: &Repo,
    pr: &PullRequest,
    summary: &str,
    comments: &[InlineComment],
) -> Result<PostedReview> {
    let comments: Vec<_> = comments
        .iter()
        .map(|c| json!({ "path": c.path, "line": c.line, "side": "RIGHT", "body": c.body }))
        .collect();
    gh.post(
        &format!("/repos/{repo}/pulls/{}/reviews", pr.number),
        &json!({ "commit_id": pr.head, "event": "COMMENT", "body": summary, "comments": comments }),
    )
    .await
}
