import wave
import contextlib
import os

wav_path = "/Volumes/Corsair_Lab/Home/Projects/tempest_forge/assets/Firefly_audio_A_.wav_file_for_my_game;_I_need_it_to_sound_like_a_variation3.wav"

if not os.path.exists(wav_path):
    print(f"File not found: {wav_path}")
    exit(1)

with contextlib.closing(wave.open(wav_path, 'r')) as f:
    frames = f.getnframes()
    rate = f.getframerate()
    channels = f.getnchannels()
    width = f.getsampwidth()
    duration = frames / float(rate)
    print(f"Channels: {channels}")
    print(f"Sample width (bytes): {width}")
    print(f"Frame rate (Hz): {rate}")
    print(f"Total frames: {frames}")
    print(f"Duration (seconds): {duration:.3f}")
