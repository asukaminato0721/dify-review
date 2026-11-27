"""Print the numbers for a dataset snapshot as markdown tables.

Usage: python3 -I scripts/dataset_stats.py backtest/feedback.toml backtest/labels.toml

Live outcomes per rule come from `dify-review feedback` (right, wrong,
ignored, pending; live precision = right / (right + wrong), the share of
settled comments that held up). Labels per rule count the human review
comments in labels.toml, the positives the backtest calibrates on.
"""
import collections
import sys
import tomllib

feedback = tomllib.load(open(sys.argv[1], "rb")).get("feedback", [])
labels = tomllib.load(open(sys.argv[2], "rb")).get("label", [])

OUTCOMES = ["right", "wrong", "ignored", "pending"]
by_rule = collections.defaultdict(collections.Counter)
for e in feedback:
    by_rule[e["rule"]][e["outcome"]] += 1
label_count = collections.Counter(l["rule"] for l in labels)

print("| rule | labels | " + " | ".join(OUTCOMES) + " | live precision |")
print("|---|---|" + "---|" * len(OUTCOMES) + "---|")
total = collections.Counter()
for rule in sorted(set(by_rule) | set(label_count)):
    c = by_rule[rule]
    total.update(c)
    settled = c["right"] + c["wrong"]
    prec = f"{100 * c['right'] / settled:.0f}%" if settled else "-"
    cells = " | ".join(str(c[o]) for o in OUTCOMES)
    print(f"| {rule} | {label_count[rule]} | {cells} | {prec} |")
settled = total["right"] + total["wrong"]
prec = f"{100 * total['right'] / settled:.0f}%" if settled else "-"
cells = " | ".join(str(total[o]) for o in OUTCOMES)
print(f"| **all** | {sum(label_count.values())} | {cells} | {prec} |")
