//! One collected review thread per JSONL line.

use std::fmt;
use std::str::FromStr;

use anyhow::{Error, bail};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

/// Bumped whenever a field changes meaning, so later stages can refuse
/// data they do not understand.
pub const SCHEMA_VERSION: u32 = 1;

/// Hidden marker dify-review puts in every comment it posts, so `collect`
/// never learns from its own output.
pub const MARKER: &str = "<!-- dify-review";

/// Review bots whose GraphQL `__typename` is not always `Bot`.
const BOT_LOGINS: &[&str] = &[
    "copilot",
    "copilot-pull-request-reviewer",
    "coderabbitai",
    "gemini-code-assist",
    "github-advanced-security",
    "sourcery-ai",
    "greptile-apps",
    "dify-review",
];

/// `OWNER/NAME`, validated once at the edge.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(try_from = "String", into = "String")]
pub struct Repo {
    owner: String,
    name: String,
}

impl Repo {
    pub fn owner(&self) -> &str {
        &self.owner
    }
    pub fn name(&self) -> &str {
        &self.name
    }
}

impl FromStr for Repo {
    type Err = Error;
    fn from_str(s: &str) -> Result<Self, Error> {
        let valid = |p: &str| {
            !p.is_empty()
                && p.chars()
                    .all(|c| c.is_ascii_alphanumeric() || matches!(c, '-' | '_' | '.'))
        };
        match s.split_once('/') {
            Some((owner, name)) if valid(owner) && valid(name) => Ok(Self {
                owner: owner.to_owned(),
                name: name.to_owned(),
            }),
            _ => bail!("repository must be OWNER/NAME, got {s:?}"),
        }
    }
}

impl TryFrom<String> for Repo {
    type Error = Error;
    fn try_from(s: String) -> Result<Self, Error> {
        s.parse()
    }
}

impl From<Repo> for String {
    fn from(r: Repo) -> String {
        r.to_string()
    }
}

impl fmt::Display for Repo {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}/{}", self.owner, self.name)
    }
}

/// A GitHub account login.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct Login(pub String);

