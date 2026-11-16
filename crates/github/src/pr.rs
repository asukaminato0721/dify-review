//! One pull request over REST: its head, its file patches, the markers of
//! comments dify-review already left, and posting a COMMENT review.

use std::collections::HashSet;

use anyhow::{Context, Result};
use dify_review_core::MARKER;
use dify_review_core::Repo;
use dify_review_core::diff::{Hunk, NewLine, parse_patch};
use dify_review_core::review::{marker_keys, marker_values, mentions};
use jiff::Timestamp;
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

#[derive(Deserialize)]
struct RawReview {
    #[serde(default)]
    body: Option<String>,
}

/// What earlier dify-review summaries on this pull request record: the
/// heads already judged and the trigger comments already answered.
#[derive(Debug, Default)]
pub struct History {
    pub heads: HashSet<String>,
    pub triggers: HashSet<u64>,
}

pub async fn history(gh: &GitHub, repo: &Repo, number: u64) -> Result<History> {
    let reviews: Vec<RawReview> = gh
        .get_all(&format!("/repos/{repo}/pulls/{number}/reviews"))
        .await?;
    let bodies: Vec<String> = reviews.into_iter().filter_map(|r| r.body).collect();
    Ok(History {
        heads: bodies
            .iter()
            .flat_map(|b| marker_values(b, "head"))
            .collect(),
        triggers: bodies
            .iter()
            .flat_map(|b| marker_values(b, "trigger"))
            .filter_map(|t| t.parse().ok())
            .collect(),
    })
}

/// A comment that asks for a review.
#[derive(Debug, Clone)]
pub struct Trigger {
    pub comment_id: u64,
    pub pr: u64,
    pub author: String,
    pub url: String,
    pub at: Timestamp,
}

#[derive(Deserialize)]
struct RawUser {
    login: String,
}

/// A pull request conversation comment (issues API) or an inline review
/// comment (pulls API); the URL that names the pull request differs.
#[derive(Deserialize)]
struct RawAnyComment {
    id: u64,
    body: String,
    html_url: String,
    author_association: String,
    updated_at: Timestamp,
    user: Option<RawUser>,
    #[serde(default)]
    issue_url: Option<String>,
    #[serde(default)]
    pull_request_url: Option<String>,
}

/// Associations allowed to summon the bot: people with a say in the repo.
const TRUSTED: &[&str] = &["OWNER", "MEMBER", "COLLABORATOR"];

/// Comments updated since `since` on pull requests that mention `handle`,
/// written by a trusted member, oldest first. dify-review's own comments
/// (they carry the marker) never trigger.
pub async fn triggers_since(
    gh: &GitHub,
    repo: &Repo,
    since: Timestamp,
    handle: &str,
) -> Result<Vec<Trigger>> {
    let query = format!("since={since}&sort=updated&direction=asc");
    let conversation: Vec<RawAnyComment> = gh
        .get_all(&format!("/repos/{repo}/issues/comments?{query}"))
        .await?;
    let inline: Vec<RawAnyComment> = gh
        .get_all(&format!("/repos/{repo}/pulls/comments?{query}"))
        .await?;
    Ok(select_triggers(
        conversation.into_iter().chain(inline),
        handle,
    ))
}

fn select_triggers(comments: impl Iterator<Item = RawAnyComment>, handle: &str) -> Vec<Trigger> {
    let mut found: Vec<Trigger> = comments
        .filter(|c| c.html_url.contains("/pull/"))
        .filter(|c| TRUSTED.contains(&c.author_association.as_str()))
        .filter(|c| !c.body.contains(MARKER) && mentions(&c.body, handle))
        .filter_map(|c| {
            let pr = c
                .issue_url
                .as_deref()
                .or(c.pull_request_url.as_deref())
                .and_then(|u| u.rsplit('/').next())
                .and_then(|n| n.parse().ok())?;
            Some(Trigger {
                comment_id: c.id,
                pr,
                author: c.user.map(|u| u.login).unwrap_or_default(),
                url: c.html_url,
                at: c.updated_at,
            })
        })
        .collect();
    found.sort_by_key(|t| t.at);
    found
}

#[cfg(test)]
mod tests {
    use super::*;

    fn comment(id: u64, assoc: &str, body: &str, url: &str) -> RawAnyComment {
        serde_json::from_value(serde_json::json!({
            "id": id, "body": body, "html_url": url, "author_association": assoc,
            "updated_at": format!("2026-11-16T0{id}:00:00Z"), "user": {"login": "someone"},
            "issue_url": "https://api.github.com/repos/o/r/issues/123",
        }))
        .unwrap()
    }

