"""One-off: wrote the first rules/<id>.toml from the hand-clustered review threads.

Kept as a record of how v1 was distilled; edit rules/*.toml directly now.
Usage: python3 -I scripts/distill_v1.py data/mine.jsonl rules
data/mine.jsonl is collect output filtered to one reviewer; the integer ids
below are line numbers in it (0-based), chosen by reading every thread.
"""
import json
import os
import sys

threads = [json.loads(line) for line in open(sys.argv[1])]


def examples(ids):
    return [threads[i]["comments"][0]["url"] for i in ids]


RULES = [
    dict(
        id="no-type-ignore", severity="should", paths=["api/**/*.py"], propose_guard=True,
        hint=r"#\s*(?:pyrefly|type)\s*:\s*ignore",
        noul=dict(
            question="Does `patch` add a type-checker suppression comment (`# pyrefly: ignore`, `# type: ignore`) instead of fixing the type?",
            comment="Please fix the type instead of suppressing it: pass the real enum member / model / class the signature declares, or type the production code. A test that needs an ignore to call something is usually testing what the type checker already rejects.",
        ),
        ids=[0, 7, 9, 10, 23, 26, 29, 35, 41, 42, 43, 44, 52, 53, 56, 24],
    ),
    dict(
        id="no-test-for-type-errors", severity="should", paths=["api/tests/**/*.py"],
        noul=dict(
            question="Does `patch` add or keep a test whose scenario is a static type error: passing a value of the wrong type (such as 123 or None where a mapping or model is declared), omitting a required argument, assigning None to a non-optional field, or instantiating an abstract class?",
            comment="This case only re-checks what the type checker already rejects; we don't need to test what tyck can find. Please remove it.",
        ),
        ids=[1, 5, 15, 32, 33, 34, 40, 51, 54, 59, 73, 74, 91, 92],
    ),
    dict(
        id="tests-real-dependencies", severity="should",
        paths=["api/tests/**/*.py", "api/providers/**/tests/**/*.py"],
        hint=r"\b(?:MagicMock|Mock|SimpleNamespace|create_autospec)\(|class _(?:Dummy|Fake|Stub)\w*|\bpatch(?:\.object)?\(|monkeypatch\.set(?:attr|item)\(|__table__\b|metadata\.create_all\(|flask_sqlalchemy",
        also_guarded_by=["no-new-mock", "no-new-stub-subclass", "no-new-session-mock"],
        choice=dict(
            question="How should the test double or patch in `patch` be removed?",
            options={
                "import-real-class": dict(
                    criteria="A Mock, MagicMock, SimpleNamespace, create_autospec or hand-written fake stands in for an object whose real class can be imported and built in a test: a model, a pydantic entity, a dataclass, a small internal service.",
                    comment="Can we import and use the real class here instead of a stand-in? Then the test checks the real behaviour, not the mock.",
                ),
                "use-sqlite-session": dict(
                    criteria="The stand-in or setup replaces a SQLAlchemy Session, engine or table setup: a mocked session, object() as a session, __table__.create, metadata.create_all, a Flask-SQLAlchemy app.",
                    comment="Please use the shared `sqlite_session` fixture with real rows (`session.add(Model(...))`) instead of a hand-built database or a mocked session.",
                ),
                "inject-dependency": dict(
                    criteria="The test patches or monkeypatches a module-level global or hidden internal dependency of dify's own code (db, current_user, current_account_with_tenant, a service method) that the production code reaches for instead of receiving as a parameter.",
                    comment="Rather than patching this in the test, change the production code to take the dependency as a parameter (pass the session / account / service in), so the test hands in a real one and needs no patch.",
                ),
                "protocol-di": dict(
                    criteria="The patch or fake replaces an external boundary: an HTTP or network client, a third-party SDK, the clock or time, randomness, the filesystem or the environment.",
                    comment="This is an external boundary: define a small `typing.Protocol` for what the code needs, have the production code accept it as a parameter, and pass a fake that implements the Protocol in the test, instead of patching or mocking it.",
                ),
                "no-double": dict(
                    criteria="`patch` does not introduce or keep a test double, a patch or a hand-built database.",
                    comment=None,
                ),
            },
        ),
        ids=[3, 4, 25, 82, 85, 86, 87, 98, 132, 21, 38, 39, 47, 49, 50, 63, 64, 66, 75, 81, 6, 57, 121,
             126, 127, 133, 145, 146, 61, 28, 65, 67, 122, 163, 164, 165],
    ),
    dict(
        id="avoid-cast", severity="should", paths=["api/**/*.py"], also_guarded_by=["no-new-cast"],
        hint=r"\bcast\(",
        noul=dict(
            question="Does `patch` add a `cast(...)` call?",
            comment="Avoid `cast`: nothing checks it. Give the producer a precise return type, take the value as a typed parameter, or narrow with `isinstance`.",
        ),
        ids=[46, 88, 89, 94, 95, 99, 100, 101, 103, 108, 128],
    ),
    dict(
        id="no-reflection", severity="should", paths=["api/**/*.py"], propose_guard=True,
        also_guarded_by=["no-new-getattr"],
        hint=r"\.__dict__\b|\bvars\(|\bsetattr\(|\bgetattr\(|def __getattr__\(",
        noul=dict(
            question="Does `patch` add reflection: reading or writing `__dict__`, `vars(...)`, `setattr`, `getattr`, or defining `__getattr__`?",
            comment="No reflection please (`__dict__`, `vars`, `setattr`, `getattr`, `__getattr__`): construct the object with its fields, or add a typed attribute or method.",
        ),
        ids=[14, 22, 69, 70, 78, 79, 80, 105, 123, 131],
    ),
    dict(
        id="inject-session", severity="should", paths=["api/controllers/**/*.py", "api/services/**/*.py"],
        propose_guard=True,
        hint=r"session\s+or\s+db\.session|session:\s*[\w\[\]. ]+\|\s*None\s*=\s*None|db\.session\(\)",
        noul=dict(
            question="Does `patch` make a database session optional with a fallback to `db.session`, or open a session inline (`db.session()`) in a controller, instead of taking the session as a required, injected parameter?",
            comment="Make `session` a required parameter and inject it (`@with_session` in controllers) instead of an optional argument that falls back to `db.session`.",
        ),
        ids=[11, 124, 125, 130, 172, 173, 174, 175],
    ),
    dict(
        id="precise-types", severity="nit", paths=["api/**/*.py"],
        hint=r"->\s*(?:object|Any)\b|:\s*Any\b",
        noul=dict(
            question="Does `patch` annotate a parameter or return value as `object` or `Any` where the concrete type is evident from the code (for example a function that returns a str or a Session)?",
            comment="Please use the precise type here (e.g. `-> str`, `-> Session`, `Callable[...]`), not `object` / `Any`.",
        ),
        ids=[27, 30, 31, 36, 37, 109, 195],
    ),
    dict(
        id="model-nested-data", severity="nit", paths=["api/**/*.py"],
        noul=dict(
            question="Does `patch` handle a nested structure as untyped dicts (dict[str, object] or dict[str, Any], chained key lookups, helpers that index into dicts) where a pydantic model, dataclass or TypedDict would describe the structure?",
            comment="Could this be modelled as a pydantic model / dataclass / TypedDict (validated once with `model_validate`) instead of passing nested dicts around?",
        ),
        ids=[19, 170, 119, 120],
    ),
    dict(
        id="no-bypass-init", severity="nit", paths=["api/**/*.py"], also_guarded_by=["no-new-constructor-bypass"],
        hint=r"\.__new__\(",
        noul=dict(
            question="Does `patch` create an object by calling `__new__` directly to skip `__init__`?",
            comment="Please construct it through `__init__` rather than bypassing it with `__new__`.",
        ),
        ids=[58, 98],
    ),
    dict(
        id="unrelated-change", severity="should", paths=["**"],
        noul=dict(
            question="Given the pull request title in `title`, is `patch` an unrelated change: a version bump, lockfile churn, corrupted or re-encoded text, or a stray file the title does not call for?",
            comment="This change looks unrelated to the PR; please leave it out.",
        ),
        ids=[12, 13, 97, 167, 171, 147],
    ),
]

