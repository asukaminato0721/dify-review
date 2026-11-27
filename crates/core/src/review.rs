//! From a rule and a chunk of a file to an inline comment: decide from Jev's answer
//! (or the hint alone), anchor on a line, render with a dedupe marker.

use serde::{Deserialize, Serialize};

use crate::diff::{Chunk, LineKind, NewLine};
use crate::rules::{HintScope, Judge, OptionName, Rule, RuleId, Severity};
use crate::size::ReviewLines;
use crate::{MARKER, Repo};

/// Jev's answer to one rule's question.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum Answer {
    Noul(f64),
    Choice { choice: OptionName, confidence: f64 },
}

/// What a rule says about a chunk, before it is placed on a line.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct Verdict {
    pub option: Option<OptionName>,
    /// Probability (noul) or confidence (choice); 1.0 for a hint match.
    pub score: f64,
    pub comment: String,
}

/// Decide about a chunk from Jev's answer at the rule's own threshold.
pub fn decide(rule: &Rule, chunk: &Chunk, answer: &Answer) -> Option<Verdict> {
    decide_at(rule, chunk, answer, rule.threshold)
}

/// A gated rule only speaks where its hint matches an added line.
pub fn passes_gate(rule: &Rule, chunk: &Chunk) -> bool {
    !rule.gate || hint_match(rule, chunk).is_some()
}

/// Decide at a given threshold: the probability of yes (noul) or the
/// confidence in a commenting option (choice). An answer of the wrong shape
/// for the rule is ignored (no comment) rather than guessed at.
pub fn decide_at(rule: &Rule, chunk: &Chunk, answer: &Answer, threshold: f64) -> Option<Verdict> {
    match (&rule.judge, answer) {
        _ if !passes_gate(rule, chunk) => None,
        (Judge::Noul { comment, .. }, Answer::Noul(p)) => (*p >= threshold).then(|| Verdict {
            option: None,
            score: *p,
            comment: comment.clone(),
        }),
        (Judge::Choice { options, .. }, Answer::Choice { choice, confidence }) => options
            .iter()
            .find(|o| &o.name == choice)
            .and_then(|o| o.comment.clone())
            .filter(|_| *confidence >= threshold)
            .map(|comment| Verdict {
                option: Some(choice.clone()),
                score: *confidence,
                comment,
            }),
        _ => None,
    }
}

/// Without Jev: a noul rule comments when its hint matches an added line.
/// Choice rules need Jev to pick the fix, so they stay silent.
pub fn decide_by_hint(rule: &Rule, chunk: &Chunk) -> Option<Verdict> {
    match &rule.judge {
        Judge::Noul { comment, .. } => hint_match(rule, chunk).map(|_| Verdict {
            option: None,
            score: 1.0,
            comment: comment.clone(),
        }),
        Judge::Choice { .. } => None,
    }
}

/// The first line in the rule's hint scope the hint matches. Removed lines
/// never count: a comment can only sit on the new side.
fn hint_match<'h>(rule: &Rule, chunk: &'h Chunk) -> Option<(NewLine, &'h str)> {
    let hint = rule.hint.as_ref()?;
    let in_scope = |kind: LineKind| match rule.hint_scope {
        HintScope::Added => kind == LineKind::Added,
        HintScope::Hunk => kind != LineKind::Removed,
    };
    chunk
        .lines()
        .filter(|l| in_scope(l.kind))
        .find(|l| hint.is_match(&l.text))
        .and_then(|l| l.new_line.map(|n| (n, l.text.as_str())))
}

/// Where the comment goes: the first added line the hint matches, else the
/// first added line of the chunk.
pub fn anchor<'h>(rule: &Rule, chunk: &'h Chunk) -> Option<(NewLine, &'h str)> {
    hint_match(rule, chunk).or_else(|| {
        chunk
            .added()
            .next()
            .and_then(|l| l.new_line.map(|n| (n, l.text.as_str())))
    })
}

#[derive(Debug, Clone, Serialize)]
pub struct Finding {
    pub rule: RuleId,
    pub severity: Severity,
    pub path: String,
    pub line: NewLine,
    pub line_text: String,
    pub verdict: Verdict,
    pub example: Option<String>,
}

