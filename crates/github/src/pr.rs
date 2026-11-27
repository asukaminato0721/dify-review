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
    pub author: String,
    pub open: bool,
    pub draft: bool,
    /// GitHub's `mergeable_state`; "dirty" means a merge conflict. Unknown
    /// (None) while GitHub is still computing it.
    pub mergeable_state: Option<String>,
    pub labels: Vec<String>,
    pub size: PrSize,
}

/// How much a pull request changes, as GitHub counts it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PrSize {
    pub additions: u64,
    pub deletions: u64,
    pub files: u64,
}

impl PrSize {
    /// Lines a reviewer has to read: added plus removed.
    pub fn lines(&self) -> u64 {
        self.additions + self.deletions
    }
}

impl PullRequest {
    /// The branch conflicts with its base and needs a rebase.
    pub fn conflicted(&self) -> bool {
        self.mergeable_state.as_deref() == Some("dirty")
    }
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
    user: RawUser,
    #[serde(default)]
    mergeable_state: Option<String>,
    #[serde(default)]
    labels: Vec<RawLabel>,
    #[serde(default)]
    additions: u64,
    #[serde(default)]
    deletions: u64,
    #[serde(default)]
    changed_files: u64,
}

#[derive(Deserialize)]
struct RawLabel {
    name: String,
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
        author: pr.user.login,
        open: pr.state == "open",
        draft: pr.draft,
        mergeable_state: pr.mergeable_state,
        labels: pr.labels.into_iter().map(|l| l.name).collect(),
        size: PrSize {
            additions: pr.additions,
            deletions: pr.deletions,
            files: pr.changed_files,
        },
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
/// How a review lands: a plain comment, or a request for changes.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ReviewEvent {
    Comment,
    RequestChanges,
}

impl ReviewEvent {
    fn api(self) -> &'static str {
        match self {
            Self::Comment => "COMMENT",
            Self::RequestChanges => "REQUEST_CHANGES",
        }
    }
}

/// One review. GitHub refuses REQUEST_CHANGES on the reviewer's own pull
/// request; then it is posted as a plain comment instead.
pub async fn post_review(
    gh: &GitHub,
    repo: &Repo,
    pr: &PullRequest,
    summary: &str,
    comments: &[InlineComment],
    event: ReviewEvent,
) -> Result<PostedReview> {
    let comments: Vec<_> = comments
        .iter()
        .map(|c| json!({ "path": c.path, "line": c.line, "side": "RIGHT", "body": c.body }))
        .collect();
    let path = format!("/repos/{repo}/pulls/{}/reviews", pr.number);
    let body = |event: ReviewEvent| json!({ "commit_id": pr.head, "event": event.api(), "body": summary, "comments": comments });
    match gh.post(&path, &body(event)).await {
        Err(e)
            if event == ReviewEvent::RequestChanges
                && e.to_string().contains("your own pull request") =>
        {
            tracing::info!(pr = pr.number, "own pull request: posting as a comment");
            gh.post(&path, &body(ReviewEvent::Comment)).await
        }
        other => other,
    }
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
    /// Heads whose merge conflict was already pointed out.
    pub conflicts: HashSet<String>,
    /// Heads whose failing CI was already pointed out.
    pub ci: HashSet<String>,
    /// Heads at which the author was asked to split the pull request.
    pub splits: HashSet<String>,
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
        conflicts: bodies
            .iter()
            .flat_map(|b| marker_values(b, "conflict"))
            .collect(),
        ci: bodies.iter().flat_map(|b| marker_values(b, "ci")).collect(),
        splits: bodies
            .iter()
            .flat_map(|b| marker_values(b, "split"))
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
    fn judged_when_labelled_after_the_head_arrived() {
        let t = |s: &str| -> Timestamp { s.parse().unwrap() };
        let stamp = |head: Option<&str>, label: Option<&str>| StatusStamp {
            head_arrived: head.map(t),
            labelled: label.map(t),
        };
        assert!(stamp(Some("2026-11-16T10:00:00Z"), Some("2026-11-16T10:05:00Z")).judged());
        assert!(!stamp(Some("2026-11-16T10:05:00Z"), Some("2026-11-16T10:00:00Z")).judged());
        assert!(!stamp(Some("2026-11-16T10:00:00Z"), None).judged());
        assert!(!stamp(None, Some("2026-11-16T10:00:00Z")).judged());
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
/// Open pull requests assigned to `login`: updated since `since`, or all
/// of them when `since` is None.
pub async fn assigned_open(
    gh: &GitHub,
    repo: &Repo,
    login: &str,
    since: Option<Timestamp>,
) -> Result<Vec<u64>> {
    let window = since.map(|s| format!("&since={s}")).unwrap_or_default();
    let issues: Vec<RawIssue> = gh
        .get_all(&format!(
            "/repos/{repo}/issues?assignee={login}&state=open{window}"
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
    pub state: String,
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

/// A review thread on the pull request, by anyone.
#[derive(Debug, Clone)]
pub struct ThreadState {
    /// GraphQL node id, for resolving it.
    pub id: String,
    /// A dify-review comment started it.
    pub bot: bool,
    pub resolved: bool,
    /// The commented lines changed after the comment.
    pub outdated: bool,
    /// The dedupe key in the bot comment's marker.
    pub key: Option<String>,
}

const THREADS_QUERY: &str = r#"
query($owner: String!, $name: String!, $number: Int!, $after: String) {
  rateLimit { cost remaining resetAt }
  repository(owner: $owner, name: $name) {
    pullRequest(number: $number) {
      reviewThreads(first: 100, after: $after) {
        pageInfo { hasNextPage endCursor }
        nodes { id isResolved isOutdated comments(first: 1) { nodes { body } } }
      }
    }
  }
}
"#;

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct ThreadsData {
    rate_limit: Option<crate::client::RateLimit>,
    repository: Option<ThreadsRepo>,
}
#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct ThreadsRepo {
    pull_request: Option<ThreadsPr>,
}
#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct ThreadsPr {
    review_threads: ThreadsConn,
}
#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct ThreadsConn {
    page_info: ThreadsPage,
    nodes: Vec<ThreadNode>,
}
#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct ThreadsPage {
    has_next_page: bool,
    end_cursor: Option<String>,
}
#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct ThreadNode {
    id: String,
    is_resolved: bool,
    is_outdated: bool,
    comments: ThreadComments,
}
#[derive(Deserialize)]
struct ThreadComments {
    nodes: Vec<ThreadComment>,
}
#[derive(Deserialize)]
struct ThreadComment {
    body: String,
}

