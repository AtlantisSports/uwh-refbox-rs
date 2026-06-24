#!/usr/bin/env python3
"""Regenerate the 7 synthesized buzzer loop-element .raw files.

Each file is a single-cycle loop element (mono, 32-bit float LE, 44,100 Hz) —
the same format/role as buzz.raw etc. The app loops the element to fill the
auto-buzzer window and the held alarm. Elements are designed to loop with an
even rhythm / continuous phase so the repeat seam is imperceptible, and to land
near the ~2.15s auto window (3 cycles for most). See the design spec.
"""
import numpy as np, os
SR = 44100
OUT = os.path.dirname(os.path.abspath(__file__))

def wave_at(freq, dur, kind="square"):
    n = int(round(dur * SR)); t = np.arange(n) / SR; ph = 2 * np.pi * freq * t
    if kind == "sine":   return np.sin(ph)
    if kind == "square": return np.sign(np.sin(ph))
    return 2 * (t * freq - np.floor(0.5 + t * freq))  # saw

def glide(farr, kind="sine"):
    ph = 2 * np.pi * np.concatenate(([0.0], np.cumsum(farr)[:-1])) / SR
    nxt = ph[-1] + 2 * np.pi * farr[-1] / SR
    k = max(1, round(nxt / (2 * np.pi)))
    ph = ph * (2 * np.pi * k / nxt)
    if kind == "saw": return 2 * ((ph / (2 * np.pi)) % 1.0) - 1.0
    return np.sin(ph)

def edge(x, ms=3):
    k = int(SR * ms / 1000)
    if k < 1 or k * 2 >= len(x): return x
    r = 0.5 * (1 - np.cos(np.linspace(0, np.pi, k))); x = x.copy()
    x[:k] *= r; x[-k:] *= r[::-1]; return x

def sil(dur): return np.zeros(int(round(dur * SR)))
def cat(*p): return np.concatenate(p)
def norm(x):
    p = np.max(np.abs(x)); return x / p * 0.95 if p > 0 else x

def e_airhorn():
    def honk(d):
        return edge(norm(wave_at(215, d, "saw") + wave_at(286, d, "saw")
                         + 0.4 * wave_at(107, d, "saw") + 0.5 * wave_at(218, d, "saw")), 10)
    return cat(honk(0.50), sil(0.20))                                    # 0.70s

def e_pipes():
    def clang(f0, d):
        n = int(round(d * SR)); t = np.arange(n) / SR
        ratios = [1.0, 2.76, 5.40, 8.93, 11.34]; amps = [1.0, 0.6, 0.35, 0.2, 0.12]
        x = sum(a * np.sin(2 * np.pi * f0 * r * t) for r, a in zip(ratios, amps))
        env = np.exp(-t / (d * 0.32)); k = int(0.0015 * SR); env[:k] *= np.linspace(0, 1, k)
        x = x * env; kf = int(0.003 * SR); x[-kf:] *= np.linspace(1, 0, kf); return x
    return norm(clang(470, 0.215))                                       # 0.215s

def e_klaxon():
    n = int(round(0.58 * SR)); h = n // 2
    farr = np.concatenate([np.linspace(300, 520, h), np.linspace(520, 300, n - h)])
    return cat(edge(glide(farr, "saw"), 8), sil(0.12))                   # 0.70s

def e_pip():
    return cat(edge(wave_at(1700, 0.07, "square"), 2), sil(0.07))        # 0.14s

def e_pulse():
    return cat(edge(wave_at(330, 0.42, "square"), 4), sil(0.28))         # 0.70s

def e_siren():
    n = int(round(0.70 * SR)); h = n // 2
    farr = np.concatenate([np.linspace(500, 1500, h), np.linspace(1500, 500, n - h)])
    return glide(farr, "sine")                                           # 0.70s

def e_trill():
    per = int(round(0.0625 * SR))
    farr = np.concatenate([np.full(per, f) for f in [1000, 1300] * 4])
    return glide(farr, "sine")                                           # 0.50s

ELEMENTS = {"airhorn": e_airhorn, "pipes": e_pipes, "klaxon": e_klaxon,
            "pip": e_pip, "pulse": e_pulse, "siren": e_siren, "trill": e_trill}
for name, fn in ELEMENTS.items():
    norm(fn()).astype("<f4").tofile(os.path.join(OUT, name + ".raw"))
    print("wrote", name + ".raw")