impl fmt::Display for Login {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

/// GraphQL node id of a review thread; stable, used to dedupe.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct ThreadId(pub String);

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum AuthorKind {
    User,
    Bot,
    /// The account was deleted; GitHub reports no author.
    Ghost,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum PrState {
    Open,
    Merged,
    Closed,
}

impl PrState {
    /// The GraphQL `PullRequestState` value.
    pub fn graphql(self) -> &'static str {
        match self {
            Self::Open => "OPEN",
            Self::Merged => "MERGED",
            Self::Closed => "CLOSED",
        }
    }
}

impl FromStr for PrState {
    type Err = Error;
    fn from_str(s: &str) -> Result<Self, Error> {
        match s.trim().to_ascii_lowercase().as_str() {
            "open" => Ok(Self::Open),
            "merged" => Ok(Self::Merged),
            "closed" => Ok(Self::Closed),
            _ => bail!("unknown pull request state {s:?}; use open, merged or closed"),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Comment {
    pub id: Option<i64>,
    pub author: Option<Login>,
    pub author_kind: AuthorKind,
    pub body: String,
    pub created_at: DateTime<Utc>,
    pub url: String,
}

impl Comment {
    /// True when a bot or an AI reviewer wrote this comment, or when it is
    /// one dify-review posted itself.
    pub fn is_machine(&self) -> bool {
        self.author_kind == AuthorKind::Bot
            || ai_style(&self.body)
            || self.author.as_ref().is_some_and(|Login(login)| {
                let login = login.to_ascii_lowercase();
                login.ends_with("[bot]") || BOT_LOGINS.contains(&login.as_str())
            })
    }
}

/// Comments pasted from an AI reviewer (`**[P1] title**`) or posted by
/// dify-review. Human reviewers here do not write that heading.
pub fn ai_style(body: &str) -> bool {
    let b = body.trim_start().as_bytes();
    body.contains(MARKER)
        || (b.len() >= 6 && b.starts_with(b"**[P") && b[4].is_ascii_digit() && b[5] == b']')
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PullRequestMeta {
    pub number: u64,
    pub url: String,
    pub title: String,
    pub author: Option<Login>,
    pub state: PrState,
    pub created_at: DateTime<Utc>,
    pub merged_at: Option<DateTime<Utc>>,
}

/// An inline review thread: the hunk it was left on and every comment in it.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ReviewThread {
    pub schema: u32,
    pub id: ThreadId,
    pub repo: Repo,
    pub pr: PullRequestMeta,
    pub path: String,
    pub line: Option<u32>,
    pub original_line: Option<u32>,
    pub diff_side: Option<String>,
    /// Someone resolved the thread.
    pub is_resolved: bool,
    /// The commented lines changed after the comment: the author most likely
    /// acted on it.
    pub is_outdated: bool,
    /// The hunk the root comment was left on, ending at the commented line.
    pub diff_hunk: String,
    pub comments: Vec<Comment>,
    pub flags: ThreadFlags,
}

/// Derived facts later stages filter on. Computed from the other fields, so
/// they can be recomputed if the rules here change.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct ThreadFlags {
    /// A human who is not the PR author started the thread.
    pub human_review: bool,
    /// Login of that reviewer.
    pub reviewer: Option<Login>,
    /// The PR author replied in the thread (often explains a terse comment).
    pub author_replied: bool,
    /// Resolved or outdated: the comment was probably acted on.
    pub acted_on: bool,
}

impl ThreadFlags {
    pub fn compute(
        pr_author: Option<&Login>,
        comments: &[Comment],
        is_resolved: bool,
        is_outdated: bool,
    ) -> Self {
        let by_pr_author = |c: &Comment| pr_author.is_some() && c.author.as_ref() == pr_author;
        let human_root = comments
            .first()
            .filter(|root| root.author_kind == AuthorKind::User)
            .filter(|root| !root.is_machine() && !by_pr_author(root));
        Self {
            human_review: human_root.is_some(),
            reviewer: human_root.and_then(|root| root.author.clone()),
            author_replied: comments.iter().skip(1).any(by_pr_author),
            acted_on: !comments.is_empty() && (is_resolved || is_outdated),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn c(author: &str, kind: AuthorKind, body: &str) -> Comment {
        Comment {
            id: Some(1),
            author: Some(Login(author.into())),
            author_kind: kind,
            body: body.into(),
            created_at: DateTime::UNIX_EPOCH,
            url: String::new(),
        }
    }

    fn author() -> Login {
        Login("author".into())
    }

    #[test]
    fn repo_round_trips_and_rejects() {
        let r: Repo = "langgenius/dify".parse().unwrap();
        assert_eq!((r.owner(), r.name()), ("langgenius", "dify"));
        assert_eq!(serde_json::to_string(&r).unwrap(), r#""langgenius/dify""#);
        assert_eq!(
            serde_json::from_str::<Repo>(r#""langgenius/dify""#).unwrap(),
            r
        );
        for bad in ["", "dify", "/dify", "a/", "a/b/c", "a b/c"] {
            assert!(bad.parse::<Repo>().is_err(), "{bad}");
        }
    }

    #[test]
    fn pr_state_parses() {
        assert_eq!(" Merged ".parse::<PrState>().unwrap(), PrState::Merged);
        assert_eq!(PrState::Closed.graphql(), "CLOSED");
        assert!("draft".parse::<PrState>().is_err());
    }

    #[test]
    fn detects_ai_style() {
        assert!(ai_style(
            "**[P1] Serialize structured metric values**\n\n..."
        ));
        assert!(ai_style("  **[P0] x**"));
        assert!(ai_style("nit <!-- dify-review:rule=x -->"));
        assert!(!ai_style("**P"));
        assert!(!ai_style("."));
        assert!(!ai_style("**[Px] x**"));
    }

    #[test]
    fn human_review_with_author_reply() {
        let cs = [
            c("reviewer", AuthorKind::User, "rm"),
            c("author", AuthorKind::User, "Removed in abc123"),
        ];
        let f = ThreadFlags::compute(Some(&author()), &cs, false, true);
        assert!(f.human_review);
        assert_eq!(f.reviewer, Some(Login("reviewer".into())));
        assert!(f.author_replied);
        assert!(f.acted_on);
    }

    #[test]
    fn machine_and_self_comments_are_not_human_review() {
        for root in [
            c("Copilot", AuthorKind::User, "x"),
            c("github-advanced-security", AuthorKind::Bot, "x"),
            c("someone", AuthorKind::User, "**[P1] Enforce entitlement**"),
            c("author", AuthorKind::User, "note to self"),
        ] {
            let f =
                ThreadFlags::compute(Some(&author()), std::slice::from_ref(&root), false, false);
            assert!(!f.human_review, "{root:?}");
            assert_eq!(f.reviewer, None);
        }
    }

    #[test]
    fn empty_thread() {
        assert_eq!(
            ThreadFlags::compute(None, &[], true, true),
            ThreadFlags::default()
        );
    }
}
