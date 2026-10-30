"""Writes a token-level hotwords file for a sherpa-onnx model from its tokens.txt (greedy longest match)."""
import sys
PHRASES = ["start", "stop", "cancel", "restart", "enter", "press enter", "clear", "clear all text", "clear text",
           "paste", "paste text", "start dictation", "stop dictation", "cancel dictation", "restart dictation"]

def write(model_dir, out, lower=True):
    vocab = {l.rsplit(" ", 1)[0] for l in open(f"{model_dir}/tokens.txt", encoding="utf-8")}
    lines = []
    for p in PHRASES:
        toks = []
        for w in p.split():
            w = "▁" + (w if lower else w.upper())
            while w:
                for n in range(len(w), 0, -1):
                    if w[:n] in vocab:
                        toks.append(w[:n]); w = w[n:]; break
                else:
                    raise SystemExit(f"no token for {w}")
        lines.append(" ".join(toks))
    open(out, "w", encoding="utf-8").write("\n".join(lines) + "\n")
    return lines

if __name__ == "__main__":
    print("\n".join(write(sys.argv[1], sys.argv[2])))
