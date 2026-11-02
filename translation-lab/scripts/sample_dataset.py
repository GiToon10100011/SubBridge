import zipfile
import random
import csv
import os

ZIP_PATH = "data/raw/OpenSubtitles-v2024-ja-ko.txt.zip"
OUTPUT_PATH = "data/samples/random_100.csv"

os.makedirs("data/samples", exist_ok=True)

with zipfile.ZipFile(ZIP_PATH) as z:
    with z.open("OpenSubtitles.ja-ko.ja") as f:
        ja = f.read().decode("utf-8").split("\n")

    with z.open("OpenSubtitles.ja-ko.ko") as f:
        ko = f.read().decode("utf-8").split("\n")

# 마지막 줄의 개행으로 생기는 빈 항목 제거
if ja and ja[-1] == "":
    ja.pop()
if ko and ko[-1] == "":
    ko.pop()

# CRLF 개행 처리
ja = [line.rstrip("\r") for line in ja]
ko = [line.rstrip("\r") for line in ko]

print("=== LINE COUNT ===")
print("Japanese:", len(ja))
print("Korean:", len(ko))

if len(ja) != len(ko):
    print("WARNING: 두 파일의 줄 수가 다릅니다.")
    print("공통으로 존재하는 줄만 표본 추출합니다.")

count = min(len(ja), len(ko))

if count < 3:
    raise ValueError("데이터가 너무 적습니다.")

random.seed(42)
indices = sorted(random.sample(range(1, count - 1), min(100, count - 2)))

with open(OUTPUT_PATH, "w", newline="", encoding="utf-8-sig") as f:
    writer = csv.writer(f)
    writer.writerow([
        "index",
        "ja_prev", "ja", "ja_next",
        "ko_prev", "ko", "ko_next"
    ])

    for i in indices:
        writer.writerow([
            i + 1,
            ja[i - 1], ja[i], ja[i + 1],
            ko[i - 1], ko[i], ko[i + 1]
        ])

print("\nSaved:", OUTPUT_PATH)
print("Sample count:", len(indices))
