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

## Reusing the binary

Every push to main publishes `dify-review-x86_64-linux` to the `rolling`
release:

```sh
gh release download rolling -R OWNER/dify-review -p 'dify-review-x86_64-linux*'
sha256sum -c dify-review-x86_64-linux.sha256 && chmod +x dify-review-x86_64-linux
```