impl Finding {
    pub fn new(rule: &Rule, path: &str, chunk: &Chunk, verdict: Verdict) -> Option<Self> {
        let (line, text) = anchor(rule, chunk)?;
        Some(Self {
            rule: rule.id.clone(),
            severity: rule.severity,
            path: path.to_owned(),
            line,
            line_text: text.to_owned(),
            verdict,
            example: rule.examples.first().cloned(),
        })
    }

    /// Stable across pushes that only move the line: rule, path and the
    /// line's trimmed text.
    pub fn key(&self) -> String {
        let input = format!("{}\0{}\0{}", self.rule, self.path, self.line_text.trim());
        format!("{:016x}", fnv1a(input.as_bytes()))
    }

    pub fn body(&self) -> String {
        let source = self
            .example
            .as_deref()
            .map(|u| format!(" Learned from [an earlier review]({u})."))
            .unwrap_or_default();
        format!(
            "{}\n\n<sub>dify-codereview · `{}`{}{} Reply here if this is wrong.</sub>\n{}:rule={}{};key={} -->",
            self.verdict.comment,
            self.rule,
            self.verdict
                .option
                .as_ref()
                .map(|o| format!(" ({o})"))
                .unwrap_or_default(),
            source,
            MARKER,
            self.rule,
            self.verdict
                .option
                .as_ref()
                .map(|o| format!(";option={o}"))
                .unwrap_or_default(),
            self.key(),
        )
    }
}

/// The dedupe keys in a comment body dify-review posted earlier.
pub fn marker_keys(body: &str) -> Vec<String> {
    marker_values(body, "key")
}

/// Values of `field` in every `<!-- dify-review:a=1;b=2 -->` marker of a body.
pub fn marker_values(body: &str, field: &str) -> Vec<String> {
    let prefix = format!("{field}=");
    body.match_indices(MARKER)
        .filter_map(|(i, _)| {
            let rest = &body[i + MARKER.len()..];
            let end = rest.find("-->")?;
            rest[..end]
                .trim_start_matches(':')
                .split(';')
                .find_map(|kv| kv.trim().strip_prefix(prefix.as_str()))
                .map(|v| v.trim().to_owned())
        })
        .collect()
}

/// A new comment needs a score this far above the rule's threshold. Jev's
/// probabilities move a little between identical calls; without a margin a
/// borderline hunk comments on one run and not the next.
pub const NEW_COMMENT_MARGIN: f64 = 0.05;

/// The bar a new comment must reach under `rule`.
pub fn posting_threshold(rule: &Rule) -> f64 {
    (rule.threshold + NEW_COMMENT_MARGIN).min(1.0)
}

/// Marker for a review summary: the head it judged and, when someone asked
/// for it, the comment that asked.
pub fn summary_marker(head: &str, trigger: Option<u64>) -> String {
    let trigger = trigger.map(|t| format!(";trigger={t}")).unwrap_or_default();
    format!("{MARKER}:head={head}{trigger} -->")
}

/// Footer every review summary ends its text with.
const SUMMARY_FOOTER: &str = "\n\n<sub>dify-codereview</sub>\n";

/// What a review summary reports.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SummaryFacts<'a> {
    pub head: &'a str,
    /// Inline comments posted with this review.
    pub new: usize,
    /// Review threads, by anyone, still unresolved (the bot's own outdated
    /// ones excluded: it resolves those).
    pub still_open: usize,
    /// Base branch the head conflicts with.
    pub conflict_base: Option<&'a str>,
    /// Failing CI checks, already written as markdown links.
    pub ci_failing: Option<&'a str>,
    pub requester: Option<&'a str>,
    pub trigger: Option<u64>,
}

