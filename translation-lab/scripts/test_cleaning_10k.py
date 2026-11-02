import csv
import json
import math
import random
import zipfile
import urllib.request
from pathlib import Path

ZIP_PATH = "data/raw/OpenSubtitles-v2024-ja-ko.txt.zip"
OUTPUT_DIR = Path("reports/cleaning_test")
MODEL = "qwen3-embedding:0.6b"
API = "http://localhost:11434/api/embed"

SAMPLE_SIZE = 10000
BATCH_SIZE = 16
random.seed(42)

def embed(texts):
    body = json.dumps({
        "model": MODEL,
        "input": texts
    }).encode("utf-8")

    req = urllib.request.Request(
        API, data=body,
        headers={"Content-Type": "application/json"}
    )

    with urllib.request.urlopen(req, timeout=180) as res:
        return json.loads(res.read())["embeddings"]

def cosine(a, b):
    dot = sum(x*y for x, y in zip(a, b))
    na = math.sqrt(sum(x*x for x in a))
    nb = math.sqrt(sum(x*x for x in b))
    return dot / (na * nb) if na and nb else 0

def inspect(ja, ko):
    reasons = []

    ja_ratio = sum(
        '\u3040' <= c <= '\u30ff' or '\u4e00' <= c <= '\u9fff'
        for c in ja
    ) / max(len(ja), 1)

    ko_ratio = sum('\uac00' <= c <= '\ud7a3' for c in ko) / max(len(ko), 1)

    ratio = len(ko) / max(len(ja), 1)

    if ja_ratio < 0.15:
        reasons.append("weak_japanese")
    if ko_ratio < 0.25:
        reasons.append("weak_korean")
    if ratio < 0.15 or ratio > 5:
        reasons.append("length_ratio")
    if len(ja) > 300 or len(ko) > 300:
        reasons.append("too_long")

    return reasons

OUTPUT_DIR.mkdir(parents=True, exist_ok=True)

with zipfile.ZipFile(ZIP_PATH) as z:
    with z.open("OpenSubtitles.ja-ko.ja") as jf, z.open("OpenSubtitles.ja-ko.ko") as kf:
        ja_lines = (line.decode("utf-8").rstrip("\\r\\n") for line in jf)
        ko_lines = (line.decode("utf-8").rstrip("\\r\\n") for line in kf)
        pairs = list(zip(ja_lines, ko_lines))

    assert len(pairs) == 1_188_550, f"Unexpected pair count: {len(pairs)}"

indices = sorted(random.sample(range(len(pairs)), SAMPLE_SIZE))

rows = []
for idx in indices:
    ja_text, ko_text = pairs[idx]
    reasons = inspect(ja_text, ko_text)
    rows.append({
        "index": idx,
        "japanese": ja_text,
        "korean": ko_text,
        "rule_reasons": ";".join(reasons),
    })

for start in range(0, len(rows), BATCH_SIZE):
    batch = rows[start:start+BATCH_SIZE]
    texts = []
    for row in batch:
        texts.extend([row["japanese"], row["korean"]])

    vectors = embed(texts)

    for i, row in enumerate(batch):
        row["similarity"] = round(
            cosine(vectors[2*i], vectors[2*i+1]), 5
        )

    print(f"Processed {min(start+BATCH_SIZE, len(rows))}/{SAMPLE_SIZE}")

for row in rows:
    score = row["similarity"]
    reasons = row["rule_reasons"]

    if reasons or score < 0.35:
        row["decision"] = "review"
    elif score < 0.55:
        row["decision"] = "review"
    else:
        row["decision"] = "candidate"

output = OUTPUT_DIR / "sample_10k.csv"

with output.open("w", newline="", encoding="utf-8-sig") as f:
    writer = csv.DictWriter(f, fieldnames=rows[0].keys())
    writer.writeheader()
    writer.writerows(rows)

print("\n=== COMPLETE ===")
print("Total:", len(rows))
print("Candidate:", sum(r["decision"] == "candidate" for r in rows))
print("Review:", sum(r["decision"] == "review" for r in rows))
print("Saved:", output)
