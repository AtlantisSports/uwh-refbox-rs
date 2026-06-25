#!/usr/bin/env python3
"""Regenerate countdown.raw: a single short "pip" played once per second during
the final 10 seconds before a playing period. Mono, 32-bit float LE, 44,100 Hz —
same format as buzz.raw / whistle.raw (raw samples, no header). Re-run after
tweaking FREQ/DUR to retune the tone, then commit countdown.raw."""
import math, struct, os

SR = 44100
FREQ = 1000.0   # Hz — clear mid-high pip
DUR = 0.25      # seconds
FADE = 0.005    # 5 ms fade in/out to avoid clicks
# Peak ~0.99 (near full-scale) so the beep matches the buzzer clips' loudness:
# the buzzers peak ~1.0, and a full-scale sine's RMS (~0.70) matches buzz.raw's
# RMS (~0.72). Playback also uses the buzzer above/under-water volumes.
AMP = 0.99
OUT = os.path.join(os.path.dirname(os.path.abspath(__file__)), "countdown.raw")

n = int(round(DUR * SR))
fade_n = max(1, int(round(FADE * SR)))
with open(OUT, "wb") as f:
    for i in range(n):
        x = math.sin(2 * math.pi * FREQ * i / SR)
        if i < fade_n:
            x *= i / fade_n
        elif i >= n - fade_n:
            x *= (n - i) / fade_n
        f.write(struct.pack("<f", x * AMP))
print(f"wrote {OUT}: {n} samples ({DUR * 1000:.0f} ms @ {SR} Hz)")
