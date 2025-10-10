import json
from wasm import decode_module

def extract_dwarf_info(wasm_file):
    with open(wasm_file, 'rb') as f:
        module = decode_module(f.read())
    
    dwarf_info = {}
    for section in module.sections:
        if section.name.startswith(b'.debug_'):
            dwarf_info[section.name.decode()] = section.data.hex()
    
    return dwarf_info

if __name__ == "__main__":
    import sys
    info = extract_dwarf_info(sys.argv[1])
    print(json.dumps(info, indent=2))
