from pathlib import Path
import json
import subprocess
import wave

import numpy as np


RATE = 48000
DURATION = 72
ROOT = Path(__file__).resolve().parents[1]
OUT = ROOT / "public" / "audio"
OUT.mkdir(parents=True, exist_ok=True)
LOCAL = ROOT.parent / ".local" / "promo-video" / "audio"
LOCAL.mkdir(parents=True, exist_ok=True)
rng = np.random.default_rng(731)
mix = np.zeros((RATE * DURATION, 2), dtype=np.float64)


def add(signal, start, gain=1, pan=0):
    offset = round(start * RATE)
    length = min(len(signal), len(mix) - offset)
    if length > 0:
        stereo = np.array([np.cos((pan + 1) * np.pi / 4), np.sin((pan + 1) * np.pi / 4)])
        mix[offset:offset + length] += signal[:length, None] * stereo * gain


def tone(note, duration, kind="pluck"):
    t = np.arange(round(duration * RATE)) / RATE
    frequency = 440 * 2 ** ((note - 69) / 12)
    if kind == "pad":
        signal = sum(np.sin(2 * np.pi * frequency * detune * t + phase) for detune, phase in [(0.998, 0.3), (1.002, 1.1), (1, 2.4)]) / 3
        signal += 0.10 * np.sin(4 * np.pi * frequency * t)
        envelope = np.minimum(t / 1.2, 1) * np.minimum((duration - t) / 1.8, 1)
    elif kind == "bass":
        signal = np.sin(2 * np.pi * frequency * t) + 0.18 * np.sin(4 * np.pi * frequency * t)
        envelope = (1 - np.exp(-t * 65)) * np.exp(-t * 2.2) * np.minimum((duration - t) / 0.08, 1)
    else:
        signal = np.sin(2 * np.pi * frequency * t) + 0.22 * np.sin(4 * np.pi * frequency * t) * np.exp(-t * 9)
        signal += 0.07 * np.sin(6 * np.pi * frequency * t) * np.exp(-t * 14)
        envelope = (1 - np.exp(-t * 180)) * np.exp(-t * 3.3) * np.minimum((duration - t) / 0.10, 1)
    return signal * envelope


def kick():
    t = np.arange(round(0.35 * RATE)) / RATE
    phase = 2 * np.pi * (44 * t + 64 * (1 - np.exp(-t * 32)) / 32)
    return np.sin(phase) * (1 - np.exp(-t * 900)) * np.exp(-t * 16)


def percussion(duration, decay, bright):
    t = np.arange(round(duration * RATE)) / RATE
    noise = rng.normal(0, 1, len(t))
    smooth = np.convolve(noise, np.ones(7) / 7, mode="same")
    sound = noise - smooth if bright else smooth
    return sound * np.exp(-t * decay) * (1 - np.exp(-t * 1500))


chords = [(50, 57, 60, 64, 69), (46, 53, 57, 60, 65), (53, 57, 60, 64, 67), (48, 55, 60, 62, 67)]
for start in np.arange(0, 59, 6.4):
    chord = chords[int(round(start / 6.4)) % 4]
    gain = 0.038 if start < 8 else 0.032
    for index, note in enumerate(chord):
        add(tone(note, min(7.8, 62.2 - start), "pad"), start, gain, (index - 2) * 0.32)

for start in (4, 68.2):
    for index, note in enumerate((62, 69, 72, 76)):
        add(tone(note, 4.5), start + index * 0.12, 0.12, (index - 1.5) * 0.25)
for index, note in enumerate((50, 57, 60, 64, 69)):
    add(tone(note, 5.4, "pad"), 68.2, 0.042, (index - 2) * 0.35)

