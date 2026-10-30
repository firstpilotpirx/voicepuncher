"""Runs one candidate over dataset.jsonl: python bench.py <candidate> -> results/<candidate>.jsonl"""
import json, os, sys, time, subprocess, tempfile
import numpy as np
import soundfile as sf
from common import keywords, collapse, load

M = os.path.expanduser("~/.voicepuncher/models/research/streaming")
CHUNK = 0.1
LANG = os.environ.get("LANG_CODE", "en")
TAIL = 0.8
GRAMMAR = ["start", "stop", "cancel", "restart", "enter", "press enter", "clear", "clear all text",
           "clear text", "paste", "paste text", "start dictation", "stop dictation", "cancel dictation",
           "restart dictation", "please", "[unk]"]


def sherpa(d, enc, dec, joi, **kw):
    import sherpa_onnx
    p = f"{M}/{d}"
    return sherpa_onnx.OnlineRecognizer.from_transducer(
        tokens=f"{p}/tokens.txt", encoder=f"{p}/{enc}", decoder=f"{p}/{dec}", joiner=f"{p}/{joi}",
        num_threads=2, sample_rate=16000, feature_dim=80, **kw)


def zip_files(d, tag):
    return d, f"encoder-{tag}.int8.onnx", f"decoder-{tag}.onnx", f"joiner-{tag}.int8.onnx"


def int8(d):
    return d, "encoder.int8.onnx", "decoder.int8.onnx", "joiner.int8.onnx"


CANDIDATES = {
    "kroko": lambda: sherpa("sherpa-onnx-streaming-zipformer-en-kroko-2025-08-06", "encoder.onnx", "decoder.onnx", "joiner.onnx"),
    "zipformer-20M": lambda: sherpa(*zip_files("sherpa-onnx-streaming-zipformer-en-20M-2023-02-17", "epoch-99-avg-1")),
    "zipformer-70M": lambda: sherpa(*zip_files("sherpa-onnx-streaming-zipformer-en-2023-06-26", "epoch-99-avg-1-chunk-16-left-128")),
    "nemo-fc-80ms": lambda: sherpa(*int8("sherpa-onnx-nemo-streaming-fast-conformer-transducer-en-80ms-int8")),
    "nemo-fc-480ms": lambda: sherpa(*int8("sherpa-onnx-nemo-streaming-fast-conformer-transducer-en-480ms-int8")),
    "nemotron-0.6b-160ms": lambda: sherpa(*int8("sherpa-onnx-nemotron-speech-streaming-en-0.6b-160ms-int8-2026-04-25")),
    "parakeet-0.6b-240ms": lambda: sherpa(*int8("sherpa-onnx-nemo-parakeet-unified-en-0.6b-int8-streaming-240ms")),
}


def run_sherpa(rec, audio):
    s = rec.create_stream()
    step = int(CHUNK * 16000)
    audio = np.concatenate([audio, np.zeros(int(TAIL * 16000), np.float32)])
    compute, emitted, chunk_ms = 0.0, {}, []
    for i in range(0, len(audio), step):
        t = time.perf_counter()
        s.accept_waveform(16000, audio[i:i + step])
        while rec.is_ready(s):
            rec.decode_stream(s)
        text = rec.get_result(s)
        dt = time.perf_counter() - t
        compute += dt
        chunk_ms.append(dt * 1000)
        pos = min(i + step, len(audio)) / 16000
        for k in collapse(keywords(text)):
            emitted.setdefault(k, pos)
    s.input_finished()
    while rec.is_ready(s):
        rec.decode_stream(s)
    return rec.get_result(s), compute, emitted, chunk_ms


def vosk_factory(d, grammar):
    import vosk
    vosk.SetLogLevel(-1)
    model = vosk.Model(f"{M}/{d}")
    return lambda: vosk.KaldiRecognizer(model, 16000, json.dumps(grammar)) if grammar else vosk.KaldiRecognizer(model, 16000)


