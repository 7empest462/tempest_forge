import wave
import os

input_wav = "/Volumes/Corsair_Lab/Home/Projects/tempest_forge/assets/Firefly_audio_A_.wav_file_for_my_game;_I_need_it_to_sound_like_a_variation3.wav"
assets_dir = "/Volumes/Corsair_Lab/Home/Projects/tempest_forge/assets"

if not os.path.exists(input_wav):
    print(f"Error: {input_wav} not found")
    exit(1)

with wave.open(input_wav, 'rb') as f:
    params = f.getparams()
    channels = f.getnchannels()
    width = f.getsampwidth()
    rate = f.getframerate()
    total_frames = f.getnframes()
    raw_data = f.readframes(total_frames)

bytes_per_frame = channels * width
print(f"Input file parameters: {params}")
print(f"Bytes per frame: {bytes_per_frame}")

# 1. Reload sound: 1.20s to 1.92s
reload_start_sec = 1.20
reload_end_sec = 1.92
reload_start_frame = int(reload_start_sec * rate)
reload_end_frame = int(reload_end_sec * rate)

reload_data = raw_data[reload_start_frame * bytes_per_frame : reload_end_frame * bytes_per_frame]

reload_out_path = os.path.join(assets_dir, "gun_reload.wav")
with wave.open(reload_out_path, 'wb') as f:
    f.setnchannels(channels)
    f.setsampwidth(width)
    f.setframerate(rate)
    f.writeframes(reload_data)
print(f"Saved reload sound to {reload_out_path} ({len(reload_data)/bytes_per_frame} frames, {len(reload_data)/bytes_per_frame/rate:.3f}s)")

# 2. Gunshot sound: 1.92s to 3.00s
shoot_start_sec = 1.92
shoot_end_sec = 3.00
shoot_start_frame = int(shoot_start_sec * rate)
shoot_end_frame = int(shoot_end_sec * rate)

shoot_data = raw_data[shoot_start_frame * bytes_per_frame : shoot_end_frame * bytes_per_frame]

shoot_targets = ["pistol_shoot.wav", "revolver_shoot.wav", "rifle_shoot.wav", "sniper_shoot.wav"]

for target in shoot_targets:
    target_path = os.path.join(assets_dir, target)
    with wave.open(target_path, 'wb') as f:
        f.setnchannels(channels)
        f.setsampwidth(width)
        f.setframerate(rate)
        f.writeframes(shoot_data)
    print(f"Saved gunshot sound to {target_path} ({len(shoot_data)/bytes_per_frame} frames, {len(shoot_data)/bytes_per_frame/rate:.3f}s)")
