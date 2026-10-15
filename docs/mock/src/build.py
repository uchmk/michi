"""Put the michi mock together: one HTML file for the repo, one for the Artifact.

Run `python build.py` here. data.json comes from `python gen_data.py data.json`
(the real history of a tsumugi clone); the made-up sessions are in fake.py.
"""
import json
import os
import sys

HERE = os.path.dirname(os.path.abspath(__file__))
sys.path.insert(0, HERE)
from fake import FAKE  # noqa: E402

REPO_OUT = os.path.join(HERE, "..", "michi.html")
ART_OUT = os.path.join(HERE, "michi-artifact.html")
TITLE = "michi 画面イメージ"
# The mock draws its own themes; keep Dark Reader from recoloring them.
LOCK = "<meta name=\"darkreader-lock\">\n"


def read(name):
    with open(os.path.join(HERE, name), encoding="utf-8", newline="") as f:
        return f.read()


def js_json(obj):
    return json.dumps(obj, ensure_ascii=False, separators=(",", ":")).replace("</", "<\\/")


css = read("style.css")
body = read("body.html")
app = read("app.js")
data = json.loads(read("data.json"))
script = f"const DATA={js_json(data)};\nconst FAKE={js_json(FAKE)};\n{app}"

inner = f"<style>\n{css}</style>\n{body}<script>\n{script}</script>\n"

page = (
    "<!doctype html>\n<html lang=\"ja\">\n<head>\n<meta charset=\"utf-8\">\n"
    "<meta name=\"viewport\" content=\"width=device-width, initial-scale=1, viewport-fit=cover\">\n"
    f"<title>{TITLE}</title>\n{LOCK}</head>\n<body>\n{inner}</body>\n</html>\n"
)
art = (
    f"<title>{TITLE}</title>\n<meta name=\"claude:viewport-fit-sides\" content=\"cover\">\n{LOCK}{inner}"
)

os.makedirs(os.path.dirname(REPO_OUT), exist_ok=True)
for path, text in ((REPO_OUT, page), (ART_OUT, art)):
    with open(path, "w", encoding="utf-8", newline="\n") as f:
        f.write(text)
    print(f"{path}: {len(text.encode('utf-8')) / 1024:.1f} KB")

with open(os.path.join(HERE, "check.js"), "w", encoding="utf-8", newline="\n") as f:
    f.write(script)