LINT = [
    ("ruff ANN001/ANN201 for api/tests", "Type annotations on test parameters and fixtures (`monkeypatch: pytest.MonkeyPatch`, `caplog: pytest.LogCaptureFixture`, `-> None`).", [135, 137, 182, 188, 198, 199]),
    ("ruff SIM117", "Merge nested `with` statements: `with a as b, c as d:`.", [178, 179, 180, 181]),
    ("ruff UP037 / prefer Self", "No quoted annotations; use `Self` / a lambda for SQLAlchemy relationships.", [71, 102]),
    ("ruff PLC2701 (import-private-name)", "No imports from private modules such as `httpx._models`.", [55]),
]


def lit(value):
    """TOML literal string: no escapes at all, so regexes stay readable."""
    assert "'" not in value and "\n" not in value, value
    return f"'{value}'"


def prose(value):
    """TOML multi-line literal string for questions and comments."""
    assert "\'\'\'" not in value, value
    return f"\'\'\'\n{value}\'\'\'"


def toml_list(values):
    return "[" + ", ".join(lit(v) for v in values) + "]"


out_dir = sys.argv[2]
os.makedirs(out_dir, exist_ok=True)
for r in RULES:
    lines = [
        f"# Distilled from {len(r['ids'])} review threads; generated by scripts/distill_v1.py.",
        f"severity = {lit(r['severity'])}",
        f"paths = {toml_list(r['paths'])}",
    ]
    if "hint" in r:
        lines.append(f"hint = {lit(r['hint'])}")
    if "also_guarded_by" in r:
        lines.append(f"also_guarded_by = {toml_list(r['also_guarded_by'])}")
    if r.get("propose_guard"):
        lines.append("propose_guard = true")
    lines.append("examples = [")
    lines += [f"  {lit(u)}," for u in examples(r["ids"])]
    lines.append("]")
    if "noul" in r:
        lines += ["", "[noul]", f"question = {prose(r['noul']['question'])}", f"comment = {prose(r['noul']['comment'])}"]
    else:
        lines += ["", "[choice]", f"question = {prose(r['choice']['question'])}"]
        for name, opt in r["choice"]["options"].items():
            lines += ["", f"[choice.options.{name}]", f"criteria = {prose(opt['criteria'])}"]
            if opt["comment"] is not None:
                lines.append(f"comment = {prose(opt['comment'])}")
    with open(os.path.join(out_dir, r["id"] + ".toml"), "w") as f:
        f.write("\n".join(lines) + "\n")

readme = [
    "# Rules",
    "",
    "One file per rule; the file name is the rule id. Every rule is judged by",
    "Jev, as a yes/no `[noul]` question or a `[choice]` whose options each carry",
    "their own comment (an option without `comment` means nothing to say).",
    "`hint` is an optional regex: it places the inline comment on the first",
    "matching added line, and is the judge when no Jev key is configured.",
    "`also_guarded_by` names dify ast-grep guards that already fail CI on the",
    "same thing; the inline comment adds how to fix it. `propose_guard` marks",
    "rules deterministic enough to also become a dify guard.",
    "",
    "Every change here is a pull request, backtested against the collected",
    "review history before it merges.",
    "",
    "## Better enforced by a linter",
    "",
    "Seen in review, but a linter rule in the api/ ruff config would catch them:",
    "",
]
for name, why, ids in LINT:
    readme.append(f"- **{name}** ({len(ids)} threads): {why} e.g. {examples(ids)[0]}")
with open(os.path.join(out_dir, "README.md"), "w") as f:
    f.write("\n".join(readme) + "\n")

covered = {i for r in RULES for i in r["ids"]} | {i for _, _, ids in LINT for i in ids}
print(f"{len(RULES)} rules, {len(covered)}/{len(threads)} threads covered")
