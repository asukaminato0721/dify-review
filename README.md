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
| Scheduled review of open pull requests | GitHub Actions cron | planned |

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

Each changed hunk is one Jev request carrying every rule whose `paths` match
the file; a rule comments when its `noul` probability is at least 0.7, or when
its `choice` picks an option that has a comment with confidence at least 0.6.
The comment goes on the first added line the rule's `hint` matches, else the
hunk's first added line. Comments already on the pull request (by their hidden
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

Positives are the hunks behind the review comments listed in
`backtest/labels.toml` (as they were when commented on). Negatives are the
added hunks in the final diff of those merged pull requests. A reviewer does
not comment every instance, so a negative that contains the rule's `hint`
pattern counts as *tolerated* (a comment the bot should make), not noise.
The suggested threshold maximises F0.5 among those with at least 50%
precision. Jev answers are cached in `data/backtest-answers.jsonl` by request
fingerprint, so a rerun only asks what changed.

A rule with `gate = true` only comments where its `hint` matches an added
line: the regex finds the spot, Jev confirms it and picks the fix.
