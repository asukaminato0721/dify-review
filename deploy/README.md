# Running dify-review

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

## Weekly feedback and calibration

```sh
systemctl --user link "$PWD/deploy/systemd/dify-review-calibrate.service" "$PWD/deploy/systemd/dify-review-calibrate.timer"
systemctl --user enable --now dify-review-calibrate.timer
```

Every Monday: `collect --append` (new review history), `feedback` (verdicts
on the bot's own comments, into `backtest/feedback.toml`), `backtest` (into
`backtest/report.md`). Nothing is applied: look at `git diff backtest/`,
then run `dify-review backtest --apply` and commit.

A 👎 on a bot comment, or a reply saying it does not apply, marks it wrong;
👍, resolving the thread or changing the commented lines marks it right.

## Reusing the binary

Every push to main publishes `dify-review-x86_64-linux` to the `rolling`
release:

```sh
gh release download rolling -R OWNER/dify-review -p 'dify-review-x86_64-linux*'
sha256sum -c dify-review-x86_64-linux.sha256 && chmod +x dify-review-x86_64-linux
```
