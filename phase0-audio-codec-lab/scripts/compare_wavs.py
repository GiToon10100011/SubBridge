#!/usr/bin/env python3
"""두 outputs 폴더(<폴더>/<픽스처>/<모드>_<트랙>.wav)의 같은 이름 WAV 를 샘플 단위로 비교한다 (W6).

SHA-256 이 다른 쌍이 "부동소수점 반올림 수준"인지 "실제로 다른 소리"인지 가른다.
  - 샘플 수가 다르면 길이 차이만 적고 비교하지 않는다.
  - 같으면 다른 샘플 수, 최대 |차이|(S16 LSB), RMS 차이(LSB), 신호 대 차이 비(dB)를 적는다.
사용: python3 -I scripts/compare_wavs.py results/macos/outputs <다른 outputs 폴더> [--ids F01,F02,...] [--csv 결과.csv]
표준 라이브러리만 쓴다. 입력 WAV 는 16-bit PCM 이어야 한다 (이 랩의 출력 규격).
"""
import argparse
import array
import csv
import math
import sys
import wave
from pathlib import Path


def read_samples(path):
    with wave.open(str(path), "rb") as w:
        if w.getsampwidth() != 2:
            raise ValueError(f"not 16-bit: sampwidth={w.getsampwidth()}")
        ch = w.getnchannels()
        data = array.array("h")
        data.frombytes(w.readframes(w.getnframes()))
    if sys.byteorder == "big":
        data.byteswap()
    return data, ch


def compare(a, b):
    if len(a) != len(b):
        return {"verdict": "LENGTH_DIFF"}
    n_diff = 0
    max_abs = 0
    sq_diff = 0
    sq_sig = 0
    for x, y in zip(a, b):
        d = x - y
        if d:
            n_diff += 1
            if abs(d) > max_abs:
                max_abs = abs(d)
            sq_diff += d * d
        sq_sig += x * x
    n = len(a)
    rms_diff = math.sqrt(sq_diff / n) if n else 0.0
    rms_sig = math.sqrt(sq_sig / n) if n else 0.0
    snr = "" if sq_diff == 0 or rms_sig == 0 else f"{20 * math.log10(rms_sig / rms_diff):.1f}"
    return {
        "verdict": "IDENTICAL" if n_diff == 0 else "DIFFERENT",
        "diff_samples": n_diff,
        "diff_pct": f"{100 * n_diff / n:.4f}" if n else "0",
        "max_abs_diff_lsb": max_abs,
        "rms_diff_lsb": f"{rms_diff:.4f}",
        "snr_db": snr,
    }


def main():
    p = argparse.ArgumentParser()
    p.add_argument("dir_a")
    p.add_argument("dir_b")
    p.add_argument("--ids", help="쉼표로 구분한 픽스처 ID. 생략하면 두 폴더에 모두 있는 것 전부")
    p.add_argument("--csv", help="결과를 CSV 로도 저장")
    args = p.parse_args()
    a_dir, b_dir = Path(args.dir_a), Path(args.dir_b)
    ids = args.ids.split(",") if args.ids else sorted(d.name for d in a_dir.iterdir() if d.is_dir() and (b_dir / d.name).is_dir())

    cols = ["fixture_id", "file", "verdict", "samples_a", "samples_b", "diff_samples", "diff_pct", "max_abs_diff_lsb", "rms_diff_lsb", "snr_db"]
    rows = []
    for fid in ids:
        for fa in sorted((a_dir / fid).glob("*.wav")):
            fb = b_dir / fid / fa.name
            row = {"fixture_id": fid, "file": fa.name}
            if not fb.exists():
                row["verdict"] = "MISSING_B"
            else:
                try:
                    (sa, ca), (sb, cb) = read_samples(fa), read_samples(fb)
                    row.update(samples_a=len(sa), samples_b=len(sb))
                    row.update(compare(sa, sb) if ca == cb == 1 else {"verdict": f"CHANNELS {ca} vs {cb}"})
                except Exception as e:  # 깨진 WAV 도 표에 남긴다
                    row["verdict"] = f"ERROR {e}"
            rows.append(row)

    print("\t".join(cols))
    for r in rows:
        print("\t".join(str(r.get(c, "")) for c in cols))
    summary = {}
    for r in rows:
        summary[r["verdict"]] = summary.get(r["verdict"], 0) + 1
    print("\nsummary:", ", ".join(f"{k}={v}" for k, v in sorted(summary.items())))
    if args.csv:
        with open(args.csv, "w", newline="", encoding="utf-8") as f:
            w = csv.DictWriter(f, fieldnames=cols)
            w.writeheader()
            w.writerows(rows)


if __name__ == "__main__":
    main()
