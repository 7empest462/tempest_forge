import re

def analyze():
    print("Searching for project-specific symbols...")
    with open("flamegraph.svg", "r") as f:
        content = f.read()
    
    # Matches <title>SymbolName (Samples, Percentage)</title>
    pattern = r"<title>(.*?)\s*\((\d+)\s+samples,\s+([\d.]+)%\)</title>"
    matches = re.findall(pattern, content)
    
    keywords = ["tempest_forge", "voxel", "water", "settlement", "animals", "wildlife", "ui", "procedural"]
    filtered = []
    for func, samples, pct in matches:
        if any(kw in func.lower() for kw in keywords):
            filtered.append((func, int(samples), float(pct)))
            
    filtered_sorted = sorted(filtered, key=lambda x: x[1], reverse=True)
    
    print("\nProject-Specific Systems & Operations in Flamegraph:")
    print("-" * 120)
    print(f"{'Samples':<10} | {'Percentage':<12} | {'Function / Symbol'}")
    print("-" * 120)
    for func, samples, pct in filtered_sorted[:50]:
        clean_func = func
        if len(clean_func) > 95:
            clean_func = clean_func[:92] + "..."
        print(f"{samples:<10} | {pct:<12}% | {clean_func}")

if __name__ == "__main__":
    analyze()
