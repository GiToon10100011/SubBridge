from comet import download_model, load_from_checkpoint
import torch

MODEL_NAME = "Unbabel/XCOMET-XL"

print("=== Loading XCOMET-XL ===")

model_path = download_model(MODEL_NAME)
model = load_from_checkpoint(model_path)

device = "mps" if torch.backends.mps.is_available() else "cpu"
model.to(device)

print("Model loaded successfully!")
print("Device:", device)
print("Model:", MODEL_NAME)
