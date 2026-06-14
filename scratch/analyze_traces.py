def analyze():
    import collections
    
    stacks = collections.defaultdict(int)
    
    try:
        with open("cargo-flamegraph.trace", "r") as f:
            for line in f:
                line = line.strip()
                if not line:
                    continue
                parts = line.split(" ")
                if len(parts) < 2:
                    continue
                stack_str = parts[0]
                try:
                    samples = int(parts[-1])
                except ValueError:
                    continue
                stacks[stack_str] += samples
    except FileNotFoundError:
        print("cargo-flamegraph.trace not found")
        return

    print("Analyzing stacks containing 'bevy_mikktspace'...")
    print("-" * 120)
    
    mikkt_stacks = []
    for stack, samples in stacks.items():
        if "bevy_mikktspace" in stack:
            # We want to see what is immediately preceding bevy_mikktspace in the stack
            frames = stack.split(";")
            mikkt_stacks.append((frames, samples))
            
    # Sort by samples descending
    mikkt_stacks.sort(key=lambda x: x[1], reverse=True)
    
    for frames, samples in mikkt_stacks[:20]:
        # Find index of first bevy_mikktspace frame
        idx = -1
        for i, frame in enumerate(frames):
            if "bevy_mikktspace" in frame:
                idx = i
                break
        if idx != -1:
            caller_stack = frames[max(0, idx-5):idx]
            print(f"Samples: {samples:<6} | Caller: {' -> '.join(caller_stack)}")

if __name__ == "__main__":
    analyze()
