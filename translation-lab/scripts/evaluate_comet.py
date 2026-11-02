from comet import download_model, load_from_checkpoint

model_path = download_model("Unbabel/XCOMET-XL")
model = load_from_checkpoint(model_path)

data = [
    {
        "src": "今日はいい天気ですね。",
        "mt": "오늘 날씨가 좋네요."
    },
    {
        "src": "今日はいい天気ですね。",
        "mt": "나는 어제 학교에 갔어요."
    },
    {
        "src": "彼は昨日、駅で友達に会いました。",
        "mt": "그는 어제 역에서 친구를 만났습니다."
    },
    {
        "src": "彼は昨日、駅で友達に会いました。",
        "mt": "그는 어제 친구를 만났습니다."
    },
]

print("=== Evaluating Japanese → Korean ===")

output = model.predict(
    data,
    batch_size=2,
    gpus=0,
    accelerator="mps"
)

for item, score in zip(data, output.scores):
    print("\nJapanese:", item["src"])
    print("Korean:", item["mt"])
    print("Score:", round(score, 4))

print("\nSystem score:", round(output.system_score, 4))
