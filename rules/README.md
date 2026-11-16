# Rules

One file per rule; the file name is the rule id. Every rule is judged by
Jev, as a yes/no `[noul]` question or a `[choice]` whose options each carry
their own comment (an option without `comment` means nothing to say).
`hint` is an optional regex: it places the inline comment on the first
matching added line, and is the judge when no Jev key is configured.
`also_guarded_by` names dify ast-grep guards that already fail CI on the
same thing; the inline comment adds how to fix it. `propose_guard` marks
rules deterministic enough to also become a dify guard.

Every change here is a pull request, backtested against the collected
review history before it merges.

## Better enforced by a linter

Seen in review, but a linter rule in the api/ ruff config would catch them:

- **ruff ANN001/ANN201 for api/tests** (6 threads): Type annotations on test parameters and fixtures (`monkeypatch: pytest.MonkeyPatch`, `caplog: pytest.LogCaptureFixture`, `-> None`). e.g. https://github.com/langgenius/dify/pull/41077#discussion_r3840518120
- **ruff SIM117** (4 threads): Merge nested `with` statements: `with a as b, c as d:`. e.g. https://github.com/langgenius/dify/pull/37621#discussion_r3434289234
- **ruff UP037 / prefer Self** (2 threads): No quoted annotations; use `Self` / a lambda for SQLAlchemy relationships. e.g. https://github.com/langgenius/dify/pull/45168#discussion_r4476556666
- **ruff PLC2701 (import-private-name)** (1 threads): No imports from private modules such as `httpx._models`. e.g. https://github.com/langgenius/dify/pull/45726#discussion_r4500483284