/// A review summary: conflict, what was found at which head (linked to the
/// commit in the pull request), what is still open, who asked, and the
/// marker recording the head, the asking comment and a flagged conflict.
pub fn summary_body(repo: &Repo, pr: u64, f: &SummaryFacts<'_>) -> String {
    let who = f
        .requester
        .map(|r| format!("Requested by @{r}. "))
        .unwrap_or_default();
    let short = &f.head[..f.head.len().min(10)];
    let at = format!(
        "[`{short}`](https://github.com/{repo}/pull/{pr}/commits/{})",
        f.head
    );
    let conflict = f
        .conflict_base
        .map(|b| {
            format!("This branch has conflicts with `{b}`: please rebase onto the latest `{b}`. ")
        })
        .unwrap_or_default();
    let ci = f
        .ci_failing
        .map(|list| format!("CI is failing: {list}. Please fix it. "))
        .unwrap_or_default();
    let found = match f.new {
        0 => String::new(),
        1 => format!(
            "1 inline suggestion at {at}, from a rule learned from earlier reviews in this repository. Reply on the comment if it is wrong."
        ),
        n => format!(
            "{n} inline suggestions at {at}, each from a rule learned from earlier reviews in this repository. Reply on a comment if it is wrong."
        ),
    };
    let open = match f.still_open {
        0 => String::new(),
        1 => "1 review thread is still unresolved.".to_owned(),
        n => format!("{n} review threads are still unresolved."),
    };
    let what = match (f.new, f.still_open, conflict.is_empty() && ci.is_empty()) {
        (0, 0, true) => format!("Nothing to flag at {at}."),
        (0, _, _) => format!("Checked {at}. {open}"),
        _ => [found, open]
            .iter()
            .filter(|p| !p.is_empty())
            .cloned()
            .collect::<Vec<_>>()
            .join(" "),
    };
    let mut marker = summary_marker(f.head, f.trigger);
    if f.conflict_base.is_some() {
        marker = marker.replacen(" -->", &format!(";conflict={} -->", f.head), 1);
    }
    if f.ci_failing.is_some() {
        marker = marker.replacen(" -->", &format!(";ci={} -->", f.head), 1);
    }
    format!(
        "{who}{conflict}{ci}{}{SUMMARY_FOOTER}{marker}",
        what.trim_end()
    )
}

/// Asking the author to split a pull request, by how far over it is.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SplitFacts<'a> {
    pub head: &'a str,
    pub lines: ReviewLines,
    /// The threshold it is over: `suggest` or `require`, as `tier` says.
    pub limit: ReviewLines,
    pub tier: SplitAsk,
}

/// The two split notices, each sent at most once per pull request.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SplitAsk {
    /// Over the rule of thumb: still reviewed. Marker `suggest_split=<head>`.
    Suggest,
    /// Too large: not judged until it is split. Marker `split=<head>`.
    Require,
}

impl SplitAsk {
    pub fn marker(self) -> &'static str {
        match self {
            SplitAsk::Suggest => "suggest_split",
            SplitAsk::Require => "split",
        }
    }
}

/// The split notice; its wording follows pyrefly's CONTRIBUTING.
pub fn split_body(f: &SplitFacts<'_>) -> String {
    let marker = summary_marker(f.head, None).replacen(
        " -->",
        &format!(";{}={} -->", f.tier.marker(), f.head),
        1,
    );
    let how = "Split it by feature: each pull request carries one functional change together with its tests, \
               so each feature stands on its own and can be reviewed, merged and reverted independently.";
    let counted = "Size counts added non-test lines; tests, lock files and snapshots do not count.";
    let text = match f.tier {
        SplitAsk::Suggest => format!(
            "Consider splitting this into smaller pull requests: it adds {} lines of non-test code, past about {}. \
             {how} A rule of thumb, not a requirement. {counted}",
            f.lines, f.limit
        ),
        SplitAsk::Require => format!(
            "Please split this into smaller pull requests: it adds {} lines of non-test code, more than {}. \
             {how} It will be reviewed once it is under {}. {counted}",
            f.lines, f.limit, f.limit
        ),
    };
    format!("{text}{SUMMARY_FOOTER}{marker}")
}

/// A split notice, of either kind.
pub fn is_split_notice(body: &str) -> bool {
    [SplitAsk::Suggest, SplitAsk::Require]
        .iter()
        .any(|a| !marker_values(body, a.marker()).is_empty())
}

/// A bot summary that flags a merge conflict or failing CI: stale once
/// that is gone.
pub fn flags_problem(body: &str) -> bool {
    ["conflict", "ci"]
        .iter()
        .any(|f| !marker_values(body, f).is_empty())
}

