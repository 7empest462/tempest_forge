import re

def analyze():
    with open("flamegraph.svg", "r") as f:
        content = f.read()
    
    # We want to match: <g><title>(.*?)</title><rect x=".*?" y="(\d+)" width=".*?" height=".*?" fill=".*?" fg:x="(\d+)" fg:w="(\d+)"
    pattern = r'<g><title>(.*?)</title><rect[^>]*?y="(\d+)"[^>]*?fg:x="(\d+)" fg:w="(\d+)"'
    matches = re.findall(pattern, content)
    
    # Let's create a list of nodes: (name, y, x, w)
    nodes = []
    for title, y_str, x_str, w_str in matches:
        # Title might have HTML entities
        title_clean = title.replace("&lt;", "<").replace("&gt;", ">").replace("&amp;", "&").replace("&quot;", '"')
        nodes.append({
            'name': title_clean,
            'y': int(y_str),
            'x': int(x_str),
            'w': int(w_str)
        })
        
    print(f"Total parsed nodes: {len(nodes)}")
    
    # Find all nodes with name containing "build_neighbors" or "mikktspace"
    target_nodes = [n for n in nodes if "build_neighbors" in n['name']]
    print(f"Found {len(target_nodes)} build_neighbors nodes")
    
    # Trace back each target node
    for idx, target in enumerate(target_nodes[:10]):
        print(f"\nTrace {idx + 1} for target: {target['name']} (samples: {target['w']}, x: {target['x']}, y: {target['y']})")
        current = target
        path = [current['name']]
        
        while True:
            # Look for caller at y_parent = y + 16
            y_parent = current['y'] + 16
            parent = None
            for n in nodes:
                if n['y'] == y_parent:
                    # Check if parent encloses current
                    if n['x'] <= current['x'] and (n['x'] + n['w']) >= (current['x'] + current['w']):
                        parent = n
                        break
            if parent is None:
                # Try close Y just in case of different rendering offset
                for n in nodes:
                    if abs(n['y'] - y_parent) < 5:
                        if n['x'] <= current['x'] and (n['x'] + n['w']) >= (current['x'] + current['w']):
                            parent = n
                            break
            if parent:
                path.append(parent['name'])
                current = parent
            else:
                break
                
        # Print path from root to target
        print(" -> ".join(reversed(path[-15:]))) # print top 15 callers

if __name__ == "__main__":
    analyze()
