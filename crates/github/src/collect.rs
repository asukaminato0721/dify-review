//! Pull review threads of recently updated pull requests.

use anyhow::{Context, Result};
use chrono::{DateTime, Utc};
use dify_review_core::{
    AuthorKind, Comment, Login, PrState, PullRequestMeta, Repo, ReviewThread, SCHEMA_VERSION,
    ThreadFlags, ThreadId,
};
use serde::Deserialize;
use serde_json::json;

use crate::client::{GitHub, HasRateLimit, RateLimit};

/// Pull requests per page. Each carries up to THREADS x COMMENTS nodes, so
/// keep it small enough that GitHub does not time out.
const PRS_PER_PAGE: i64 = 20;
const THREADS: i64 = 50;
const COMMENTS: i64 = 50;

const FRAGMENTS: &str = r#"
fragment C on PullRequestReviewComment {
  databaseId body createdAt diffHunk url
  author { __typename login }
}
fragment T on PullRequestReviewThread {
  id isResolved isOutdated path line originalLine diffSide
  comments(first: $comments) {
    pageInfo { hasNextPage endCursor }
    nodes { ...C }
  }
}
"#;

const PRS_QUERY: &str = r#"
query($owner: String!, $name: String!, $states: [PullRequestState!], $first: Int!, $after: String, $threads: Int!, $comments: Int!) {
  rateLimit { cost remaining resetAt }
  repository(owner: $owner, name: $name) {
    pullRequests(states: $states, orderBy: {field: UPDATED_AT, direction: DESC}, first: $first, after: $after) {
      pageInfo { hasNextPage endCursor }
      nodes {
        id number url title state createdAt updatedAt mergedAt
        author { login }
        reviewThreads(first: $threads) {
          pageInfo { hasNextPage endCursor }
          nodes { ...T }
        }
      }
    }
  }
}
"#;

const MORE_THREADS_QUERY: &str = r#"
query($id: ID!, $after: String, $threads: Int!, $comments: Int!) {
  rateLimit { cost remaining resetAt }
  node(id: $id) {
    ... on PullRequest {
      reviewThreads(first: $threads, after: $after) {
        pageInfo { hasNextPage endCursor }
        nodes { ...T }
      }
    }
  }
}
"#;

const MORE_COMMENTS_QUERY: &str = r#"
query($id: ID!, $after: String) {
  rateLimit { cost remaining resetAt }
  node(id: $id) {
    ... on PullRequestReviewThread {
      comments(first: 100, after: $after) {
        pageInfo { hasNextPage endCursor }
        nodes { ...C }
      }
    }
  }
}
"#;

// ---- GraphQL response shapes ----

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct PageInfo {
    has_next_page: bool,
    end_cursor: Option<String>,
}