/// A summary posted in an earlier format, rewritten as `summary_body` would
/// write it now. None when it is current or is not a dify-review summary.
pub fn refreshed_summary(body: &str, repo: &Repo, pr: u64) -> Option<String> {
    let head = marker_values(body, "head").into_iter().next()?;
    // A split notice has its own text; summary_body would rewrite it away.
    (!is_split_notice(body)).then_some(())?;
    let trigger = marker_values(body, "trigger")
        .into_iter()
        .next()
        .and_then(|t| t.parse().ok());
    let text = body.split(SUMMARY_FOOTER).next()?;
    let requester = text
        .strip_prefix("Requested by @")
        .and_then(|r| r.split(". ").next());
    let count_before = |marker: &str| -> Option<usize> {
        text.split(marker)
            .next()
            .filter(|before| before.len() < text.len())?
            .rsplit(' ')
            .next()?
            .parse()
            .ok()
    };
    let conflict_base = text
        .split("conflicts with `")
        .nth(1)
        .and_then(|r| r.split('`').next());
    let ci_failing = text
        .split("CI is failing: ")
        .nth(1)
        .and_then(|r| r.split(". Please fix it.").next());
    let facts = SummaryFacts {
        head: &head,
        new: count_before(" inline suggestion").unwrap_or(0),
        still_open: count_before(" review thread")
            .or_else(|| count_before(" earlier suggestion"))
            .unwrap_or(0),
        conflict_base,
        ci_failing,
        requester,
        trigger,
    };
    let fresh = summary_body(repo, pr, &facts);
    (fresh != body).then_some(fresh)
}

/// Whether `body` mentions `handle` (e.g. `@dify-review`) as a whole word:
/// not inside an e-mail address and not as the prefix of a longer handle.
pub fn mentions(body: &str, handle: &str) -> bool {
    let word = |c: char| c.is_ascii_alphanumeric() || c == '-' || c == '_';
    let lower = body.to_ascii_lowercase();
    let handle = handle.to_ascii_lowercase();
    lower.match_indices(&handle).any(|(i, _)| {
        let before = lower[..i].chars().next_back();
        let after = lower[i + handle.len()..].chars().next();
        !before.is_some_and(word) && !after.is_some_and(word)
    })
}

/// The body a posted comment should have under the current rules: its text
/// replaced by the rule's (or option's) comment, footer and marker kept, so
/// dedupe and feedback still match. None when nothing changes, or when its
/// rule or option no longer exists or no longer comments.
pub fn refreshed_body(body: &str, rules: &[Rule]) -> Option<String> {
    let rule_id = marker_values(body, "rule").into_iter().next()?;
    let option = marker_values(body, "option")
        .into_iter()
        .next()
        .or_else(|| {
            body.split(&format!("`{rule_id}` ("))
                .nth(1)?
                .split(')')
                .next()
                .map(str::to_owned)
        });
    let rule = rules.iter().find(|r| r.id.0 == rule_id)?;
    let text = match (&rule.judge, option) {
        (Judge::Noul { comment, .. }, _) => comment.clone(),
        (Judge::Choice { options, .. }, Some(o)) => options
            .iter()
            .find(|opt| opt.name.0 == o)
            .and_then(|opt| opt.comment.clone())?,
        (Judge::Choice { .. }, None) => None?,
    };
    let (old, rest) = body.split_once("\n\n<sub>")?;
    (old != text).then(|| format!("{text}\n\n<sub>{rest}"))
}

/// FNV-1a: a small hash that is stable across Rust versions and platforms.
pub fn fnv1a(bytes: &[u8]) -> u64 {
    bytes.iter().fold(0xcbf2_9ce4_8422_2325, |h, b| {
        (h ^ u64::from(*b)).wrapping_mul(0x0000_0100_0000_01b3)
    })
}

#[cfg(test)]
pub(crate) mod tests {
    use super::*;
    use crate::diff::{Chunk, parse_patch};

    pub(crate) fn chunk(patch: &str) -> Chunk {
        Chunk::pack(parse_patch(patch).unwrap(), usize::MAX)
            .chunks
            .remove(0)
    }
    use crate::rules::parse_rule;
    use crate::rules::tests::{CHOICE, NOUL};

    fn setup() -> (Vec<Rule>, Chunk) {
        let rules = vec![
            parse_rule("no-type-ignore", NOUL).unwrap(),
            parse_rule("doubles", CHOICE).unwrap(),
        ];
        let hunk = chunk("@@ -1,2 +1,3 @@\n a = 1\n+b = 2\n+c = f()  # type: ignore\n d");
        (rules, hunk)
    }

    #[test]
    fn noul_threshold_and_shape() {
        let (rules, hunk) = setup();
        assert!(decide(&rules[0], &hunk, &Answer::Noul(0.92)).is_some());
        assert!(decide(&rules[0], &hunk, &Answer::Noul(0.5)).is_none());
        let wrong = Answer::Choice {
            choice: OptionName("real".into()),
            confidence: 0.99,
        };
        assert!(decide(&rules[0], &hunk, &wrong).is_none());
    }