impl crate::client::HasRateLimit for ThreadsData {
    fn rate_limit(&self) -> Option<&crate::client::RateLimit> {
        self.rate_limit.as_ref()
    }
}

/// Every review thread on the pull request.
pub async fn review_threads(gh: &GitHub, repo: &Repo, number: u64) -> Result<Vec<ThreadState>> {
    let mut found = Vec::new();
    let mut after: Option<String> = None;
    let mut more = true;
    while more {
        let d: ThreadsData = gh
            .query(
                THREADS_QUERY,
                json!({ "owner": repo.owner(), "name": repo.name(), "number": number, "after": after }),
            )
            .await?;
        let conn = d
            .repository
            .and_then(|r| r.pull_request)
            .with_context(|| format!("pull request {number} not found"))?
            .review_threads;
        found.extend(conn.nodes.into_iter().map(|t| {
            let body = t
                .comments
                .nodes
                .first()
                .map(|c| c.body.as_str())
                .unwrap_or_default();
            ThreadState {
                id: t.id,
                bot: !marker_values(body, "rule").is_empty(),
                resolved: t.is_resolved,
                outdated: t.is_outdated,
                key: marker_values(body, "key").into_iter().next(),
            }
        }));
        more = conn.page_info.has_next_page;
        after = conn.page_info.end_cursor;
    }
    Ok(found)
}

#[derive(Deserialize)]
struct Resolved {}

impl crate::client::HasRateLimit for Resolved {
    fn rate_limit(&self) -> Option<&crate::client::RateLimit> {
        None
    }
}

/// Mark a review thread resolved.
pub async fn resolve_thread(gh: &GitHub, thread_id: &str) -> Result<()> {
    let _: Resolved = gh
        .query(
            "mutation($id: ID!) { resolveReviewThread(input: {threadId: $id}) { thread { id } } }",
            json!({ "id": thread_id }),
        )
        .await?;
    Ok(())
}

/// A review summary dify-review posted (it carries the head marker).
#[derive(Debug, Clone)]
pub struct BotReview {
    pub node_id: String,
    pub body: String,
    /// It left inline suggestions: never folded, they are what to act on.
    pub has_inline: bool,
    pub minimized: bool,
}

const BOT_REVIEWS_QUERY: &str = r#"
query($owner: String!, $name: String!, $number: Int!) {
  rateLimit { cost remaining resetAt }
  repository(owner: $owner, name: $name) {
    pullRequest(number: $number) {
      reviews(last: 100) {
        nodes { id body isMinimized comments { totalCount } }
      }
    }
  }
}
"#;

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct BotReviewsData {
    rate_limit: Option<crate::client::RateLimit>,
    repository: Option<BotReviewsRepo>,
}
#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct BotReviewsRepo {
    pull_request: Option<BotReviewsPr>,
}
#[derive(Deserialize)]
struct BotReviewsPr {
    reviews: Nodes<RawBotReview>,
}
#[derive(Deserialize)]
struct Nodes<T> {
    nodes: Vec<T>,
}
#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct RawBotReview {
    id: String,
    #[serde(default)]
    body: String,
    is_minimized: bool,
    comments: Count,
}
#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct Count {
    total_count: u64,
}