#[derive(Deserialize)]
struct Conn<N> {
    #[serde(rename = "pageInfo")]
    page_info: PageInfo,
    nodes: Vec<N>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct Actor {
    #[serde(rename = "__typename", default)]
    typename: Option<String>,
    login: Login,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct GqlComment {
    database_id: Option<i64>,
    body: String,
    created_at: DateTime<Utc>,
    diff_hunk: String,
    url: String,
    author: Option<Actor>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct GqlThread {
    id: ThreadId,
    is_resolved: bool,
    is_outdated: bool,
    path: String,
    line: Option<u32>,
    original_line: Option<u32>,
    diff_side: Option<String>,
    comments: Conn<GqlComment>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct GqlPr {
    id: String,
    number: u64,
    url: String,
    title: String,
    state: GqlPrState,
    created_at: DateTime<Utc>,
    updated_at: DateTime<Utc>,
    merged_at: Option<DateTime<Utc>>,
    author: Option<Actor>,
    review_threads: Conn<GqlThread>,
}

/// GraphQL `PullRequestState`. An unknown value fails decoding instead of
/// being guessed.
#[derive(Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
enum GqlPrState {
    Open,
    Merged,
    Closed,
}

impl From<GqlPrState> for PrState {
    fn from(s: GqlPrState) -> Self {
        match s {
            GqlPrState::Open => Self::Open,
            GqlPrState::Merged => Self::Merged,
            GqlPrState::Closed => Self::Closed,
        }
    }
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct PrsData {
    rate_limit: Option<RateLimit>,
    repository: Option<PrsRepo>,
}
#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct PrsRepo {
    pull_requests: Conn<GqlPr>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct MoreThreadsData {
    rate_limit: Option<RateLimit>,
    node: Option<MoreThreadsNode>,
}
#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct MoreThreadsNode {
    review_threads: Conn<GqlThread>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct MoreCommentsData {
    rate_limit: Option<RateLimit>,
    node: Option<MoreCommentsNode>,
}
#[derive(Deserialize)]
struct MoreCommentsNode {
    comments: Conn<GqlComment>,
}

macro_rules! rate_limited {
    ($($t:ty),*) => {$(
        impl HasRateLimit for $t {
            fn rate_limit(&self) -> Option<&RateLimit> { self.rate_limit.as_ref() }
        }
    )*};
}
rate_limited!(PrsData, MoreThreadsData, MoreCommentsData);

// ---- public API ----

pub struct CollectOptions {
    pub repo: Repo,
    pub since: DateTime<Utc>,
    pub states: Vec<PrState>,
    pub max_prs: Option<usize>,
}

#[derive(Debug, Default)]
pub struct CollectStats {
    pub prs_seen: usize,
    pub prs_with_threads: usize,
    pub threads: usize,
    pub human_threads: usize,
    pub api_cost: i64,
}

/// Walk pull requests by `updatedAt`, newest first, until one is older than
/// `since`, and hand every review thread to `sink`.
pub async fn collect(
    gh: &GitHub,
    opts: &CollectOptions,
    mut sink: impl FnMut(ReviewThread) -> Result<()>,
) -> Result<CollectStats> {
    let repo = &opts.repo;
    let states: Vec<_> = opts.states.iter().map(|s| s.graphql()).collect();
    let mut stats = CollectStats::default();
    let mut after: Option<String> = None;
    let mut more = true;
    while more {
        let data: PrsData = gh
            .query(
                &format!("{PRS_QUERY}{FRAGMENTS}"),
                json!({
                    "owner": repo.owner(), "name": repo.name(), "states": states,
                    "first": PRS_PER_PAGE, "after": after, "threads": THREADS, "comments": COMMENTS,
                }),
            )
            .await?;
        stats.api_cost += data.rate_limit.as_ref().map_or(0, |r| r.cost);
        let prs = data
            .repository
            .with_context(|| format!("repository {repo} not found"))?
            .pull_requests;

        // Pull requests come newest-updated first: the window ends at the
        // first one older than `since`, or when the cap is reached.
        let budget = opts
            .max_prs
            .map_or(usize::MAX, |m| m.saturating_sub(stats.prs_seen));
        let page_len = prs.nodes.len();
        let in_window: Vec<GqlPr> = prs
            .nodes
            .into_iter()
            .take_while(|pr| pr.updated_at >= opts.since)
            .take(budget)
            .collect();
        more = prs.page_info.has_next_page && in_window.len() == page_len && page_len < budget;
        after = prs.page_info.end_cursor;

        for pr in in_window {
            stats.prs_seen += 1;
            let (meta, threads) = complete_pr(gh, pr, &mut stats).await?;
            stats.prs_with_threads += usize::from(!threads.is_empty());
            for t in threads {
                let t = to_thread(repo, &meta, t);
                stats.threads += 1;
                stats.human_threads += usize::from(t.flags.human_review);
                sink(t)?;
            }
        }
        tracing::info!(
            prs = stats.prs_seen,
            threads = stats.threads,
            human = stats.human_threads,
            remaining = data.rate_limit.as_ref().map(|r| r.remaining),
            "page done"
        );
    }
    Ok(stats)
}

/// Fetch the threads and comments that did not fit in the first page.
async fn complete_pr(
    gh: &GitHub,
    pr: GqlPr,
    stats: &mut CollectStats,
) -> Result<(PullRequestMeta, Vec<GqlThread>)> {
    let meta = PullRequestMeta {
        number: pr.number,
        url: pr.url,
        title: pr.title,
        author: pr.author.map(|a| a.login),
        state: pr.state.into(),
        created_at: pr.created_at,
        merged_at: pr.merged_at,
    };
    let mut threads = pr.review_threads.nodes;
    let mut page = pr.review_threads.page_info;
    while page.has_next_page {
        let d: MoreThreadsData = gh
            .query(
                &format!("{MORE_THREADS_QUERY}{FRAGMENTS}"),
                json!({ "id": pr.id, "after": page.end_cursor, "threads": THREADS, "comments": COMMENTS }),
            )
            .await?;
        stats.api_cost += d.rate_limit.as_ref().map_or(0, |r| r.cost);
        let conn = d
            .node
            .context("pull request vanished while paging threads")?
            .review_threads;
        threads.extend(conn.nodes);
        page = conn.page_info;
    }
    for t in &mut threads {
        while t.comments.page_info.has_next_page {
            // MORE_COMMENTS_QUERY uses only fragment C, and GitHub rejects
            // unused fragments, so send C alone.
            let c_only = FRAGMENTS.split("fragment T").next().unwrap_or_default();
            let d: MoreCommentsData = gh
                .query(
                    &format!("{MORE_COMMENTS_QUERY}{c_only}"),
                    json!({ "id": t.id, "after": t.comments.page_info.end_cursor }),
                )
                .await?;
            stats.api_cost += d.rate_limit.as_ref().map_or(0, |r| r.cost);
            let conn = d
                .node
                .context("thread vanished while paging comments")?
                .comments;
            t.comments.nodes.extend(conn.nodes);
            t.comments.page_info = conn.page_info;
        }
    }
    Ok((meta, threads))
}

fn to_thread(repo: &Repo, pr: &PullRequestMeta, t: GqlThread) -> ReviewThread {
    let diff_hunk = t
        .comments
        .nodes
        .first()
        .map(|c| c.diff_hunk.clone())
        .unwrap_or_default();
    let comments: Vec<Comment> = t
        .comments
        .nodes
        .into_iter()
        .map(|c| {
            let (author, author_kind) = match c.author {
                None => (None, AuthorKind::Ghost),
                Some(a) => {
                    let kind = if a.typename.as_deref() == Some("Bot") {
                        AuthorKind::Bot
                    } else {
                        AuthorKind::User
                    };
                    (Some(a.login), kind)
                }
            };
            Comment {
                id: c.database_id,
                author,
                author_kind,
                body: c.body,
                created_at: c.created_at,
                url: c.url,
            }
        })
        .collect();
    let flags = ThreadFlags::compute(pr.author.as_ref(), &comments, t.is_resolved, t.is_outdated);
    ReviewThread {
        schema: SCHEMA_VERSION,
        id: t.id,
        repo: repo.clone(),
        pr: pr.clone(),
        path: t.path,
        line: t.line,
        original_line: t.original_line,
        diff_side: t.diff_side,
        is_resolved: t.is_resolved,
        is_outdated: t.is_outdated,
        diff_hunk,
        comments,
        flags,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const PAGE: &str = include_str!("../testdata/prs_page.json");

    #[test]
    fn converts_a_page() {
        let env: serde_json::Value = serde_json::from_str(PAGE).unwrap();
        let data: PrsData = serde_json::from_value(env["data"].clone()).unwrap();
        let pr = data
            .repository
            .unwrap()
            .pull_requests
            .nodes
            .into_iter()
            .next()
            .unwrap();
        assert!(!pr.review_threads.page_info.has_next_page);
        let meta = PullRequestMeta {
            number: pr.number,
            url: pr.url.clone(),
            title: pr.title.clone(),
            author: pr.author.as_ref().map(|a| a.login.clone()),
            state: PrState::Merged,
            created_at: pr.created_at,
            merged_at: pr.merged_at,
        };
        let threads: Vec<_> = pr
            .review_threads
            .nodes
            .into_iter()
            .map(|t| to_thread(&"o/r".parse().unwrap(), &meta, t))
            .collect();
        assert_eq!(threads.len(), 2);

        let human = &threads[0];
        assert_eq!(human.path, "api/tests/unit_tests/test_x.py");
        assert!(human.diff_hunk.contains("list[object]()"));
        assert!(human.flags.human_review);
        assert_eq!(human.flags.reviewer, Some(Login("reviewer".into())));
        assert!(human.flags.author_replied);
        assert!(human.flags.acted_on);

        let bot = &threads[1];
        assert_eq!(bot.comments[0].author_kind, AuthorKind::Bot);
        assert!(!bot.flags.human_review);
    }

    #[test]
    fn queries_reference_only_their_fragments() {
        // GitHub rejects documents with unused fragments.
        let c_only = FRAGMENTS.split("fragment T").next().unwrap();
        assert!(c_only.contains("fragment C") && !c_only.contains("fragment T"));
        assert!(MORE_COMMENTS_QUERY.contains("...C") && !MORE_COMMENTS_QUERY.contains("...T"));
    }
}
