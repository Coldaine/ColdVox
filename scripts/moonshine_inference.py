import torch
from transformers import AutoModelForSpeechSeq2Seq, AutoProcessor
import librosa

# Load model and processor
device = "cpu"
torch_dtype = torch.float32

model = AutoModelForSpeechSeq2Seq.from_pretrained(
    model_id,
    torch_dtype=torch_dtype,
    low_cpu_mem_usage=True,
)
model.to(device)

processor = AutoProcessor.from_pretrained(model_id)

# Load audio
audio_array, sampling_rate = librosa.load(audio_path_str, sr=16000, mono=True)

# Process
inputs = processor(audio_array, sampling_rate=16000, return_tensors="pt")
inputs = {k: v.to(device) for k, v in inputs.items()}

# Generate
with torch.no_grad():
    generated_ids = model.generate(**inputs)

# Decode
transcription = processor.batch_decode(generated_ids, skip_special_tokens=True)[0]
