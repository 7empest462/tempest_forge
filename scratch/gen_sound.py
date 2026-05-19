import struct
import math

def generate_laser_hum(filename, duration=2.0, sample_rate=44100):
    num_samples = int(duration * sample_rate)
    amplitude = 16384  # 50% volume for 16-bit
    
    # WAV Header
    header = bytearray()
    header.extend(b'RIFF')
    header.extend(struct.pack('<I', 36 + num_samples * 2)) # ChunkSize
    header.extend(b'WAVE')
    header.extend(b'fmt ')
    header.extend(struct.pack('<I', 16)) # Subchunk1Size
    header.extend(struct.pack('<H', 1))  # AudioFormat (PCM)
    header.extend(struct.pack('<H', 1))  # NumChannels (Mono)
    header.extend(struct.pack('<I', sample_rate))
    header.extend(struct.pack('<I', sample_rate * 2)) # ByteRate
    header.extend(struct.pack('<H', 2))  # BlockAlign
    header.extend(struct.pack('<H', 16)) # BitsPerSample
    header.extend(b'data')
    header.extend(struct.pack('<I', num_samples * 2)) # Subchunk2Size
    
    with open(filename, 'wb') as f:
        f.write(header)
        for i in range(num_samples):
            t = i / sample_rate
            # A pulsing laser hum (combination of a low base and a higher frequency)
            val = math.sin(2 * math.pi * 100 * t) * 0.6 + math.sin(2 * math.pi * 440 * t) * 0.2
            # Add some "vibrato"
            vibrato = math.sin(2 * math.pi * 5 * t) * 20
            val += math.sin(2 * math.pi * (200 + vibrato) * t) * 0.2
            
            sample = int(val * amplitude)
            f.write(struct.pack('<h', sample))

if __name__ == "__main__":
    import os
    os.makedirs("assets", exist_ok=True)
    generate_laser_hum("assets/laser_hum_fixed.wav")
    print("Generated assets/laser_hum_fixed.wav")
