"""Scores results/*.jsonl against dataset.jsonl."""
import glob, json, os, re
import numpy as np
from common import keywords, collapse

# The project's known mishearings, applied to every candidate alike.
ALIASES = [(r"\bstar(s|ts)? dictation", "start dictation"), (r"\bstuck dictation", "start dictation"),
           (r"\bpresenter\b", "press enter"), (r"\bcapsule dictation", "cancel dictation"),
           (r"\bstar\b", "start")]


def with_aliases(text):
    t = text.lower()
    for a, b in ALIASES:
        t = re.sub(a, b, t)
    return collapse(keywords(t))


data = {json.loads(l)["id"]: json.loads(l) for l in open("dataset.jsonl")}
rows = []
for f in sorted(glob.glob("results/*.jsonl")):
    name = os.path.basename(f)[:-6]
    res = [json.loads(l) for l in open(f)]
    if len(res) < len(data):
        continue
    vs_ok = vs_n = h_ok = h_n = fa = neg = 0
    vs_ok_a = h_ok_a = fa_a = 0
    delays, rtf, p95 = [], [], []
    for r in res:
        it = data[r["id"]]
        if it["source"] == "history" and not it["confident"]:
            continue
        pa = with_aliases(r["text"])
        rtf.append(r["compute"] / r["dur"])
        p95.append(r["chunk_p95_ms"])
        if not it["label"]:
            neg += 1
            fa += bool(r["pred"])
            fa_a += bool(pa)
            continue
        ok, ok_a = r["pred"] == it["label"], pa == it["label"]
        if it["source"] == "voice-samples":
            vs_n += 1; vs_ok += ok; vs_ok_a += ok_a
        else:
            h_n += 1; h_ok += ok; h_ok_a += ok_a
        last = it["label"][-1]
        if ok and it.get("word_end") and last in r["emitted"]:
            delays.append(r["emitted"][last] - it["word_end"])
    d = np.array(delays) if delays else np.array([np.nan])
    rows.append((name, vs_ok / vs_n, vs_ok_a / vs_n, h_ok / h_n, h_ok_a / h_n, fa / neg, fa_a / neg,
                 np.nanmedian(d), np.nanpercentile(d, 90), float(np.mean(rtf)), float(np.median(p95)), vs_n, h_n, neg))

rows.sort(key=lambda r: -(r[2] * r[11] + r[4] * r[12]) / (r[11] + r[12]))
print(f"{'model':22} {'voice':>6} {'+alias':>6} {'hist':>6} {'+alias':>6} {'FA':>5} {'FA+al':>5} {'delay50':>7} {'delay90':>7} {'RTF':>6} {'chunk95ms':>9}")
for r in rows:
    print(f"{r[0]:22} {r[1]:6.0%} {r[2]:6.0%} {r[3]:6.0%} {r[4]:6.0%} {r[5]:5.0%} {r[6]:5.0%} {r[7]:7.2f} {r[8]:7.2f} {r[9]:6.3f} {r[10]:9.1f}")
if rows:
    print(f"n: voice samples {rows[0][11]}, history {rows[0][12]}, negatives {rows[0][13]}")