    #[test]
    fn choice_comments_only_for_commenting_options() {
        let (rules, hunk) = setup();
        let pick = |name: &str, c| Answer::Choice {
            choice: OptionName(name.into()),
            confidence: c,
        };
        let v = decide(&rules[1], &hunk, &pick("real", 0.9)).unwrap();
        assert_eq!(v.option, Some(OptionName("real".into())));
        assert_eq!(v.comment, "Use the real class.");
        assert!(decide(&rules[1], &hunk, &pick("none", 0.99)).is_none());
        assert!(decide(&rules[1], &hunk, &pick("real", 0.3)).is_none());
        assert!(decide(&rules[1], &hunk, &pick("unknown", 0.99)).is_none());
    }

    #[test]
    fn anchors_on_hint_else_first_added_line() {
        let (rules, hunk) = setup();
        assert_eq!(anchor(&rules[0], &hunk).unwrap().0, NewLine(3));
        assert_eq!(anchor(&rules[1], &hunk).unwrap().0, NewLine(2));
        assert!(decide_by_hint(&rules[0], &hunk).is_some());
        assert!(decide_by_hint(&rules[1], &hunk).is_none());
    }

    #[test]
    fn hint_scope_hunk_also_matches_context_lines() {
        let gated = |scope: &str| {
            let text = format!(
                "severity = 'nit'\npaths = ['**']\ngate = true\nhint_scope = '{scope}'\nhint = 'MagicMock'\n[noul]\nquestion = 'q'\ncomment = 'c'\n"
            );
            crate::rules::parse_rule("r", &text).unwrap()
        };
        let hunk = chunk("@@ -1,2 +1,3 @@\n m = MagicMock()\n+x: int = 1\n y");
        assert!(!passes_gate(&gated("added"), &hunk));
        assert!(passes_gate(&gated("hunk"), &hunk));
        assert_eq!(anchor(&gated("hunk"), &hunk).unwrap().0, NewLine(1));
    }

    #[test]
    fn mention_is_a_whole_handle() {
        assert!(mentions("@dify-review please", "@dify-review"));
        assert!(mentions("cc @Dify-Review.", "@dify-review"));
        assert!(!mentions("@dify-review-bot", "@dify-review"));
        assert!(!mentions("ops@dify-review", "@dify-review"));
        assert!(!mentions("dify-review", "@dify-review"));
    }

    #[test]
    fn summary_marker_round_trips() {
        let body = format!("text\n{}", summary_marker("abc123", Some(42)));
        assert_eq!(marker_values(&body, "head"), vec!["abc123".to_owned()]);
        assert_eq!(marker_values(&body, "trigger"), vec!["42".to_owned()]);
        assert!(marker_values(&summary_marker("abc", None), "trigger").is_empty());
    }

    #[test]
    fn refresh_swaps_only_the_text() {
        let (rules, hunk) = setup();
        let v = decide(&rules[0], &hunk, &Answer::Noul(0.9)).unwrap();
        let body = Finding::new(&rules[0], "api/x.py", &hunk, v)
            .unwrap()
            .body();
        assert_eq!(refreshed_body(&body, &rules), None);
        let stale = body.replacen("Fix the type.", "Old wording.", 1);
        let fresh = refreshed_body(&stale, &rules).unwrap();
        assert_eq!(fresh, body);
        assert_eq!(marker_keys(&fresh), marker_keys(&body));
        assert_eq!(
            refreshed_body(&stale.replace("rule=no-type-ignore", "rule=gone"), &rules),
            None
        );
    }

