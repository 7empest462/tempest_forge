import re
from collections import defaultdict

def analyze():
    with open("flamegraph.svg", "r") as f:
        content = f.read()
    
    # Matches <title>SymbolName (Samples, Percentage)</title>
    pattern = r"<title>(.*?)\s*\((\d+)\s+samples,\s+([\d.]+)%\)</title>"
    matches = re.findall(pattern, content)
    
    module_samples = defaultdict(int)
    module_percentage = defaultdict(float)
    total_samples = 0
    
    for func, samples, pct in matches:
        samples = int(samples)
        pct = float(pct)
        # Extract module prefix (everything before the first double colon)
        parts = func.split("::")
        module = parts[0] if parts else "unknown"
        module_samples[module] += samples
        total_samples += samples
        
    print("\nCrate/Module Resource Breakdown in Flamegraph:")
    print("-" * 80)
    print(f"{'Crate / Module':<30} | {'Total Samples':<15} | {'Percentage'}")
    print("-" * 80)
    
    sorted_modules = sorted(module_samples.items(), key=lambda x: x[1], reverse=True)
    for module, samples in sorted_modules[:30]:
        pct = (samples / total_samples) * 100 if total_samples > 0 else 0
        print(f"{module:<30} | {samples:<15} | {pct:.2f}%")

if __name__ == "__main__":
    analyze()
