import wave
import struct
import os

wav_path = "/Volumes/Corsair_Lab/Home/Projects/tempest_forge/assets/puddle_stepping.wav"
out_path = "/Volumes/Corsair_Lab/Home/Projects/tempest_forge/assets/puddle_stepping.wav"
backup_path = "/Volumes/Corsair_Lab/Home/Projects/tempest_forge/assets/puddle_stepping_original.wav"

if not os.path.exists(wav_path):
    print(f"File not found: {wav_path}")
    exit(1)

# Back up the original file if not already backed up
if not os.path.exists(backup_path):
    import shutil
    shutil.copyfile(wav_path, backup_path)
    print("Created backup of original wav file")

with wave.open(backup_path, 'rb') as w:
    params = w.getparams()
    nchannels, sampwidth, framerate, nframes, comptype, compname = params
    frames = w.readframes(nframes)

print(f"Original: Channels={nchannels}, Width={sampwidth}, Rate={framerate}, Frames={nframes}, Duration={nframes/framerate:.3f}s")

# Unpack frames to integers
if sampwidth == 2:
    fmt = f"<{nframes * nchannels}h" # 16-bit signed little endian
    samples = list(struct.unpack(fmt, frames))
elif sampwidth == 1:
    fmt = f"<{nframes * nchannels}B" # 8-bit unsigned
    samples = [float(s) - 128.0 for s in struct.unpack(fmt, frames)]
else:
    raise ValueError("Unsupported sample width")

# If stereo, average channels
if nchannels == 2:
    mono_samples = [(samples[i] + samples[i+1]) / 2.0 for i in range(0, len(samples), 2)]
else:
    mono_samples = samples

# Find the first footstep by analyzing energy
# We can compute root-mean-square (RMS) over small windows, e.g., 10ms
window_size = int(framerate * 0.01)
rms_envelope = []
for i in range(0, len(mono_samples), window_size):
    window = mono_samples[i:i+window_size]
    if not window:
        break
    rms = (sum(x*x for x in window) / len(window)) ** 0.5
    rms_envelope.append((i, rms))

# Find the peak of the first footstep
max_val = max(rms for _, rms in rms_envelope)
threshold = max_val * 0.05  # 5% of peak energy (lowered to catch the tail of the step)

# Find the peak in the first 0.8 seconds (to avoid hitting subsequent steps)
max_idx_to_search = min(len(rms_envelope), int(0.8 / 0.01))
peak_idx = 0
peak_val = 0
for idx, (sample_pos, rms) in enumerate(rms_envelope[:max_idx_to_search]):
    if rms > peak_val:
        peak_val = rms
        peak_idx = idx

# Now find where the first footstep ends (energy goes below threshold after the peak)
# We want it to stay below threshold for at least 150ms (15 windows)
end_frame = nframes
for idx, (sample_pos, rms) in enumerate(rms_envelope[peak_idx:]):
    actual_idx = peak_idx + idx
    if rms < threshold:
        # Verify it stays low for a bit (indicating a gap between steps)
        if all(r < threshold for _, r in rms_envelope[actual_idx:actual_idx+15]):
            # Add a small padding (e.g. 50ms) to not clip the tail
            end_frame = min(nframes, sample_pos + int(framerate * 0.05))
            break

# If we didn't find a clear end, default to 0.45 seconds
if end_frame == nframes or end_frame < int(framerate * 0.2):
    end_frame = int(framerate * 0.45)

print(f"Sliced at frame: {end_frame} ({end_frame / framerate:.3f}s)")

# Write the sliced frames
sliced_frames = frames[:end_frame * nchannels * sampwidth]
with wave.open(out_path, 'wb') as w:
    w.setparams((nchannels, sampwidth, framerate, end_frame, comptype, compname))
    w.writeframes(sliced_frames)
print("Successfully sliced and saved the first footstep!")
