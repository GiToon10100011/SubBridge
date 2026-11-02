import pandas as pd

INPUT = "reports/similarity_scores.csv"
OUTPUT = "reports/validation_sample.csv"

df = pd.read_csv(INPUT)

df["group"] = pd.cut(
    df["similarity"],
    bins=[-1, 0.35, 0.55, 2],
    labels=["low", "middle", "high"]
)

samples = []

for group in ["low", "middle", "high"]:
    part = df[df["group"] == group]

    selected = part.sample(
        n=min(30, len(part)),
        random_state=42
    ).copy()

    selected["judgment"] = ""
    selected["memo"] = ""

    samples.append(selected)

result = pd.concat(samples, ignore_index=True)

result.to_csv(
    OUTPUT,
    index=False,
    encoding="utf-8-sig"
)

print(result.groupby("group", observed=False).size())
print("Saved:", OUTPUT)
