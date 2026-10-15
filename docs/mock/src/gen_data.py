"""Collect real tsumugi history for the michi mock (writes data.json)."""
import json
import os
import subprocess
import sys

REPO = os.environ.get("TSUMUGI_REPO", r"C:\dev\tsumugi")
N = 130          # commits in the graph
DIFF_TOP = 40    # commits that carry real diffs
FILE_CAP = 70    # diff lines kept per file
FILES_CAP = 6    # files with diffs per commit


def git(*args):
    out = subprocess.run(["git", "-C", REPO, *args], capture_output=True)
    return out.stdout.decode("utf-8", "replace")


def short_author(a):
    return a.split("\\")[-1]


log = git("log", "origin/main", "--date-order", f"-n{N}", "--format=%h%x1f%p%x1f%an%x1f%at%x1f%D%x1f%s%x1f%b%x1e")
commits = []
for rec in log.split("\x1e"):
    rec = rec.strip("\n")
    if not rec:
        continue
    h, p, an, at, d, s, b = rec.split("\x1f")
    refs = [r.strip() for r in d.split(",") if r.strip()]
    body = b.strip().split("\n\n")[0].replace("\n", " ").strip()
    if body.startswith("Co-Authored-By"):
        body = ""
    commits.append({
        "h": h,
        "p": p.split() if p else [],
        "a": short_author(an),
        "t": int(at),
        "s": s,
        "b": body[:400],
        "r": refs,
    })

known = {c["h"] for c in commits}
for i, c in enumerate(commits):
    files = []
    if c["p"]:
        ns = git("diff", "--numstat", c["p"][0], c["h"])
        for line in ns.splitlines():
            add, dele, path = line.split("\t", 2)
            files.append([path, -1 if add == "-" else int(add), -1 if dele == "-" else int(dele)])
    c["f"] = files
    if i < DIFF_TOP and c["p"]:
        diffs = {}
        for path, add, dele in files[:FILES_CAP]:
            if add < 0:
                continue
            d = git("diff", "-U2", "--no-color", c["p"][0], c["h"], "--", path)
            lines = d.splitlines()
            # Drop the header (diff --git, index, ---, +++).
            while lines and not lines[0].startswith("@@"):
                lines.pop(0)
            if len(lines) > FILE_CAP:
                lines = lines[:FILE_CAP] + ["\x00cut"]
            diffs[path] = lines
        c["d"] = diffs
    # Parents outside the window are cut (the graph shows a fading stub).
    c["p"] = [x for x in c["p"] if x in known] or ([] if not c["p"] else ["\x00out"])

remotes = [r.strip() for r in git("branch", "-r", "--format=%(refname:short)").splitlines() if r.strip() and "HEAD" not in r]
locals_ = [r.strip() for r in git("branch", "--format=%(refname:short)").splitlines() if r.strip()]
tags = [t for t in git("tag", "--sort=-creatordate").splitlines() if t.strip()]

json.dump({"commits": commits, "remotes": remotes, "locals": locals_, "tags": tags}, open(sys.argv[1], "w", encoding="utf-8", newline="\n"), ensure_ascii=False, separators=(",", ":"))
print(len(commits), "commits;", sum(1 for c in commits if "d" in c), "with diffs;", len(remotes), "remotes;", len(locals_), "locals")
