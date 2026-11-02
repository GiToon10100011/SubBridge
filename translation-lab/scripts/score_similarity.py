import csv
import json
import urllib.request
import math
import os

INPUT = "reports/comparison_samples.csv"
OUTPUT = "reports/similarity_scores.csv"

API = "http://localhost:11434/api/embed"
MODEL = "qwen3-embedding:0.6b"
BATCH_SIZE = 16

def embed(texts):
    data = json.dumps({
        "model": MODEL,
        "input": texts
    }).encode("utf-8")

    req = urllib.request.Request(
        API,
        data=data,
        headers={"Content-Type": "application/json"}
    )

    with urllib.request.urlopen(req, timeout=120) as res:
        result = json.loads(res.read().decode("utf-8"))

    return result["embeddings"]

def cosine(a, b):
    dot = sum(x * y for x, y in zip(a, b))
    norm_a = math.sqrt(sum(x * x for x in a))
    norm_b = math.sqrt(sum(x * x for x in b))

    return dot / (norm_a * norm_b)

with open(INPUT, encoding="utf-8-sig") as f:
    rows = list(csv.DictReader(f))

results = []

for start in range(0, len(rows), BATCH_SIZE):
    batch = rows[start:start + BATCH_SIZE]

    texts = []
    for row in batch:
        texts.extend([
            row["japanese"],
            row["korean"]
        ])

    vectors = embed(texts)

    for i, row in enumerate(batch):
        score = cosine(vectors[i * 2], vectors[i * 2 + 1])

        results.append({
            "index": row["index"],
            "category": row["category"],
            "similarity": round(score, 5),
            "japanese": row["japanese"],
            "korean": row["korean"]
        })

    print(f"Processed: {min(start + len(batch), len(rows))}/{len(rows)}")

with open(OUTPUT, "w", newline="", encoding="utf-8-sig") as f:
    writer = csv.DictWriter(
        f,
        fieldnames=[
            "index", "category", "similarity",
            "japanese", "korean"
        ]
    )
    writer.writeheader()
    writer.writerows(results)

print("\n=== COMPLETE ===")
print("Total:", len(results))
print("Saved:", OUTPUT)

for category in ["candidate", "review"]:
    scores = [
        x["similarity"]
        for x in results
        if x["category"] == category
    ]

    if scores:
        print(
            category,
            "average similarity:",
            round(sum(scores) / len(scores), 4)
        )
