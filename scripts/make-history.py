#!/usr/bin/env python3
"""A big history to measure michi on: PRs merged into main, four commits
each, every branch started from main two merges back (so lanes overlap), a
tag every 1000 merges and one branch left open.

    git init -q -b main big && cd big
    python ../michi/scripts/make-history.py 20000 | git fast-import --quiet --done
    git update-ref -d refs/heads/tmp && git reset -q --hard main

20000 PRs make 100001 commits. `git commit-graph write --reachable` then
gives the repository the cache `git gc` would write. The measuring is the
ignored test `big_history` in crates/michi/src/graph.rs.
"""

import sys

out = sys.stdout.buffer
AUTHORS = [
    "Alice <alice@example.com>",
    "Bob <bob@example.com>",
    "Carol <carol@example.com>",
    "dependabot[bot] <bot@example.com>",
    "Dave <dave@example.com>",
]
mark = 0
when = 1600000000


def put(text):
    out.write(text.encode())


def data(text):
    b = text.encode()
    put("data %d\n" % len(b))
    out.write(b)
    put("\n")


def commit(ref, message, parents, path, n):
    global mark, when
    mark += 1
    when += 60
    who = AUTHORS[n % len(AUTHORS)]
    put("commit %s\nmark :%d\n" % (ref, mark))
    put("author %s %d +0900\ncommitter %s %d +0900\n" % (who, when, who, when))
    data(message)
    if parents:
        put("from :%d\n" % parents[0])
        for p in parents[1:]:
            put("merge :%d\n" % p)
    put("M 644 inline %s\n" % path)
    data("%d\n" % mark)
    return mark


def main():
    prs = int(sys.argv[1]) if len(sys.argv) > 1 else 20000
    trunk = [commit("refs/heads/main", "Initial commit", [], "README", 0)]
    for i in range(prs):
        tip = trunk[max(0, len(trunk) - 3)]
        for k in range(4):
            tip = commit("refs/heads/tmp", "Part %d step %d" % (i, k), [tip], "f%d.txt" % (i % 50), i + k)
        m = commit("refs/heads/main", "Merge pull request #%d from feat/part-%d" % (i + 1, i), [trunk[-1], tip], "merged.txt", i)
        trunk.append(m)
        if i % 1000 == 999:
            put("reset refs/tags/v0.%d.0\nfrom :%d\n\n" % (i // 1000, m))
        if i == prs - 2:
            put("reset refs/heads/feat/open\nfrom :%d\n\n" % tip)
    put("done\n")


main()
