
import time
import torch
from transformers import M2M100ForConditionalGeneration, M2M100Tokenizer

MODEL_NAME = "facebook/m2m100_418M"

device = "mps" if torch.backends.mps.is_available() else "cpu"

print(f"Device: {device}")
print(f"Loading model: {MODEL_NAME}")

start = time.perf_counter()

tokenizer = M2M100Tokenizer.from_pretrained(MODEL_NAME)
model = M2M100ForConditionalGeneration.from_pretrained(MODEL_NAME)
model.to(device)
model.eval()

print(f"Model loaded in {time.perf_counter() - start:.2f}s")

def translate(text: str) -> str:
    tokenizer.src_lang = "ja"

    encoded = tokenizer(
        text,
        return_tensors="pt",
        truncation=True,
        max_length=512,
    )

    encoded = {key: value.to(device) for key, value in encoded.items()}

    start = time.perf_counter()

    with torch.inference_mode():
        generated = model.generate(
            **encoded,
            forced_bos_token_id=tokenizer.get_lang_id("ko"),
            max_new_tokens=256,
        )

    elapsed = time.perf_counter() - start
    result = tokenizer.batch_decode(
        generated,
        skip_special_tokens=True,
    )[0]

    return result, elapsed


tests = [
    "そんなこと言われても困るんだけど。こっちだって色々考えてるんだから。",
    "そんなことを言われても、私はどうすればいいかわからないよ。",
    "なんでそんなことしたの？",
    "もうええ加減にせえや。",
]

for text in tests:
    result, elapsed = translate(text)

    print("\n" + "=" * 60)
    print("原文:", text)
    print("번역:", result)
    print(f"번역 시간: {elapsed:.3f}s")

