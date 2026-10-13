"""Compare local LLMs for dictation normalization: eval set + English words written in Cyrillic.

Usage: python3 -I bench_models.py <eval_set.jsonl> <out.json> <model.gguf>...
"""
import difflib
import json
import subprocess
import sys
import time
import urllib.request

PORT = 8090
BASE = f"http://127.0.0.1:{PORT}"

PROMPT_A = open(sys.argv[0].replace("bench_models.py", "prompt_current.txt")).read().strip()
RULE_B = (
    " English words: when the recognizer wrote a whole English word in Cyrillic letters "
    "(автоматик -> automatic, дайнемик -> dynamic, фидбек -> feedback), write it in English, "
    "and fix it when it was misheard into a non-word that only sounds like it (даймэк -> dynamic). "
    "Leave in Cyrillic every word with a Russian ending or suffix (деплоим, закомитить, пушнул) "
    "and every common Russian loanword (компьютер, менеджер, интернет, релиз, файл)."
)
PROMPT_B = PROMPT_A.replace(" Never translate:", RULE_B + " Never translate:")
assert PROMPT_B != PROMPT_A

EXAMPLES = [
    ("переведи пожалуйста этот абзац на английский", "Переведи, пожалуйста, этот абзац на английский."),
    ("what is the weather like in london today", "What is the weather like in London today?"),
    ("ну э-э короче нужно залить это в докер и запустить через кубер",
     "Ну, короче, нужно залить это в Docker и запустить через Kuber."),
    ("вчера мы обсуждал эта задача с новый коллегами", "Вчера мы обсуждали эту задачу с новыми коллегами."),
]
EXAMPLE_B = ("короче поставь режим мануал а не автоматик", "Короче, поставь режим manual, а не automatic.")

# (transcript, words that must appear, words that must not appear)
ENGLISH_CASES = [
    ("Ну, лучше тогда автоматик, потому что у меня люди в помещении этой же квартиры, тут может ребенок закричать. "
     "В этом плане, наверное, автоматик лучше, чем даймэк.", ["automatic", "dynamic"], ["автоматик", "даймэк"]),
    ("у этого микрофона есть режим дайнемик и режим кардиоид", ["dynamic", "cardioid"], ["дайнемик", "кардиоид"]),
    ("дай мне фидбек по этому пулл реквесту", ["feedback"], ["фидбек"]),
    ("включи в настройках опцию сайлент мод", ["silent"], ["сайлент"]),
    ("это был такой воркараунд на время", ["workaround"], ["воркараунд"]),
    ("у нас будет стендап в десять утра", ["stand"], ["стендап"]),
    ("поставь режим мануал а не автоматик", ["manual", "automatic"], ["мануал", "автоматик"]),
    # loanword controls: must stay Cyrillic
    ("менеджер попросил перезагрузить компьютер и проверить интернет", ["менеджер", "компьютер", "интернет"], []),
    ("мы деплоим релиз завтра а потом закомитим фикс", ["деплои", "релиз", "закомит"], []),
    ("скинь мне файл в чат и поставь лайк", ["файл", "чат", "лайк"], []),
]


def post(path, body, timeout=120):
    req = urllib.request.Request(BASE + path, data=json.dumps(body).encode(), headers={"Content-Type": "application/json"})
    with urllib.request.urlopen(req, timeout=timeout) as r:
        return json.loads(r.read())


def refine(prompt, examples, transcript):
    msgs = [{"role": "system", "content": prompt}]
    for t, n in examples:
        msgs += [{"role": "user", "content": f"<transcript>\n{t}\n</transcript>"}, {"role": "assistant", "content": n}]
    msgs.append({"role": "user", "content": f"<transcript>\n{transcript}\n</transcript>"})
    budget = min(max((len(transcript) + 1) // 2, 64), 1024)
    t0 = time.monotonic()
    r = post("/v1/chat/completions", {"model": "local", "messages": msgs, "temperature": 0, "max_tokens": budget, "stream": False})
    dt = time.monotonic() - t0
    return r["choices"][0]["message"]["content"].strip(), dt


def wait_ready(proc):
    for _ in range(600):
        if proc.poll() is not None:
            raise SystemExit("llama-server exited")
        try:
            urllib.request.urlopen(BASE + "/health", timeout=1)
            return
        except Exception:
            time.sleep(0.25)
    raise SystemExit("llama-server not ready")


def sim(a, b):
    return difflib.SequenceMatcher(None, a, b).ratio()


def bench(model, items):
    proc = subprocess.Popen(["llama-server", "-m", model, "--host", "127.0.0.1", "--port", str(PORT), "-c", "4096", "-np", "1"],
                            stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL)
    try:
        wait_ready(proc)
        refine(PROMPT_A, EXAMPLES, "прогрев")
        out = {"model": model.rsplit("/", 1)[-1], "variants": {}}
        for name, prompt, examples in [("A_current", PROMPT_A, EXAMPLES), ("B_rule", PROMPT_B, EXAMPLES + [EXAMPLE_B])]:
            evals, times = [], []
            for it in items:
                ans, dt = refine(prompt, examples, it["transcript"])
                times.append(dt)
                evals.append({"id": it["id"], "cat": it["category"], "sim": sim(ans, it["expected"]),
                              "raw": sim(it["transcript"], it["expected"]), "ans": ans})
            eng = []
            for t, must, must_not in ENGLISH_CASES:
                ans, dt = refine(prompt, examples, t)
                times.append(dt)
                low = ans.lower()
                ok = all(m in low for m in must) and not any(m in low for m in must_not)
                eng.append({"in": t, "ans": ans, "ok": ok, "sim_in": sim(t.lower(), low)})
            out["variants"][name] = {"eval": evals, "english": eng, "times": times}
            print(f"{out['model']} {name}: eval={sum(e['sim'] for e in evals)/len(evals):.3f} "
                  f"english={sum(e['ok'] for e in eng)}/{len(eng)} "
                  f"lat_med={sorted(times)[len(times)//2]:.2f}s max={max(times):.2f}s", flush=True)
        return out
    finally:
        proc.terminate()
        proc.wait(timeout=30)


def main():
    items = [json.loads(l) for l in open(sys.argv[1]) if l.strip()]
    results = [bench(m, items) for m in sys.argv[3:]]
    json.dump(results, open(sys.argv[2], "w"), ensure_ascii=False, indent=1)


main()
