import struct
import json
import sys

def inspect_glb_nodes(filepath):
    print(f"=== Nodes in {filepath} ===")
    with open(filepath, 'rb') as f:
        magic = f.read(4)
        if magic != b'glTF':
            print("Not a valid glTF/GLB file")
            return
        version = struct.unpack('<I', f.read(4))[0]
        length = struct.unpack('<I', f.read(4))[0]
        
        chunk_length = struct.unpack('<I', f.read(4))[0]
        chunk_type = f.read(4)
        json_data = f.read(chunk_length)
        gltf = json.loads(json_data.decode('utf-8'))
        
        nodes = gltf.get('nodes', [])
        for idx, node in enumerate(nodes):
            name = node.get('name', f'Node{idx}')
            children = node.get('children', [])
            mesh = node.get('mesh', None)
            skin = node.get('skin', None)
            print(f"Node {idx}: name='{name}' children={children} mesh={mesh} skin={skin}")

inspect_glb_nodes("assets/059_Triangaroo_Art.glb")
inspect_glb_nodes("assets/060_Polypug_Art.glb")
