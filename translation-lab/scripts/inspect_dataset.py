import zipfile
import re
import csv
import os
from collections import Counter

ZIP_PATH = "data/raw/OpenSubtitles-v2024-ja-ko.txt.zip"
REPORT_PATH = "reports/dataset_inspection.txt"
SAMPLE_PATH = "reports/inspection_samples.csv"

SAMPLE_LIMIT = 300

def count_japanese(text):
    return len(re.findall(r'[\u3040-\u30ff\u3400-\u4dbf\u4e00-\u9fff]', text))

def count_korean(text):
    return len(re.findall(r'[\uac00-\ud7a3]', text))

def inspect(ja, ko):
    ja = ja.strip()
    ko = ko.strip()

    reasons = []

    ja_chars = len(ja)
    ko_chars = len(ko)

    j_count = count_japanese(ja)
    k_count = count_korean(ko)

    if not ja or not ko:
        reasons.append("empty")

    if ja_chars > 0 and j_count / ja_chars < 0.15:
        reasons.append("weak_japanese")

    if ko_chars > 0 and k_count / ko_chars < 0.25:
        reasons.append("weak_korean")

    if ja_chars > 0 and ko_chars > 0:
        ratio = ko_chars / ja_chars

        if ratio < 0.15 or ratio > 5:
            reasons.append("length_ratio")

    if ja_chars > 300 or ko_chars > 300:
        reasons.append("too_long")

    if re.search(r'[\ufffd]', ja + ko):
        reasons.append("encoding_error")

    if re.search(r'[�□■]', ja + ko):
        reasons.append("suspicious_character")

    if not reasons:
        category = "candidate"
    elif any(x in reasons for x in [
        "empty", "encoding_error", "suspicious_character"
    ]):
        category = "rejected"
    else:
        category = "review"

    return category, reasons

def main():
    os.makedirs("reports", exist_ok=True)

    counts = Counter()
    reasons_count = Counter()
    samples = []
    total = 0

    with zipfile.ZipFile(ZIP_PATH) as z:
        with z.open("OpenSubtitles.ja-ko.ja") as ja_file, \
             z.open("OpenSubtitles.ja-ko.ko") as ko_file:

            for index, (ja_raw, ko_raw) in enumerate(
                zip(ja_file, ko_file), start=1
            ):
                ja = ja_raw.decode("utf-8", errors="replace").strip()
                ko = ko_raw.decode("utf-8", errors="replace").strip()

                category, reasons = inspect(ja, ko)

                total += 1
                counts[category] += 1
                reasons_count.update(reasons)

                if reasons and len(samples) < SAMPLE_LIMIT:
                    samples.append([
                        index, category, ",".join(reasons), ja, ko
                    ])

                if total % 100000 == 0:
                    print(f"Inspected: {total:,}")

    with open(REPORT_PATH, "w", encoding="utf-8") as f:
        f.write(f"Total pairs: {total:,}\n\n")

        f.write("[Categories]\n")
        for category in ["candidate", "review", "rejected"]:
            count = counts[category]
            percent = count / total * 100 if total else 0
            f.write(f"{category}: {count:,} ({percent:.2f}%)\n")

        f.write("\n[Reasons]\n")
        for reason, count in reasons_count.most_common():
            f.write(f"{reason}: {count:,}\n")

    with open(SAMPLE_PATH, "w", newline="", encoding="utf-8-sig") as f:
        writer = csv.writer(f)
        writer.writerow([
            "index", "category", "reasons",
            "japanese", "korean"
        ])
        writer.writerows(samples)

    print("\n=== INSPECTION COMPLETE ===")
    print("Total:", f"{total:,}")
    print("Categories:", dict(counts))
    print("Reasons:", dict(reasons_count))
    print("Report:", REPORT_PATH)
    print("Samples:", SAMPLE_PATH)

if __name__ == "__main__":
    main()
