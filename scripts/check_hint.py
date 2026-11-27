"""Check a rule's hint regex against the lines it should (and should not) match.

Usage: python3 -I scripts/check_hint.py REGEX LINE [LINE ...]
Prefix a LINE with `!` for one the hint must not match. Prints each line with
its result and exits non-zero if any expectation fails. Uses Python's `re`,
close enough to Rust's `regex` for the hints in rules/ (no look-around).
"""
import re
import sys

hint = re.compile(sys.argv[1])
results = [
    (line, want, bool(hint.search(text)))
    for line in sys.argv[2:]
    for want, text in [(not line.startswith("!"), line.removeprefix("!"))]
]
for line, want, got in results:
    print(f"{'ok  ' if want == got else 'FAIL'} {'match' if got else 'no match':8} {line}")
sys.exit(int(any(want != got for _, want, got in results)))
