# Running dify-review

## On GitHub Actions (this repository)

- `rolling.yml`: every push to main publishes the binary to the `rolling` release.
- `watch.yml`: every 15 minutes, `watch --assignee $ASSIGNEE --since 1h --post`.
- `calibrate.yml`: every Monday, `collect --append`, `feedback`, `backtest --apply`,
  then a pull request with the changes in `rules/` and `backtest/`. `data/` is kept
  in the `data` release.

Settings:

| Kind | Name | Value |
|---|---|---|
| secret | `TYPESAFE_API_KEY` | Jev key |
| variable | `JEV_BASE_URL` | `https://api.tokener.ai/typesafe/v1` |
| variable | `ASSIGNEE` | whose assigned pull requests to review |
| variable + secret | `APP_ID` + `APP_PRIVATE_KEY` | the dify-codereview App, installed on langgenius/dify |
| secret | `DIFY_REVIEW_TOKEN` | fallback until the App is installed: a token that can comment on langgenius/dify |

Repository setting: Actions may create pull requests (`calibrate.yml` uses
`GITHUB_TOKEN` for that). Stop the local timers once Actions runs, or both
will review.

## Locally (systemd user timer)

```sh
cargo build --release
systemctl --user link "$PWD/deploy/systemd/dify-review-watch.service" "$PWD/deploy/systemd/dify-review-watch.timer"
systemctl --user enable --now dify-review-watch.timer
journalctl --user -u dify-review-watch -f        # follow passes
systemctl --user disable --now dify-review-watch.timer   # stop
```

Comments are posted with your own GitHub token until the dify-codereview
App is installed.

## Weekly dataset update

```sh
systemctl --user link "$PWD/deploy/systemd/dify-review-calibrate.service" "$PWD/deploy/systemd/dify-review-calibrate.timer"
systemctl --user enable --now dify-review-calibrate.timer
journalctl --user -u dify-review-calibrate      # the run's summary
```

Every Monday at 09:00, Claude Code runs the `update-dataset` skill
(`.claude/skills/update-dataset/`) unattended: `collect --append`,
`feedback`, labelling the reviewer's new comments, `backtest`, and a
snapshot in `dataset/<date>.md`. Its permissions are only those in
`deploy/claude-update-dataset.json`: the commands it needs, edits under
`dataset/`, `rules/` and `backtest/labels.toml`, no git and no `--apply`.
Read the snapshot and `git diff`, then `dify-review backtest --apply` and
commit. Interactively, `/update-dataset` in Claude Code does the same and
asks before applying.

A 👎 on a bot comment, or a reply saying it does not apply, marks it wrong;
👍, resolving the thread or changing the commented lines marks it right.

## Reusing the binary

Every push to main publishes `dify-review-x86_64-linux` to the `rolling`
release:

```sh
gh release download rolling -R OWNER/dify-review -p 'dify-review-x86_64-linux*'
sha256sum -c dify-review-x86_64-linux.sha256 && chmod +x dify-review-x86_64-linux
```
