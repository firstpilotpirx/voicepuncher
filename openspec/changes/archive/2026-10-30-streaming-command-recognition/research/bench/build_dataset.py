"""Builds dataset.jsonl: the user's own command recordings with the command words said in each."""
import json, os, subprocess
from concurrent.futures import ThreadPoolExecutor
from common import keywords, collapse, PHRASE_KW

HOME = os.path.expanduser("~/.voicepuncher")
VS = f"{HOME}/voice-samples"
CH = f"{HOME}/command-history"
TURBO = [18082, 18083, 18084]


def turbo(path, i, lang):
    out = subprocess.run(["curl", "-s", f"http://127.0.0.1:{TURBO[i % 3]}/v1/audio/transcriptions",
                          "-F", f"file=@{path}", "-F", "response_format=json", "-F", "temperature=0",
                          "-F", f"language={lang}"], capture_output=True, text=True).stdout
    return json.loads(out)["text"].strip()


items = []
for l in open(f"{VS}/session-1/manifest.jsonl"):
    r = json.loads(l)
    items.append({"id": f"vs1/{r['file']}", "path": f"{VS}/session-1/{r['file']}", "source": "voice-samples",
                  "label": [PHRASE_KW[r["command"]]], "said": r["said"], "word_end": r["end"]})
for s in ["session-2", "session-3", "session-4", "session-5"]:
    for n, l in enumerate(open(f"{VS}/{s}/forms.jsonl")):
        r = json.loads(l)
        st = max(0.0, r["start"] - 0.6)
        items.append({"id": f"{s}/{n}", "path": f"{VS}/{s}.wav", "cut": [st, r["end"] + 0.6],
                      "source": "voice-samples", "label": collapse(keywords(r["said"])), "said": r["said"],
                      "word_end": r["end"] - st})

hist = sorted(d for d in os.listdir(CH) if os.path.exists(f"{CH}/{d}/entry.json"))


def label_history(args):
    i, d = args
    e = json.load(open(f"{CH}/{d}/entry.json"))
    p = f"{CH}/{d}/audio.wav"
    en, auto = turbo(p, i, "en"), turbo(p, i, "auto")
    small = e.get("transcript") or ""
    k_en, k_auto, k_small = (collapse(keywords(t)) for t in (en, auto, small))
    agree = k_en == k_small or k_en == k_auto
    ends = [r["end"] for r in e.get("reported", [])]
    return {"id": f"history/{d}", "path": p, "source": "history", "label": k_en, "confident": agree,
            "turbo_en": en, "turbo_auto": auto, "small": small, "small_kw": k_small, "auto_kw": k_auto,
            "outcome": e["outcome"], "word_end": max(ends) if ends else None}


with ThreadPoolExecutor(3) as ex:
    items += list(ex.map(label_history, enumerate(hist)))

with open("dataset.jsonl", "w") as f:
    for it in items:
        f.write(json.dumps(it, ensure_ascii=False) + "\n")
h = [i for i in items if i["source"] == "history"]
print(len(items), "items;", len(items) - len(h), "voice samples;", len(h), "history;",
      sum(i["confident"] for i in h), "confident;", sum(bool(i["label"]) for i in h if i["confident"]), "with commands")
