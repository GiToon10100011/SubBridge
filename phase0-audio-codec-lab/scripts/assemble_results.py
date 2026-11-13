#!/usr/bin/env python3
"""results/<platform>/*.csv 를 results/ 최상위 CSV 로 합친다.

실행하지 못한 플랫폼(결과 CSV 가 없는 곳)은 행을 지어내지 않고 NOT_RUN 표식 행 하나를 넣는다.
사용: python3 scripts/assemble_results.py
"""
import csv
from pathlib import Path

RESULTS = Path(__file__).resolve().parent.parent / "results"
PLATFORMS = [("macos", "macos-aarch64"), ("windows", "windows-x86_64")]
NAMES = ["compatibility", "audio_quality", "performance", "decoder_compare"]
MARK_COLUMNS = ("verdict", "success", "pair", "note")

for name in NAMES:
    header = None
    rows = []
    missing = []
    for d, plat in PLATFORMS:
        # name.csv = 호환성 세트(또는 성능 세트의 성능 CSV), name_perf.csv = 성능 세트의 판정/비교 CSV
        files = [RESULTS / d / f"{name}.csv"]
        if name != "performance":
            files.append(RESULTS / d / f"{name}_perf.csv")
        found = [p for p in files if p.exists()]
        if not found:
            missing.append(plat)
            continue
        for p in found:
            with open(p, newline="", encoding="utf-8") as f:
                r = csv.reader(f)
                h = next(r)
                header = header or h
                if h != header:
                    raise SystemExit(f"{p}: header differs from other platforms")
                rows += list(r)
    if header is None:
        print(f"skip {name}: no platform results")
        continue
    for plat in missing:
        marker = [""] * len(header)
        marker[header.index("platform")] = plat
        marker[header.index("fixture_id")] = "ALL"
        for c in MARK_COLUMNS:
            if c in header:
                marker[header.index(c)] = "NOT_RUN"
        rows.append(marker)
    with open(RESULTS / f"{name}.csv", "w", newline="", encoding="utf-8") as f:
        w = csv.writer(f)
        w.writerow(header)
        w.writerows(rows)
    print(f"{name}.csv: {len(rows)} rows (NOT_RUN: {', '.join(missing) or 'none'})")