impl crate::client::HasRateLimit for BotReviewsData {
    fn rate_limit(&self) -> Option<&crate::client::RateLimit> {
        self.rate_limit.as_ref()
    }
}

/// dify-review's review summaries on the pull request, oldest first.
pub async fn bot_reviews(gh: &GitHub, repo: &Repo, number: u64) -> Result<Vec<BotReview>> {
    let d: BotReviewsData = gh
        .query(
            BOT_REVIEWS_QUERY,
            json!({ "owner": repo.owner(), "name": repo.name(), "number": number }),
        )
        .await?;
    let reviews = d
        .repository
        .and_then(|r| r.pull_request)
        .with_context(|| format!("pull request {number} not found"))?
        .reviews
        .nodes;
    Ok(reviews
        .into_iter()
        .filter(|r| !marker_values(&r.body, "head").is_empty())
        .map(|r| BotReview {
            node_id: r.id,
            body: r.body,
            has_inline: r.comments.total_count > 0,
            minimized: r.is_minimized,
        })
        .collect())
}

/// Fold a review as outdated. Its body, and the markers in it, stay.
pub async fn minimize(gh: &GitHub, node_id: &str) -> Result<()> {
    let _: Resolved = gh
        .query(
            "mutation($id: ID!) { minimizeComment(input: {subjectId: $id, classifier: OUTDATED}) { minimizedComment { isMinimized } } }",
            json!({ "id": node_id }),
        )
        .await?;
    Ok(())
}

/// dify-review's change requests on the pull request that still stand.
pub async fn bot_change_requests(
    gh: &GitHub,
    repo: &Repo,
    number: u64,
) -> Result<Vec<ReviewSummary>> {
    Ok(reviews(gh, repo, number)
        .await?
        .into_iter()
        .filter(|r| r.state == "CHANGES_REQUESTED")
        .filter(|r| {
            r.body
                .as_deref()
                .is_some_and(|b| !marker_values(b, "head").is_empty())
        })
        .collect())
}

/// Dismiss the given change requests, once nothing is left to change.
pub async fn dismiss_reviews(
    gh: &GitHub,
    repo: &Repo,
    number: u64,
    reviews: &[ReviewSummary],
    message: &str,
) -> Result<()> {
    for r in reviews {
        let _: serde_json::Value = gh
            .put(
                &format!("/repos/{repo}/pulls/{number}/reviews/{}/dismissals", r.id),
                &json!({ "message": message, "event": "DISMISS" }),
            )
            .await?;
    }
    Ok(())
}

/// A CI check that failed on a commit.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FailingCheck {
    pub name: String,
    pub url: String,
}

#[derive(Deserialize)]
struct RawCheckRuns {
    check_runs: Vec<RawCheckRun>,
}

#[derive(Deserialize)]
struct RawCheckRun {
    name: String,
    #[serde(default)]
    conclusion: Option<String>,
    html_url: Option<String>,
    #[serde(default)]
    app: Option<RawApp>,
}

#[derive(Deserialize)]
struct RawApp {
    slug: String,
}

/// GitHub Actions jobs that failed on `sha`: the repository's own CI. Checks
/// from other apps (autofix.ci) and commit statuses (codecov coverage gates)
/// are left out, and so are runs still going or cancelled (often superseded
/// by a newer run), and jobs whose name contains one of `ignore` (case
/// insensitive), such as flaky E2E suites.
pub async fn failing_checks(
    gh: &GitHub,
    repo: &Repo,
    sha: &CommitSha,
    ignore: &[String],
) -> Result<Vec<FailingCheck>> {
    let runs: RawCheckRuns = gh
        .get(&format!(
            "/repos/{repo}/commits/{}/check-runs?per_page=100",
            sha.0
        ))
        .await?;
    let mut failing: Vec<FailingCheck> = runs
        .check_runs
        .into_iter()
        .filter(|r| r.app.as_ref().is_some_and(|a| a.slug == "github-actions"))
        .filter(|r| {
            let name = r.name.to_lowercase();
            !ignore.iter().any(|i| name.contains(&i.to_lowercase()))
        })
        .filter(|r| {
            matches!(
                r.conclusion.as_deref(),
                Some("failure" | "timed_out" | "startup_failure")
            )
        })
        .map(|r| FailingCheck {
            url: r.html_url.unwrap_or_default(),
            name: r.name,
        })
        .collect();
    failing.sort_by(|a, b| a.name.cmp(&b.name));
    failing.dedup_by(|a, b| a.name == b.name);
    Ok(failing)
}

