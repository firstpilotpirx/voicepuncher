"""Scores single models with both readings and English+Russian pairs: a pair takes the English command when it
heard one, else the Russian one."""
import json, re, sys
from common import keywords, keywords_ru, collapse

ALIASES = [(r"\bstar(s|ts)? dictation", "start dictation"), (r"\bstuck dictation", "start dictation"),
           (r"\bpresenter\b", "press enter"), (r"\bcapsule dictation", "cancel dictation"),
           (r"\bstar\b", "start")]


def with_aliases(text):
    t = text.lower()
    for a, b in ALIASES:
        t = re.sub(a, b, t)
    return collapse(keywords(t))

data = {json.loads(l)["id"]: json.loads(l) for l in open("dataset.jsonl")}


def load(name):
    return {json.loads(l)["id"]: json.loads(l) for l in open(f"results/{name}.jsonl")}


def pred_both(text):
    latin = with_aliases(text)
    return latin if latin else collapse(keywords_ru(text))


def score(name, pred_of, emitted_of=None):
    ok = n = fa = neg = 0
    for i, it in data.items():
        if it["source"] == "history" and not it["confident"]:
            continue
        p = pred_of(i)
        if not it["label"]:
            neg += 1; fa += bool(p)
        else:
            n += 1; ok += p == it["label"]
    print(f"{name:42} commands {ok / n:5.0%}   false alarms {fa / neg:5.0%}")


singles = sys.argv[1].split(",")
pairs = [p.split("+") for p in sys.argv[2].split(",")] if len(sys.argv) > 2 else []
R = {n: load(n) for n in set(singles) | {x for p in pairs for x in p}}
for n in singles:
    score(n, lambda i, n=n: pred_both(R[n][i]["text"]))
for en, ru in pairs:
    score(f"{en} + {ru}", lambda i, en=en, ru=ru: with_aliases(R[en][i]["text"]) or pred_both(R[ru][i]["text"]))
