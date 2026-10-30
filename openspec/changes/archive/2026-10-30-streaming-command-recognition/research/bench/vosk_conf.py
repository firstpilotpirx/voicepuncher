"""Vosk small with the command grammar, keeping each word's confidence: results/vosk-conf.jsonl."""
import json, os, numpy as np, vosk, bench
from common import load
vosk.SetLogLevel(-1)
m = vosk.Model(f"{bench.M}/vosk-model-small-en-us-0.15")
with open("results_conf.jsonl", "w") as out:
    for l in open("dataset.jsonl"):
        it = json.loads(l)
        a = load(it["path"], *it.get("cut", [None, None]))
        a = np.concatenate([a, np.zeros(int(0.8 * 16000), np.float32)])
        r = vosk.KaldiRecognizer(m, 16000, json.dumps(bench.GRAMMAR)); r.SetWords(True)
        pcm = (np.clip(a, -1, 1) * 32767).astype(np.int16).tobytes()
        words = []
        for i in range(0, len(pcm), 3200):
            if r.AcceptWaveform(pcm[i:i + 3200]):
                words += json.loads(r.Result()).get("result", [])
        words += json.loads(r.FinalResult()).get("result", [])
        out.write(json.dumps({"id": it["id"], "words": [(w["word"], w["conf"]) for w in words]}) + "\n")
