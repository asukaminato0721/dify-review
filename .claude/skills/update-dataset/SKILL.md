---
name: update-dataset
description: Refresh dify-review's calibration dataset from the last window of review history - collect new threads, judge what became of the bot's live comments, label the reviewer's new human comments against rules/ (proposing a rule for a repeated new pattern), rerun the backtest, and write a dated snapshot under dataset/. Use when asked to update / refresh / collect the dataset, labels or feedback, or to recalibrate the rules on recent reviews.
---

# Update the dataset

A snapshot is one window of history turned into calibration data, the way
RADAR reports its numbers: by time window, from production outcomes. Two
sources feed the backtest:

- **labels** (`backtest/labels.toml`): human review comments by the reviewer,
  each an instance of a rule (and option). Positives. Needs reading: you.
- **feedback** (`backtest/feedback.toml`): what became of each comment the bot
  posted. Written by `dify-review feedback`; never edit it by hand.

`dataset/reviewed.txt` lists the root URL of every thread already read, labelled
or left out, so each snapshot reads only what is new.

Reviewer: `asukaminato0721` unless the user names another. Binary:
`./target/release/dify-review` (build with `cargo build --release` if stale; that
binary is also what the watch timer runs, so a rebuild ships the current code).

## Unattended

The weekly timer runs `claude -p "/update-dataset unattended"` with the
permissions in `deploy/claude-update-dataset.json`: only the commands below,
edits under `dataset/`, `rules/` and `backtest/labels.toml`, no git, no
`--apply`. Nobody is there to answer: do not ask, decide as the steps say,
and leave every judgment call (a borderline label, a proposed rule) written
in the snapshot for the user to review. A command that is denied is not
retried another way; note it in the summary.

## Steps

1. **Window.** The last snapshot is the newest `dataset/*.md`; the window runs
   from its date to today. If a snapshot for today exists, update it.

2. **Collect and judge.** Days since the last snapshot plus 2 for overlap:
   ```sh
   ./target/release/dify-review collect --since <N>d --append
   ./target/release/dify-review feedback --since 30d
   ```
   `feedback` asks Jev only about threads with human replies (cheap).

3. **Candidates.**
   ```sh
   python3 -I scripts/dataset_candidates.py data/threads.jsonl dataset/reviewed.txt <reviewer> <scratchpad>/candidates.jsonl
   ```
   Read every candidate: the hunk tail, the comment, and the author's reply
   (a reply often explains a terse "rm" or "real class ?").

4. **Label.** For each candidate, decide which rule in `rules/` it is an
   instance of; for a `choice` rule, also the option, which must be one that
   has a comment. One comment may be an instance of two rules (a test that
   needs a `# pyrefly: ignore` to pass a wrong type: `no-test-for-type-errors`
   and `no-type-ignore`). Label what the reviewer asked for even when the
   author pushed back; the pushback shows up in feedback, not here.
   Leave a thread out when it is a question, too vague to generalise
   ("fix the origin code"), or a one-off; write down why.

   Append the labels to `backtest/labels.toml` under a comment header naming
   the snapshot date, and append every candidate's URL, labelled or not, to
   `dataset/reviewed.txt`.

5. **New rules.** When at least 3 left-out comments ask for the same thing,
   propose a rule: `rules/<id>.toml` in the shape of the existing ones, with a
   `hint` regex checked against the actual added lines
   (`python3 -I scripts/check_hint.py REGEX 'line' ... '!line it must not match'`)
   and `gate = true` when the hint alone
   finds the spot; label its instances. Fewer than 3: list it as a candidate
   in the snapshot, to revisit when more arrive.

6. **Backtest.**
   ```sh
   ./target/release/dify-review backtest
   ```
   Run it in the foreground with the longest timeout; if it times out, rerun
   it: answers are cached, so a rerun only asks what is left. Read `backtest/report.md`: per rule, positives,
   hits, noise, the suggested threshold. A rule with "no threshold reaches 50%
   precision" needs its question rewritten or its labels checked against the
   missed list. Do not pass `--apply` yourself: show the user the threshold
   changes and apply them only when they agree.

7. **Snapshot.** Write `dataset/<YYYY-MM-DD>.md` (copy the layout of the
   previous one): the window, what was collected, the table from
   ```sh
   python3 -I scripts/dataset_stats.py backtest/feedback.toml backtest/labels.toml
   ```
   the labels added by rule, rules added or changed, the left-out threads with
   reasons, candidate patterns, the backtest summary, and the caveats.
   Report live precision as what it measures: `right` is mostly "the commented
   lines changed afterwards" (the bot resolves its own outdated threads), not
   "the author accepted it"; say how many rights were of each kind.

8. **Hand back.** Unattended, end with a short summary (headline numbers,
   suggested threshold changes, proposed rules, anything that failed); it
   lands in the journal. Otherwise tell the user the headline numbers, the suggested threshold
   changes, and any proposed rule, and ask before `--apply` and before
   committing. Raw `data/` stays out of git; `dataset/`, `backtest/` and
   `rules/` go in.
