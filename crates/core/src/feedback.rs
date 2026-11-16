//! What became of a comment dify-review posted, read from its thread.
//!
//! Wrong beats right: a 👎 or a reply saying the suggestion does not apply
//! outweighs the lines changing afterwards (they may have changed for
//! another reason). Right: 👍, resolved, or the commented lines changed.
//! A merged or closed pull request where nothing happened is "ignored", a
//! weak signal kept apart; an open one is "pending".

use serde::{Deserialize, Serialize};

use crate::review::marker_values;
use crate::rules::{OptionName, RuleId};
use crate::{Comment, PrState, ReviewThread};

/// A reply probability at or above this says the suggestion was wrong.
pub const DISAGREE_THRESHOLD: f64 = 0.7;

/// The question Jev answers about the replies to a bot comment.
pub const DISAGREE_QUESTION: &str = "Do the `replies` say that the review `suggestion` is wrong, does not apply to this code, or is not worth doing? Agreeing, asking a question, or saying it was fixed is not disagreeing.";

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum Outcome {
    Right,
    Wrong,
    Ignored,
    Pending,
}

/// The rule (and option) a dify-review comment came from.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BotComment {
    pub rule: RuleId,
    pub option: Option<OptionName>,
}

/// The thread's root comment, if dify-review posted it.
pub fn bot_comment(thread: &ReviewThread) -> Option<BotComment> {
    let root = thread.comments.first()?;
    let rule = marker_values(&root.body, "rule").into_iter().next()?;
    let option = marker_values(&root.body, "option")
        .into_iter()
        .next()
        .or_else(|| footer_option(&root.body, &rule));
    Some(BotComment {
        rule: RuleId(rule),
        option: option.map(OptionName),
    })
}

/// Comments posted before the option was in the marker name it in the
/// footer: "`rule` (option)".
fn footer_option(body: &str, rule: &str) -> Option<String> {
    let after = body.split(&format!("`{rule}` (")).nth(1)?;
    after.split(')').next().map(str::to_owned)
}

/// What people answered below the bot's comment.
pub fn human_replies(thread: &ReviewThread) -> Vec<&Comment> {
    thread
        .comments
        .iter()
        .skip(1)
        .filter(|c| !c.is_machine())
        .collect()
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Judgement {
    pub outcome: Outcome,
    pub reason: String,
}

/// `disagree`: Jev's probability that the replies reject the suggestion,
/// None when there are no replies.
pub fn judge(thread: &ReviewThread, disagree: Option<f64>) -> Judgement {
    let root = thread.comments.first();
    let thumbs = |f: fn(&Comment) -> u32| root.map_or(0, f);
    let j = |outcome, reason: String| Judgement { outcome, reason };
    match () {
        _ if thumbs(|c| c.thumbs_down) > 0 => j(Outcome::Wrong, "👎".into()),
        _ if disagree.is_some_and(|p| p >= DISAGREE_THRESHOLD) => j(
            Outcome::Wrong,
            format!("reply disagrees ({:.2})", disagree.unwrap_or_default()),
        ),
        _ if thumbs(|c| c.thumbs_up) > 0 => j(Outcome::Right, "👍".into()),
        _ if thread.is_resolved => j(Outcome::Right, "resolved".into()),
        _ if thread.is_outdated => j(Outcome::Right, "lines changed afterwards".into()),
        _ if thread.pr.state == PrState::Open => {
            j(Outcome::Pending, "pull request still open".into())
        }
        _ => j(
            Outcome::Ignored,
            "pull request finished, comment untouched".into(),
        ),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{AuthorKind, Login, PullRequestMeta, Repo, SCHEMA_VERSION, ThreadFlags, ThreadId};
    use jiff::Timestamp;

    fn comment(body: &str, up: u32, down: u32) -> Comment {
        Comment {
            id: None,
            author: Some(Login("someone".into())),
            author_kind: AuthorKind::User,
            body: body.into(),
            created_at: Timestamp::UNIX_EPOCH,
            url: String::new(),
            thumbs_up: up,
            thumbs_down: down,
        }
    }

    fn thread(
        comments: Vec<Comment>,
        state: PrState,
        resolved: bool,
        outdated: bool,
    ) -> ReviewThread {
        ReviewThread {
            schema: SCHEMA_VERSION,
            id: ThreadId("t".into()),
            repo: "o/r".parse::<Repo>().unwrap(),
            pr: PullRequestMeta {
                number: 1,
                url: String::new(),
                title: String::new(),
                author: None,
                state,
                created_at: Timestamp::UNIX_EPOCH,
                merged_at: None,
            },
            path: "a.py".into(),
            line: None,
            original_line: None,
            diff_side: None,
            is_resolved: resolved,
            is_outdated: outdated,
            diff_hunk: String::new(),
            comments,
            flags: ThreadFlags::default(),
        }
    }

    const BOT: &str = "Use the real class.\n\n<sub>dify-codereview · `doubles` (real)</sub>\n<!-- dify-review:rule=doubles;option=real;key=ab -->";

    #[test]
    fn reads_the_marker() {
        let t = thread(vec![comment(BOT, 0, 0)], PrState::Open, false, false);
        let b = bot_comment(&t).unwrap();
        assert_eq!(
            (b.rule.0.as_str(), b.option.unwrap().0.as_str()),
            ("doubles", "real")
        );
        let old = BOT.replace(";option=real", "");
        let t = thread(vec![comment(&old, 0, 0)], PrState::Open, false, false);
        assert_eq!(bot_comment(&t).unwrap().option.unwrap().0, "real");
        let human = thread(
            vec![comment("plain review", 0, 0)],
            PrState::Open,
            false,
            false,
        );
        assert!(bot_comment(&human).is_none());
    }

    #[test]
    fn wrong_beats_right() {
        let o = |t: &ReviewThread, d| judge(t, d).outcome;
        let downvoted = thread(vec![comment(BOT, 1, 1)], PrState::Merged, true, true);
        assert_eq!(o(&downvoted, None), Outcome::Wrong);
        let disputed = thread(
            vec![comment(BOT, 0, 0), comment("no, needed", 0, 0)],
            PrState::Merged,
            true,
            false,
        );
        assert_eq!(o(&disputed, Some(0.9)), Outcome::Wrong);
        assert_eq!(o(&disputed, Some(0.2)), Outcome::Right);
        let changed = thread(vec![comment(BOT, 0, 0)], PrState::Open, false, true);
        assert_eq!(o(&changed, None), Outcome::Right);
        let open = thread(vec![comment(BOT, 0, 0)], PrState::Open, false, false);
        assert_eq!(o(&open, None), Outcome::Pending);
        let done = thread(vec![comment(BOT, 0, 0)], PrState::Merged, false, false);
        assert_eq!(o(&done, None), Outcome::Ignored);
    }

    #[test]
    fn own_comments_are_not_replies() {
        let t = thread(
            vec![comment(BOT, 0, 0), comment(BOT, 0, 0), comment("ok", 0, 0)],
            PrState::Open,
            false,
            false,
        );
        assert_eq!(human_replies(&t).len(), 1);
    }
}