def run_vosk(make, audio):
    r = make()
    step = int(CHUNK * 16000)
    audio = np.concatenate([audio, np.zeros(int(TAIL * 16000), np.float32)])
    pcm = (np.clip(audio, -1, 1) * 32767).astype(np.int16)
    compute, emitted, chunk_ms, final = 0.0, {}, [], []
    for i in range(0, len(pcm), step):
        t = time.perf_counter()
        if r.AcceptWaveform(pcm[i:i + step].tobytes()):
            final.append(json.loads(r.Result())["text"])
            text = " ".join(final)
        else:
            text = " ".join(final + [json.loads(r.PartialResult())["partial"]])
        dt = time.perf_counter() - t
        compute += dt
        chunk_ms.append(dt * 1000)
        pos = min(i + step, len(pcm)) / 16000
        for k in collapse(keywords(text.replace("[unk]", ""))):
            emitted.setdefault(k, pos)
    final.append(json.loads(r.FinalResult())["text"])
    return " ".join(final).replace("[unk]", ""), compute, emitted, chunk_ms


def kws_factory():
    """The current keyword spotter with the service's keywords.txt; its triggers count as their commands."""
    import sherpa_onnx
    p = os.path.expanduser("~/.voicepuncher/models/sherpa-onnx-kws-zipformer-gigaspeech-3.3M-2024-01-01")
    tag = "epoch-12-avg-2-chunk-16-left-64"
    kw = "/Users/pirx/Workspaces/my/voicepuncher/command-spotting/keywords.txt"
    lines = [l for l in open(kw) if l.strip() and not l.startswith("#")]
    path = os.path.abspath("kws_keywords.txt")
    open(path, "w").write("".join(lines))
    return sherpa_onnx.KeywordSpotter(
        tokens=f"{p}/tokens.txt", encoder=f"{p}/encoder-{tag}.int8.onnx", decoder=f"{p}/decoder-{tag}.int8.onnx",
        joiner=f"{p}/joiner-{tag}.int8.onnx", keywords_file=path, num_threads=1)


def run_kws(kws, audio):
    s = kws.create_stream()
    step = int(CHUNK * 16000)
    audio = np.concatenate([audio, np.zeros(int(TAIL * 16000), np.float32)])
    compute, emitted, chunk_ms, hits = 0.0, {}, [], []
    for i in range(0, len(audio), step):
        t = time.perf_counter()
        s.accept_waveform(16000, audio[i:i + step])
        while kws.is_ready(s):
            kws.decode_stream(s)
            r = kws.get_result(s)
            if r:
                hits.append(r.replace("_", " "))
                kws.reset_stream(s)
        dt = time.perf_counter() - t
        compute += dt
        chunk_ms.append(dt * 1000)
        pos = min(i + step, len(audio)) / 16000
        for k in collapse(keywords(" ".join(hits))):
            emitted.setdefault(k, pos)
    return " ".join(hits), compute, emitted, chunk_ms


def whisper_factory(port):
    def run(audio):
        with tempfile.NamedTemporaryFile(suffix=".wav") as f:
            sf.write(f.name, audio, 16000)
            t = time.perf_counter()
            out = subprocess.run(["curl", "-s", f"http://127.0.0.1:{port}/v1/audio/transcriptions", "-F", f"file=@{f.name}",
                                  "-F", "response_format=json", "-F", "temperature=0", "-F", f"language={LANG}"],
                                 capture_output=True, text=True).stdout
            dt = time.perf_counter() - t
        return json.loads(out)["text"].strip(), dt, {}, [dt * 1000]
    return run


def main(name):
    if name in CANDIDATES:
        rec = CANDIDATES[name]()
        run = lambda a: run_sherpa(rec, a)
    elif name.startswith("vosk"):
        d = "vosk-model-small-en-us-0.15" if "small" in name else "vosk-model-en-us-0.22-lgraph"
        make = vosk_factory(d, GRAMMAR if "grammar" in name else None)
        run = lambda a: run_vosk(make, a)
    elif name == "kws-current":
        kws = kws_factory()
        run = lambda a: run_kws(kws, a)
    elif name in ("whisper-small", "whisper-small-ru"):
        run = whisper_factory(18086)
    elif name == "whisper-turbo":
        run = whisper_factory(18082)
    else:
        raise SystemExit(f"unknown {name}")
    os.makedirs("results", exist_ok=True)
    with open(f"results/{name}.jsonl", "w") as out:
        for l in open("dataset.jsonl"):
            it = json.loads(l)
            a = load(it["path"], *it.get("cut", [None, None]))
            text, compute, emitted, chunk_ms = run(a)
            out.write(json.dumps({"id": it["id"], "text": text, "pred": collapse(keywords(text)),
                                  "compute": compute, "dur": len(a) / 16000, "emitted": emitted,
                                  "chunk_p95_ms": float(np.percentile(chunk_ms, 95))}) + "\n")


if __name__ == "__main__":
    main(sys.argv[1])
