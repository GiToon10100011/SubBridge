import zipfile
import random
import csv
import os

ZIP_PATH = "data/raw/OpenSubtitles-v2024-ja-ko.txt.zip"
OUTPUT = "reports/comparison_samples.csv"

random.seed(42)
os.makedirs("reports", exist_ok=True)

def inspect(ja, ko):
    import re

    ja = ja.strip()
    ko = ko.strip()

    jc = len(ja)
    kc = len(ko)

    j_count = len(re.findall(r'[\u3040-\u30ff\u3400-\u4dbf\u4e00-\u9fff]', ja))
    k_count = len(re.findall(r'[\uac00-\ud7a3]', ko))

    reasons = []

    if not ja or not ko:
        reasons.append("empty")
    if jc and j_count / jc < 0.15:
        reasons.append("weak_japanese")
    if kc and k_count / kc < 0.25:
        reasons.append("weak_korean")
    if jc and kc and (kc / jc < 0.15 or kc / jc > 5):
        reasons.append("length_ratio")
    if jc > 300 or kc > 300:
        reasons.append("too_long")
    if re.search(r'[\ufffd]', ja + ko):
        reasons.append("encoding_error")
    if re.search(r'[�□■]', ja + ko):
        reasons.append("suspicious_character")

    if not reasons:
        return "candidate"
    if any(x in reasons for x in [
        "empty", "encoding_error", "suspicious_character"
    ]):
        return "rejected"
    return "review"

reservoir = {
    "candidate": [],
    "review": []
}

counts = {
    "candidate": 0,
    "review": 0,
    "rejected": 0
}

with zipfile.ZipFile(ZIP_PATH) as z:
    with z.open("OpenSubtitles.ja-ko.ja") as ja_file, \
         z.open("OpenSubtitles.ja-ko.ko") as ko_file:

        for i, (ja_raw, ko_raw) in enumerate(
            zip(ja_file, ko_file), start=1
        ):
            ja = ja_raw.decode("utf-8", errors="replace").strip()
            ko = ko_raw.decode("utf-8", errors="replace").strip()

            category = inspect(ja, ko)
            counts[category] += 1

            if category not in reservoir:
                continue

            bucket = reservoir[category]
            item = (i, category, ja, ko)
            n = counts[category]

            if len(bucket) < 100:
                bucket.append(item)
            else:
                pos = random.randrange(n)
                if pos < 100:
                    bucket[pos] = item

with open(OUTPUT, "w", newline="", encoding="utf-8-sig") as f:
    writer = csv.writer(f)
    writer.writerow(["index", "category", "japanese", "korean"])

    for category in ["candidate", "review"]:
        for item in reservoir[category]:
            writer.writerow(item)

print("=== COUNTS ===")
print(counts)
print("\n=== SAMPLES ===")
print("Candidate:", len(reservoir["candidate"]))
print("Review:", len(reservoir["review"]))
print("Saved:", OUTPUT)