/// Status labels, after pyrefly's: whose turn it is on the pull request.
pub const WAITING_FOR_AUTHOR: &str = "waiting-for-author";
pub const WAITING_FOR_REVIEWER: &str = "waiting-for-reviewer";
const STATUS_LABELS: [&str; 2] = [WAITING_FOR_AUTHOR, WAITING_FOR_REVIEWER];

/// Set the status label. It is removed and added again even when already
/// there, so the label event's time records when the head was judged.
pub async fn set_status(gh: &GitHub, repo: &Repo, pr: &PullRequest, label: &str) -> Result<()> {
    for old in pr
        .labels
        .iter()
        .filter(|l| STATUS_LABELS.contains(&l.as_str()))
    {
        gh.delete(&format!("/repos/{repo}/issues/{}/labels/{old}", pr.number))
            .await?;
    }
    let _: serde_json::Value = gh
        .post(
            &format!("/repos/{repo}/issues/{}/labels", pr.number),
            &json!({ "labels": [label] }),
        )
        .await?;
    Ok(())
}

const STAMP_QUERY: &str = r#"
query($owner: String!, $name: String!, $number: Int!) {
  rateLimit { cost remaining resetAt }
  repository(owner: $owner, name: $name) {
    pullRequest(number: $number) {
      commits(last: 1) { nodes { commit { committedDate } } }
      timelineItems(last: 100, itemTypes: [LABELED_EVENT, HEAD_REF_FORCE_PUSHED_EVENT]) {
        nodes {
          __typename
          ... on LabeledEvent { createdAt label { name } }
          ... on HeadRefForcePushedEvent { createdAt }
        }
      }
    }
  }
}
"#;

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct StampData {
    rate_limit: Option<crate::client::RateLimit>,
    repository: Option<StampRepo>,
}
#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct StampRepo {
    pull_request: Option<StampPr>,
}
#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct StampPr {
    commits: StampCommits,
    timeline_items: StampItems,
}
#[derive(Deserialize)]
struct StampCommits {
    nodes: Vec<StampCommitNode>,
}
#[derive(Deserialize)]
struct StampCommitNode {
    commit: StampCommit,
}
#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct StampCommit {
    committed_date: Timestamp,
}
#[derive(Deserialize)]
struct StampItems {
    nodes: Vec<StampItem>,
}
#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct StampItem {
    #[serde(rename = "__typename")]
    kind: String,
    created_at: Option<Timestamp>,
    #[serde(default)]
    label: Option<RawLabel>,
}

impl crate::client::HasRateLimit for StampData {
    fn rate_limit(&self) -> Option<&crate::client::RateLimit> {
        self.rate_limit.as_ref()
    }
}

#[derive(Deserialize)]
struct RawCheckSuites {
    check_suites: Vec<RawCheckSuite>,
}
#[derive(Deserialize)]
struct RawCheckSuite {
    created_at: Timestamp,
}

/// When the current head arrived, and when a status label was last set.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct StatusStamp {
    /// The latest of: the head commit's date, the last force-push, and the
    /// first CI check suite on the head (created when it was pushed, which
    /// catches commits made long before they were pushed).
    pub head_arrived: Option<Timestamp>,
    pub labelled: Option<Timestamp>,
}

impl StatusStamp {
    /// A status label was set after the head arrived: the head was judged.
    pub fn judged(&self) -> bool {
        matches!((self.head_arrived, self.labelled), (Some(h), Some(l)) if l >= h)
    }
}

pub async fn status_stamp(gh: &GitHub, repo: &Repo, pr: &PullRequest) -> Result<StatusStamp> {
    let d: StampData = gh
        .query(
            STAMP_QUERY,
            json!({ "owner": repo.owner(), "name": repo.name(), "number": pr.number }),
        )
        .await?;
    let p = d
        .repository
        .and_then(|r| r.pull_request)
        .with_context(|| format!("pull request {} not found", pr.number))?;
    let committed = p.commits.nodes.last().map(|n| n.commit.committed_date);
    let latest = |kind: &str, pick: &dyn Fn(&StampItem) -> bool| {
        p.timeline_items
            .nodes
            .iter()
            .filter(|i| i.kind == kind && pick(i))
            .filter_map(|i| i.created_at)
            .max()
    };
    let force_pushed = latest("HeadRefForcePushedEvent", &|_| true);
    let labelled = latest("LabeledEvent", &|i| {
        i.label
            .as_ref()
            .is_some_and(|l| STATUS_LABELS.contains(&l.name.as_str()))
    });
    let suites: RawCheckSuites = gh
        .get(&format!(
            "/repos/{repo}/commits/{}/check-suites?per_page=100",
            pr.head.0
        ))
        .await?;
    let first_ci = suites.check_suites.iter().map(|s| s.created_at).min();
    Ok(StatusStamp {
        head_arrived: [committed, force_pushed, first_ci]
            .into_iter()
            .flatten()
            .max(),
        labelled,
    })
}
