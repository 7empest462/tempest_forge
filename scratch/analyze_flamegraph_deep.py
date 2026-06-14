import re

def analyze():
    print("Searching for the deepest stack frames (smallest y values)...")
    with open("flamegraph.svg", "r") as f:
        content = f.read()
    
    # Each group has <rect ... y="VALUE" ... /> and <title>SymbolName (Samples, Percentage)</title>
    # Let's match: <g><title>(.*?)</title><rect x=".*?" y="(\d+)"
    pattern = r"<g><title>(.*?)</title><rect\s+[^>]*?y=\"(\d+)\""
    matches = re.findall(pattern, content)
    
    # Let's convert Y to integer and sort ascending (smallest Y = highest in the visual graph)
    sorted_matches = sorted(matches, key=lambda x: int(x[1]))
    
    print("\nTop 50 Deepest Stack Frames (Highest Visual Spikes):")
    print("-" * 120)
    print(f"{'Y (pixels)':<12} | {'Symbol'}")
    print("-" * 120)
    
    seen = set()
    count = 0
    for title, y in sorted_matches:
        if title not in seen:
            seen.add(title)
            print(f"{y:<12} | {title}")
            count += 1
            if count >= 50:
                break

if __name__ == "__main__":
    analyze()
