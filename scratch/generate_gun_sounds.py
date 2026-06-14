import wave
import struct
import math
import random
import os

SAMPLE_RATE = 44100

def write_wav(filename, samples):
    os.makedirs(os.path.dirname(filename), exist_ok=True)
    with wave.open(filename, 'w') as w:
        w.setnchannels(1)
        w.setsampwidth(2)
        w.setframerate(SAMPLE_RATE)
        for s in samples:
            # Clamp to 16-bit range
            val = int(max(-32768, min(32767, s * 32767)))
            w.writeframesraw(struct.pack('<h', val))
    print(f"Generated {filename}")

def gen_gunshot(duration, noise_decay, pitch_start, pitch_end, pitch_decay, volume=1.0, 
                metallic_ring_freq=3200, metallic_ring_decay=40.0, metallic_ring_amp=0.15):
    num_samples = int(SAMPLE_RATE * duration)
    samples = []
    
    prev_noise = 0.0
    for i in range(num_samples):
        t = i / SAMPLE_RATE
        
        # 1. High-passed white noise (crisp explosive crack)
        raw_noise = random.uniform(-1.0, 1.0)
        hp_noise = raw_noise - prev_noise
        prev_noise = raw_noise
        
        noise_amp = math.exp(-t * noise_decay)
        
        # 2. Pitch thump (low-end punch)
        freq = pitch_end + (pitch_start - pitch_end) * math.exp(-t * pitch_decay)
        phase = 2.0 * math.pi * freq * t
        sine = math.sin(phase)
        sine_amp = math.exp(-t * (pitch_decay * 1.5))
        
        # 3. Metallic ring (barrel/chamber vibration)
        # Mix two frequencies to form a rich metallic timbre
        ring1 = math.sin(2.0 * math.pi * metallic_ring_freq * t)
        ring2 = math.sin(2.0 * math.pi * (metallic_ring_freq * 1.43) * t)
        ring = (ring1 * 0.6 + ring2 * 0.4) * math.exp(-t * metallic_ring_decay) * metallic_ring_amp
        
        # 4. Firing pin click (extremely short mechanical strike at t=0)
        click = 0.0
        if t < 0.005:
            click = random.uniform(-1.0, 1.0) * math.exp(-t * 1000.0) * 0.3
            
        # Mix components: hp_noise (50%), sine (25%), metallic ring, and click
        sample = (hp_noise * 0.5 * noise_amp + sine * 0.25 * sine_amp + ring + click) * volume
        
        # Apply overall quick fade out at the very end to prevent clicking
        if i > num_samples - 200:
            sample *= (num_samples - i) / 200.0
            
        samples.append(sample)
        
    return samples

