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
| Distill threads into rules | — | next |
| Backtest rules with Jev | `dify-review backtest` | planned |
| Review new pull requests (shadow) | `dify-review review` | planned |

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
