import wave
import struct
import os

wav_path = "/Volumes/Corsair_Lab/Home/Projects/tempest_forge/assets/Firefly_audio_A_.wav_file_for_my_game;_I_need_it_to_sound_like_a_variation3.wav"

with wave.open(wav_path, 'rb') as f:
    channels = f.getnchannels()
    width = f.getsampwidth()
    rate = f.getframerate()
    frames = f.getnframes()
    raw_data = f.readframes(frames)

# Interpret raw data as 16-bit signed integers (little-endian)
num_samples = len(raw_data) // (width * channels)
fmt = f"<{num_samples * channels}h"  # 'h' is 16-bit signed int
samples = struct.unpack(fmt, raw_data)

# Compute absolute amplitude for both channels combined per window
window_sec = 0.02  # 20ms windows
window_size = int(rate * window_sec)
num_windows = num_samples // window_size

print(f"Window size: {window_size} frames ({window_sec*1000:.0f}ms)")
print("Time (s) | Amplitude Peak")
print("------------------------")
for i in range(num_windows):
    start_idx = i * window_size * channels
    end_idx = start_idx + window_size * channels
    win_samples = samples[start_idx:end_idx]
    
    # Peak absolute amplitude in this window
    peak = max(abs(s) for s in win_samples) if win_samples else 0
    time_sec = i * window_sec
    
    # Render a small bar graph to visualize it textually
    bar = "#" * int(peak / 1000)
    print(f"{time_sec:7.2f} | {peak:5d} {bar}")