    #[test]
    fn summary_links_the_commit_and_refreshes_old_ones() {
        let repo: Repo = "o/r".parse().unwrap();
        let head = "d689cee60fabcdef";
        let facts = |new, still_open, conflict_base, requester, trigger| SummaryFacts {
            head,
            new,
            still_open,
            conflict_base,
            ci_failing: None,
            requester,
            trigger,
        };
        let one = summary_body(&repo, 7, &facts(1, 0, None, None, None));
        assert!(one.starts_with("1 inline suggestion at [`d689cee60f`](https://github.com/o/r/pull/7/commits/d689cee60fabcdef), from"));
        assert_eq!(refreshed_summary(&one, &repo, 7), None);
        let old = format!(
            "Requested by @me. 3 inline suggestion(s) at `d689cee60f`, each from a rule.\n\n<sub>dify-codereview</sub>\n{}",
            summary_marker(head, Some(9))
        );
        assert_eq!(
            refreshed_summary(&old, &repo, 7).unwrap(),
            summary_body(&repo, 7, &facts(3, 0, None, Some("me"), Some(9)))
        );
        let nothing = format!(
            "Nothing to flag at `d689cee60f`.\n\n<sub>dify-codereview</sub>\n{}",
            summary_marker(head, None)
        );
        assert_eq!(
            refreshed_summary(&nothing, &repo, 7).unwrap(),
            summary_body(&repo, 7, &facts(0, 0, None, None, None))
        );
        assert_eq!(refreshed_summary("a human review", &repo, 7), None);

        let conflicted = summary_body(&repo, 7, &facts(2, 1, Some("main"), None, None));
        assert!(conflicted.starts_with("This branch has conflicts with `main`: please rebase onto the latest `main`. 2 inline suggestions"));
        assert!(conflicted.contains("1 review thread is still unresolved."));
        let older = conflicted.replace(
            "1 review thread is still unresolved.",
            "1 earlier suggestion is still open.",
        );
        assert_eq!(refreshed_summary(&older, &repo, 7).unwrap(), conflicted);
        assert_eq!(
            marker_values(&conflicted, "conflict"),
            vec![head.to_owned()]
        );
        assert_eq!(refreshed_summary(&conflicted, &repo, 7), None);
        let ci = SummaryFacts {
            ci_failing: Some("[Python Style](https://x/1), [Lint](https://x/2)"),
            ..facts(0, 0, Some("main"), None, None)
        };
        let ci_body = summary_body(&repo, 7, &ci);
        assert!(ci_body.starts_with("This branch has conflicts with `main`: please rebase onto the latest `main`. CI is failing: [Python Style](https://x/1), [Lint](https://x/2). Please fix it. Checked"));
        assert_eq!(marker_values(&ci_body, "ci"), vec![head.to_owned()]);
        assert_eq!(refreshed_summary(&ci_body, &repo, 7), None);
        let only_open = summary_body(&repo, 7, &facts(0, 2, None, None, None));
        assert!(only_open.starts_with("Checked [`d689cee60f`]"));
        assert_eq!(refreshed_summary(&only_open, &repo, 7), None);
    }

    #[test]
    fn split_notices_are_marked_and_left_alone_by_refresh() {
        let repo: Repo = "o/r".parse().unwrap();
        let facts = |tier, limit| SplitFacts {
            head: "abc",
            lines: ReviewLines(555),
            limit: ReviewLines(limit),
            tier,
        };
        let required = split_body(&facts(SplitAsk::Require, 300));
        assert!(required.starts_with(
            "Please split this into smaller pull requests: it adds 555 lines of non-test code, more than 300."
        ));
        assert!(required.contains("one functional change together with its tests"));
        assert_eq!(marker_values(&required, "split"), vec!["abc".to_owned()]);
        assert_eq!(marker_values(&required, "head"), vec!["abc".to_owned()]);
        let suggested = split_body(&facts(SplitAsk::Suggest, 150));
        assert!(suggested.starts_with("Consider splitting this into smaller pull requests"));
        assert!(suggested.contains("past about 150"));
        assert_eq!(
            marker_values(&suggested, "suggest_split"),
            vec!["abc".to_owned()]
        );
        assert!(marker_values(&suggested, "split").is_empty());
        for body in [&required, &suggested] {
            assert!(is_split_notice(body) && !flags_problem(body));
            assert_eq!(refreshed_summary(body, &repo, 1), None);
        }
        assert!(!is_split_notice(&summary_marker("abc", None)));
    }

    #[test]
    fn body_round_trips_its_key() {
        let (rules, hunk) = setup();
        let v = decide(&rules[0], &hunk, &Answer::Noul(0.9)).unwrap();
        let f = Finding::new(&rules[0], "api/x.py", &hunk, v).unwrap();
        let body = f.body();
        assert!(body.starts_with("Fix the type."));
        assert!(body.contains("https://example.test/r1"));
        assert_eq!(marker_keys(&body), vec![f.key()]);
        assert!(marker_keys("no marker here").is_empty());
    }
}
