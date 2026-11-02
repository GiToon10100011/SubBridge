import zipfile
import random
import json
import math
import csv
import urllib.request
from pathlib import Path

ZIP_PATH = "data/raw/OpenSubtitles-v2024-ja-ko.txt.zip"
OUTPUT = "reports/context_similarity_200.csv"
MODEL = "qwen3-embedding:0.6b"
API = "http://localhost:11434/api/embed"

SAMPLE_SIZE = 200
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

with zipfile.ZipFile(ZIP_PATH) as z:
    with z.open("OpenSubtitles.ja-ko.ja") as jf, \
         z.open("OpenSubtitles.ja-ko.ko") as kf:

        ja = [line.decode("utf-8").rstrip("\r\n") for line in jf]
        ko = [line.decode("utf-8").rstrip("\r\n") for line in kf]

assert len(ja) == len(ko), (
    f"Line mismatch: ja={len(ja):,}, ko={len(ko):,}"
)

print(f"Matched pairs: {len(ja):,}")

indices = sorted(random.sample(range(1, len(ja)-1), SAMPLE_SIZE))
rows = []

for idx in indices:
    rows.append({
        "index": idx,
        "ja_context": "\n".join(ja[idx-1:idx+2]),
        "ko_context": "\n".join(ko[idx-1:idx+2]),
        "ja_current": ja[idx],
        "ko_current": ko[idx],
    })

for start in range(0, len(rows), BATCH_SIZE):
    batch = rows[start:start+BATCH_SIZE]
    texts = []

    for r in batch:
        texts.extend([
            r["ja_current"],
            r["ko_current"],
            r["ja_context"],
            r["ko_context"]
        ])

    vectors = embed(texts)

    for i, r in enumerate(batch):
        r["current_similarity"] = round(
            cosine(vectors[i*4], vectors[i*4+1]), 5
        )
        r["context_similarity"] = round(
            cosine(vectors[i*4+2], vectors[i*4+3]), 5
        )

    print(f"Processed {min(start+BATCH_SIZE, SAMPLE_SIZE)}/{SAMPLE_SIZE}")

Path("reports").mkdir(exist_ok=True)

with open(OUTPUT, "w", newline="", encoding="utf-8-sig") as f:
    writer = csv.DictWriter(f, fieldnames=rows[0].keys())
    writer.writeheader()
    writer.writerows(rows)

print("\n완료:", OUTPUT)
print("현재 문장 평균:", round(
    sum(r["current_similarity"] for r in rows)/len(rows), 4
))
print("문맥 포함 평균:", round(
    sum(r["context_similarity"] for r in rows)/len(rows), 4
))