    #[test]
    fn only_trusted_mentions_on_pull_requests_trigger() {
        let pr = "https://github.com/o/r/pull/123#issuecomment-1";
        let got = select_triggers(
            [
                comment(1, "MEMBER", "@dify-codereview please", pr),
                comment(2, "CONTRIBUTOR", "@dify-codereview please", pr),
                comment(
                    3,
                    "MEMBER",
                    "@dify-codereview on an issue",
                    "https://github.com/o/r/issues/9#c",
                ),
                comment(4, "OWNER", "no mention here", pr),
                comment(5, "MEMBER", "@dify-codereview-bot", pr),
                comment(
                    6,
                    "COLLABORATOR",
                    "x <!-- dify-review:head=a --> @dify-codereview",
                    pr,
                ),
                comment(7, "COLLABORATOR", "again @dify-codereview", pr),
            ]
            .into_iter(),
            "@dify-codereview",
        );
        let ids: Vec<u64> = got.iter().map(|t| t.comment_id).collect();
        assert_eq!(ids, vec![1, 7]);
        assert!(got.iter().all(|t| t.pr == 123));
    }
}

#[derive(Deserialize)]
struct RawIssue {
    number: u64,
    /// Present only when the issue is a pull request.
    #[serde(default)]
    pull_request: Option<serde_json::Value>,
}

/// Open pull requests assigned to `login` and updated since `since`.
pub async fn assigned_open(
    gh: &GitHub,
    repo: &Repo,
    login: &str,
    since: Timestamp,
) -> Result<Vec<u64>> {
    let issues: Vec<RawIssue> = gh
        .get_all(&format!(
            "/repos/{repo}/issues?assignee={login}&state=open&since={since}"
        ))
        .await?;
    Ok(issues
        .into_iter()
        .filter(|i| i.pull_request.is_some())
        .map(|i| i.number)
        .collect())
}

/// An inline comment dify-review posted.
#[derive(Debug, Clone)]
pub struct PostedComment {
    pub id: u64,
    pub url: String,
    pub body: String,
    pub pr: u64,
}

#[derive(Deserialize)]
struct RawPostedComment {
    id: u64,
    html_url: String,
    body: String,
    pull_request_url: String,
}

/// Every dify-review inline comment in the repository updated since `since`.
pub async fn bot_comments_since(
    gh: &GitHub,
    repo: &Repo,
    since: Timestamp,
) -> Result<Vec<PostedComment>> {
    let all: Vec<RawPostedComment> = gh
        .get_all(&format!(
            "/repos/{repo}/pulls/comments?since={since}&sort=updated&direction=asc"
        ))
        .await?;
    Ok(all
        .into_iter()
        .filter(|c| !marker_values(&c.body, "rule").is_empty())
        .filter_map(|c| {
            let pr = c.pull_request_url.rsplit('/').next()?.parse().ok()?;
            Some(PostedComment {
                id: c.id,
                url: c.html_url,
                body: c.body,
                pr,
            })
        })
        .collect())
}

/// Replace a review comment's body.
pub async fn edit_comment(gh: &GitHub, repo: &Repo, id: u64, body: &str) -> Result<()> {
    let _: serde_json::Value = gh
        .patch(
            &format!("/repos/{repo}/pulls/comments/{id}"),
            &serde_json::json!({ "body": body }),
        )
        .await?;
    Ok(())
}

/// A review on a pull request, with the id needed to edit it.
#[derive(Debug, Clone, Deserialize)]
pub struct ReviewSummary {
    pub id: u64,
    #[serde(default)]
    pub body: Option<String>,
    pub html_url: String,
}

pub async fn reviews(gh: &GitHub, repo: &Repo, number: u64) -> Result<Vec<ReviewSummary>> {
    gh.get_all(&format!("/repos/{repo}/pulls/{number}/reviews"))
        .await
}

/// Replace a review's summary text.
pub async fn edit_review(gh: &GitHub, repo: &Repo, number: u64, id: u64, body: &str) -> Result<()> {
    let _: serde_json::Value = gh
        .put(
            &format!("/repos/{repo}/pulls/{number}/reviews/{id}"),
            &serde_json::json!({ "body": body }),
        )
        .await?;
    Ok(())
}
