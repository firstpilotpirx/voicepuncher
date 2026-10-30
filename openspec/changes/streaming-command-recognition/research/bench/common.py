import re
import numpy as np
import soundfile as sf

KEYWORDS = ["restart", "start", "stop", "cancel", "enter", "clear", "paste"]
# Russian spellings whisper writes for the user's English command words (auto language).
RU = {"рестарт": "restart", "старт": "start", "стоп": "stop", "энтер": "enter", "ентер": "enter", "интер": "enter",
      "интер": "enter", "клир": "clear", "клиа": "clear", "кенсел": "cancel", "кэнсел": "cancel", "кансел": "cancel",
      "пейст": "paste", "пэйст": "paste"}
PHRASE_KW = {"start dictation": "start", "stop dictation": "stop", "cancel dictation": "cancel",
             "restart dictation": "restart", "press enter": "enter", "clear all text": "clear",
             "paste text": "paste"}


def keywords(text):
    """Command words in `text`, in order: hyphens and glued words split ("enter-enter", "enterenter")."""
    t = text.lower()
    for ru, en in RU.items():
        t = t.replace(ru, f" {en} ")
    t = re.sub(r"[^a-z]+", " ", t)
    out = []
    for w in t.split():
        while w:
            for k in KEYWORDS:
                if w.startswith(k) and (w == k or any(w[len(k):].startswith(k2) for k2 in KEYWORDS)):
                    out.append(k)
                    w = w[len(k):]
                    break
            else:
                break
    return out


def collapse(seq):
    """Doubled words count once: "stop stop" is one stop."""
    out = []
    for k in seq:
        if not out or out[-1] != k:
            out.append(k)
    return out


def load(path, start=None, end=None):
    a, sr = sf.read(path, dtype="float32")
    if a.ndim > 1:
        a = a.mean(axis=1)
    assert sr == 16000, path
    if start is not None:
        a = a[max(0, int(start * sr)):int(end * sr)]
    return a


# How Russian models spell the user's English command words, glued or apart.
RU_CMD = [("restart", r"р[еи]стар?т?"), ("start", r"стар?т|стар"), ("stop", r"ст[оа]п"),
          ("enter", r"[эеи]н[тд][еэоа]?р"), ("clear", r"кл[иеэ][иеэа]?р"), ("cancel", r"к[эеа]н?с[еэ]?л"),
          ("paste", r"п[еэ]й?ст")]
RU_FILL = r"(дикт\w*|диктейш\w*|плиз|плис|плейс|пресс?|т[еэ]кс?т|ол+|прав)"


def keywords_ru(text):
    """Command words in Cyrillic `text`: a word counts only when nothing but commands and fillers is left in it."""
    out = []
    for w in re.sub(r"[^а-яё]+", " ", text.lower()).split():
        found, rest = [], w
        while rest:
            for k, p in RU_CMD:
                m = re.match(p, rest)
                if m:
                    found.append(k); rest = rest[m.end():]; break
            else:
                m = re.match(RU_FILL, rest)
                if not m:
                    break
                rest = rest[m.end():]
        if not rest:
            out += found
    return out