for step in range(20, 147):
    start = step * 0.4
    chord = chords[int(start // 6.4) % 4]
    active = 0.7 if 41 <= start < 45 or 54 <= start < 55 else 1
    if step % 2 == 0:
        add(tone(chord[0] - 12, 0.75, "bass"), start, 0.16 * active)
    if start >= 8:
        if step % 4 in (0, 2):
            add(kick(), start, 0.22 * active)
        if step % 4 == 2:
            add(percussion(0.16, 32, False), start, 0.16 * active, 0.08)
        add(percussion(0.055, 75, True), start + 0.2, 0.019 * active, -0.28 if step % 2 else 0.28)
    if start >= 14 and not 41 <= start < 43:
        pattern = (1, 3, 2, 4, 1, 2, 3, 2)
        note = chord[pattern[step % 8]] + 12
        if step % 8 not in (3, 7):
            add(tone(note, 1.25), start + 0.025, 0.041 * active, np.sin(step * 0.7) * 0.55)

for phrase in (20, 28, 36, 46, 54):
    for offset, note, strength in ((0, 81, 1), (0.6, 79, 0.8), (1.2, 76, 0.9), (2.4, 74, 0.65)):
        add(tone(note, 2.5), phrase + offset, 0.048 * strength, -0.18)

for start in (2.2, 6.2, 25.2, 57.2):
    duration = 1.8
    t = np.arange(round(duration * RATE)) / RATE
    air = np.convolve(rng.normal(0, 1, len(t)), np.ones(90) / 90, mode="same")
    add(air * np.sin(np.pi * t / duration) ** 2, start, 0.10, 0.35)

for index in range(8):
    start = 59 + index * 0.4
    add(tone((74, 76, 79, 81, 79, 76, 74, 69)[index], 0.19), start, 0.058, (index % 2 - 0.5) * 0.5)
    add(percussion(0.045, 85, False), start, 0.09)

for start in np.arange(62.3, 65.2, 0.14):
    add(percussion(0.025, 180, False), float(start), 0.024, float(rng.uniform(-0.22, 0.22)))
for index, note in enumerate((50, 57, 64)):
    add(tone(note, 6, "pad"), 62.2, 0.023, (index - 1) * 0.5)

dry = mix.copy()
for delay, gain in ((0.15, 0.12), (0.3, 0.10), (0.6, 0.065), (0.9, 0.035)):
    samples = round(delay * RATE)
    mix[samples:] += dry[:-samples, ::-1] * gain
timeline = np.arange(len(mix)) / RATE
mix *= (np.minimum(timeline / 1.5, 1) * np.clip((DURATION - timeline) / 2.5, 0, 1))[:, None]
mix *= 0.75 / np.max(np.abs(mix))
assert mix.shape == (DURATION * RATE, 2)
assert np.max(np.abs(mix)) < 1
assert np.max(np.abs(mix[:480])) < 0.003 and np.max(np.abs(mix[-480:])) < 0.003
wav = OUT / "combe-score.wav"
with wave.open(str(wav), "wb") as output:
    output.setnchannels(2)
    output.setsampwidth(2)
    output.setframerate(RATE)
    output.writeframes((mix * 32767).astype("<i2").tobytes())
subprocess.run(["ffmpeg", "-hide_banner", "-loglevel", "error", "-y", "-i", str(wav), "-c:a", "aac", "-b:a", "256k", str(LOCAL / "combe-score.m4a")], check=True)
notes = {"duration_seconds": DURATION, "sample_rate": RATE, "channels": 2, "tempo_bpm": 150, "peak_dbfs": float(20 * np.log10(np.max(np.abs(mix)))), "rms_dbfs": float(20 * np.log10(np.sqrt(np.mean(mix ** 2)))), "origin": "Original deterministic procedural synthesis; no samples or third-party music", "sections": {"0-4": "warm pad reveal and soft air", "4-6.5": "four-note brand motif", "6.5-8": "product establishing breath", "8-59": "150 BPM beat, arpeggios, sparse melody and breathing sections", "59-62.2": "eight light recap pulses spaced 0.4 seconds", "62.2-65.2": "beat drops out; soft typing-like taps and pad", "65.2-68.2": "pad only; reading hold", "68.2-72": "brand motif returns; sustained final chord and fade"}}
measurement = subprocess.run(["ffmpeg", "-hide_banner", "-i", str(wav), "-af", "loudnorm=print_format=json", "-f", "null", "-"], capture_output=True, text=True, check=True).stderr
levels = json.loads(measurement[measurement.rfind("{"):measurement.rfind("}") + 1])
notes.update(integrated_lufs=float(levels["input_i"]), true_peak_dbtp=float(levels["input_tp"]), loudness_range_lu=float(levels["input_lra"]))
(LOCAL / "score-notes.json").write_text(json.dumps(notes, indent=2) + "\n")
print(json.dumps(notes, indent=2))