def gen_reload():
    # A full mechanical reload sequence:
    # 1. Slide pull / eject (metallic drag and click) at t=0.0 to 0.3
    # 2. Magazine insert (metallic snap) at t=0.5 to 0.8
    # 3. Slide release clack (heavy metal snap with resonance ring) at t=1.0 to 1.3
    duration = 1.3
    num_samples = int(SAMPLE_RATE * duration)
    samples = []
    
    prev_noise = 0.0
    for i in range(num_samples):
        t = i / SAMPLE_RATE
        sample = 0.0
        
        # High-passed noise
        raw_noise = random.uniform(-1.0, 1.0)
        hp_noise = raw_noise - prev_noise
        prev_noise = raw_noise
        
        # Part 1: Slide pull (t = 0.0 to 0.3)
        if 0.0 <= t < 0.3:
            # Click at start
            click1 = random.uniform(-1.0, 1.0) * math.exp(-t * 200.0) * 0.3
            # Slithering slide drag
            drag = hp_noise * 0.08 * math.exp(-(t - 0.05) * 5.0) if t > 0.05 else 0.0
            # Click at end of pull
            click2 = random.uniform(-1.0, 1.0) * math.exp(-(t - 0.2) * 300.0) * 0.4 if t > 0.2 else 0.0
            # Metallic ring
            ring = math.sin(2.0 * math.pi * 2200.0 * t) * math.exp(-t * 50.0) * 0.08
            sample = click1 + drag + click2 + ring
            
        # Part 2: Mag insert (t = 0.5 to 0.8)
        elif 0.5 <= t < 0.8:
            t_mag = t - 0.5
            # Initial touch
            click1 = random.uniform(-1.0, 1.0) * math.exp(-t_mag * 150.0) * 0.3
            # Friction drag
            drag = hp_noise * 0.06 * math.exp(-(t_mag - 0.05) * 8.0) if t_mag > 0.05 else 0.0
            # Locking snap
            click2 = random.uniform(-1.0, 1.0) * math.exp(-(t_mag - 0.15) * 250.0) * 0.5 if t_mag > 0.15 else 0.0
            # Resonance
            ring = math.sin(2.0 * math.pi * 1800.0 * t_mag) * math.exp(-t_mag * 40.0) * 0.05
            sample = click1 + drag + click2 + ring
            
        # Part 3: Slide release clack (t = 1.0 to 1.3)
        elif 1.0 <= t < 1.3:
            t_release = t - 1.0
            # Loud metal-on-metal collision
            clack = random.uniform(-1.0, 1.0) * math.exp(-t_release * 80.0) * 0.6
            # Heavy metal ringing frequencies
            ring1 = math.sin(2.0 * math.pi * 1500.0 * t_release) * 0.5
            ring2 = math.sin(2.0 * math.pi * 2800.0 * t_release) * 0.3
            ring3 = math.sin(2.0 * math.pi * 4000.0 * t_release) * 0.2
            ring = (ring1 + ring2 + ring3) * math.exp(-t_release * 30.0) * 0.25
            sample = clack + ring
            
        samples.append(sample)
        
    return samples

if __name__ == "__main__":
    assets_dir = "/Volumes/Corsair_Lab/Home/Projects/tempest_forge/assets"
    
    # 1. Pistol Shoot: Snappy pop, higher ring
    pistol = gen_gunshot(
        duration=0.3,
        noise_decay=14.0,
        pitch_start=700,
        pitch_end=150,
        pitch_decay=35.0,
        volume=0.8,
        metallic_ring_freq=3400,
        metallic_ring_decay=45.0,
        metallic_ring_amp=0.18
    )
    write_wav(os.path.join(assets_dir, "pistol_shoot.wav"), pistol)
    
    # 2. Revolver Shoot: Snappy but deeper punch and heavier ring
    revolver = gen_gunshot(
        duration=0.5,
        noise_decay=9.0,
        pitch_start=450,
        pitch_end=90,
        pitch_decay=22.0,
        volume=1.0,
        metallic_ring_freq=2200,
        metallic_ring_decay=25.0,
        metallic_ring_amp=0.22
    )
    write_wav(os.path.join(assets_dir, "revolver_shoot.wav"), revolver)
    
    # 3. Rifle Shoot: Faster decay for auto-fire, high snappy ring
    rifle = gen_gunshot(
        duration=0.2,
        noise_decay=18.0,
        pitch_start=600,
        pitch_end=170,
        pitch_decay=40.0,
        volume=0.75,
        metallic_ring_freq=3200,
        metallic_ring_decay=50.0,
        metallic_ring_amp=0.15
    )
    write_wav(os.path.join(assets_dir, "rifle_shoot.wav"), rifle)
    
    # 4. Sniper Shoot: Powerful deep thump, extremely long ringing sustain
    sniper = gen_gunshot(
        duration=1.5,
        noise_decay=3.5,
        pitch_start=350,
        pitch_end=55,
        pitch_decay=10.0,
        volume=1.0,
        metallic_ring_freq=1200,
        metallic_ring_decay=12.0,
        metallic_ring_amp=0.35
    )
    write_wav(os.path.join(assets_dir, "sniper_shoot.wav"), sniper)
    
    # 5. Reload sound: Heavy slide eject, mag insert, slide release clack
    reload_snd = gen_reload()
    write_wav(os.path.join(assets_dir, "gun_reload.wav"), reload_snd)
