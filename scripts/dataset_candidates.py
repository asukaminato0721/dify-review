"""List review threads still to read for the next dataset snapshot.

Usage: python3 -I scripts/dataset_candidates.py data/threads.jsonl dataset/reviewed.txt LOGIN [OUT.jsonl]

A candidate is a thread LOGIN started as a human reviewer (`human_review`)
whose root comment URL is not yet in dataset/reviewed.txt: every thread read
for a snapshot, labelled or left out, goes there, so nothing is read twice.
Prints each candidate compactly (index, URL, path, hunk tail, first comments)
and, with OUT, writes them as JSON lines in the same order.
"""
import json
import sys

threads_path, reviewed_path, login = sys.argv[1:4]
out_path = sys.argv[4] if len(sys.argv) > 4 else None

reviewed = {line.strip() for line in open(reviewed_path) if line.strip() and not line.startswith("#")}
threads = [json.loads(line) for line in open(threads_path) if line.strip()]
candidates = [
    t
    for t in threads
    if t["flags"].get("human_review")
    and t["comments"][0]["author"] == login
    and t["comments"][0]["url"] not in reviewed
]
candidates.sort(key=lambda t: t["comments"][0]["created_at"])

for i, t in enumerate(candidates):
    root = t["comments"][0]
    print(f"#{i} {root['url']}  {root['created_at'][:10]}")
    print(f"  {t['path']}  acted_on={t['flags']['acted_on']} replied={t['flags']['author_replied']}")
    for line in t["diff_hunk"].splitlines()[-6:]:
        print("   |", line[:140])
    for c in t["comments"][:3]:
        print("  >", c["author"] + ":", c["body"][:400].replace("\n", " / "))
print(f"\n{len(candidates)} candidate(s)", file=sys.stderr)

if out_path:
    with open(out_path, "w") as f:
        f.writelines(json.dumps(t) + "\n" for t in candidates)
