"""One-off: write backtest/labels.toml from the v1 hand clustering.

Usage: python3 -I scripts/labels_v1.py data/mine.jsonl backtest/labels.toml
The integer ids are 0-based lines of data/mine.jsonl (see distill_v1.py).
Kept as a record; edit backtest/labels.toml directly from now on.
"""
import json
import sys

threads = [json.loads(line) for line in open(sys.argv[1])]

LABELS = {
    ("no-type-ignore", None): [0, 7, 9, 10, 23, 26, 29, 35, 41, 42, 43, 44, 52, 53, 56, 24],
    ("no-test-for-type-errors", None): [1, 5, 15, 32, 33, 34, 40, 51, 54, 59, 73, 74, 91, 92],
    ("tests-real-dependencies", "import-real-class"): [3, 4, 25, 82, 85, 86, 87, 98, 132],
    ("tests-real-dependencies", "use-sqlite-session"): [21, 38, 39, 47, 49, 50, 63, 64, 66, 75, 81, 6, 57, 121, 126, 127, 133, 145, 146, 61, 28],
    ("tests-real-dependencies", "inject-dependency"): [65, 67, 122, 163, 164, 165],
    ("avoid-cast", None): [46, 88, 89, 94, 95, 99, 100, 101, 103, 108, 128],
    ("no-reflection", None): [14, 22, 69, 70, 78, 79, 80, 105, 123, 131],
    ("inject-session", None): [11, 124, 125, 130, 172, 173, 174, 175],
    ("precise-types", "concrete-type"): [27, 30, 31, 36, 37, 109, 195],
    ("precise-types", "paramspec"): [90, 150, 151],
    ("precise-types", "explicit-params"): [83],
    ("model-nested-data", None): [19, 170, 119, 120],
    ("no-bypass-init", None): [58, 98],
    ("unrelated-change", None): [12, 13, 97, 167, 171, 147],
}

out = [
    "# Which earlier review comment is an instance of which rule (and, for a",
    "# choice rule, which option). Positives for `dify-review backtest`.",
    "# Generated once by scripts/labels_v1.py; edit by hand from now on.",
]
for (rule, option), ids in LABELS.items():
    for i in ids:
        url = threads[i]["comments"][0]["url"]
        out += ["", "[[label]]", f"comment = '{url}'", f"rule = '{rule}'"]
        if option:
            out.append(f"option = '{option}'")
open(sys.argv[2], "w").write("\n".join(out) + "\n")
print(sum(len(v) for v in LABELS.values()), "labels")
