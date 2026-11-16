# dify-review

Learns review rules from the human review history of a GitHub repository and
applies them to new pull requests with [Jev](https://typesafe.ai), a decision
model that answers typed questions with calibrated probabilities. Jev does not
write text, so each rule carries its own comment; Jev only decides whether the
rule applies to a hunk.

The design follows RADAR (*Automating Low-Risk Code Review at Meta*,
arXiv:2605.30208): typed signals, thresholds calibrated on history, fail closed.

## Status

| Stage | Command | State |
|---|---|---|
| Collect review threads | `dify-review collect` | done |
| Distill threads into rules | `scripts/distill_v1.py` (one-off) | done: `rules/*.toml` |
| Review one pull request | `dify-review review --pr N [--post]` | done (dry run by default) |
| Backtest and calibrate thresholds | `dify-review backtest [--apply]` | done: `backtest/report.md` |
| Watch assigned and mentioned pull requests | `dify-review watch --assignee LOGIN [--post]` | done; local timer |
| Judge live comments, recalibrate | `dify-review feedback`, then `backtest` | done; weekly local timer |

## Collect

```sh
cargo run --release -- collect --repo langgenius/dify --since 180d --out data/threads.jsonl
```

Uses `GITHUB_TOKEN`/`GH_TOKEN`, or `gh auth token`. Writes one inline review
thread per line: the pull request, path, the hunk the root comment was left on,
every comment in the thread, and derived flags:

- `human_review`: a human other than the PR author started the thread (bots,
  Copilot-style reviewers and `**[P1] …**` AI comments excluded);
- `author_replied`: the PR author answered, which often explains a terse comment;
- `acted_on`: the thread was resolved or its lines changed afterwards.

`--append` adds only threads not already in the file, so a scheduled job can
keep the history current.

Comments dify-review posts carry `<!-- dify-review`, so it never learns from
its own output.

## Review

```sh
cargo run --release -- review --pr 46483           # dry run: print the comments
cargo run --release -- review --pr 46483 --post    # post them as one COMMENT review
```

Each changed file is one Jev request carrying every rule whose `paths` match
it: the rules' questions are most of a request, so they go once per file, not
once per hunk. The file's hunks with additions are packed together, split into
more requests only past Jev's input limit (a single hunk past it is skipped). A
rule comments when its `noul` probability is at least 0.7, or when
its `choice` picks an option that has a comment with confidence at least 0.6.
The comment goes on the first added line the rule's `hint` matches, else the
first added line of the request's hunks. Comments already on the pull request (by their hidden
marker) are not posted again. Without a Jev key (`--hints-only`, or no
`TYPESAFE_API_KEY`), `noul` rules fall back to their hint regex and `choice`
rules stay silent. A review never approves or requests changes.

Configuration comes from the environment or a local `.env` (gitignored):

| Variable | Meaning |
|---|---|
| `TYPESAFE_API_KEY` | Jev key |
| `JEV_BASE_URL` | Jev endpoint, default `https://api.typesafe.ai/v1` (`/systemone` is appended) |
| `JEV_MODEL` | default `jev-latest` |
| `GITHUB_TOKEN` / `GH_TOKEN` | GitHub token, else `gh auth token` |

Rules live in `rules/<id>.toml`; see `rules/README.md`.

## Backtest

```sh
cargo run --release -- backtest            # report per rule, suggest thresholds
cargo run --release -- backtest --apply    # write the suggestions into rules/*.toml
```

Samples have the shape review sends: a file's hunks, packed the same way.
Positives are the files behind the review comments listed in
`backtest/labels.toml`, as they were when commented on: the file's diff in the
pull request's `base...commit` at the comment's commit (cached in
`data/backtest-files.jsonl`), falling back to GitHub's `diff_hunk`, which stops
at the commented line. Negatives are the changed files in the final diff of
those merged pull requests. A reviewer does
not comment every instance, so a negative that contains the rule's `hint`
pattern counts as *tolerated* (a comment the bot should make), not noise.
The suggested threshold maximises F0.5 among those with at least 50%
precision. Jev answers are cached in `data/backtest-answers.jsonl` by request
fingerprint, so a rerun only asks what changed.

A rule with `gate = true` only comments where its `hint` matches: the regex
finds the spot, Jev confirms it and picks the fix. `hint_scope = 'hunk'`
also matches unchanged lines of a touched hunk (asking the author to fix code
next to their change); the default `added` matches added lines only. The
report lists every missed comment with Jev's answer, to tell a bad question
from a bad label.

## Watch

```sh
cargo run --release -- watch --assignee asukaminato0721 --since 2h          # dry run
cargo run --release -- watch --assignee asukaminato0721 --since 2h --post   # post
```

One pass over the pull requests the bot should look at; run it on a
schedule, with `--since` a little longer than the interval:

- every open pull request assigned to `--assignee`. Status lives in two
  labels, after pyrefly's: `waiting-for-author` (a new suggestion, an
  unresolved review thread by anyone, a merge conflict or failing CI) and
  `waiting-for-reviewer` (nothing left for the author). A head is judged
  once: when a review names it, or a status label was set after it arrived
  (its commit date, a force-push, or its first CI run, whichever is latest;
  the label is re-set to refresh that time). A new head gets a Jev review;
  a judged head only gets cheap checks each pass (merge conflict, failing
  GitHub Actions jobs, except flaky ones named in `--ignore-ci`, default
  `E2E`; unresolved threads) to keep the label right. Drafts
  wait.
  A review is posted only when the author must act: new suggestions, a
  merge conflict (asking for a rebase) or failing CI (linked), each
  requesting changes and each flagged once per head. When the author's
  turn ends with the bot still requesting changes, it says nothing is left
  to flag and dismisses that request. Before reviewing a new head, the bot
  resolves its own comments the push made outdated. On the author's own
  pull request GitHub allows no change request, so it comments instead.
- pull requests where an OWNER, MEMBER or COLLABORATOR mentions
  `@dify-codereview` (the handle follows Meta's `meta-codesync`): each
  mention is answered once, even on a head already reviewed, and always gets
  a reply, if only "nothing to flag".

A new comment needs a score 0.05 above its rule's threshold, since Jev's
scores move a little between calls. A rule leaves at most two comments per
pull request. Posted with a GitHub App's token, the bot appears as
`dify-codereview[bot]`.

## Refresh

```sh
cargo run --release -- refresh --since 30d           # list comments whose wording is out of date
cargo run --release -- refresh --since 30d --apply   # rewrite them
```

After a rule's comment text changes, rewrites the text of comments already
posted from it, and brings review summaries on the same pull requests to the
current format. Footer and marker stay, so dedupe and feedback still match.
