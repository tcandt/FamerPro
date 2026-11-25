import re
import subprocess
import time
import random
import xml.etree.ElementTree as ET
import sys
import os

DEVICE_SERIAL = "192.168.1.168:5555"
SCRATCH_DIR = r"C:\Users\TINH-NGUYEN\.gemini\antigravity\brain\ee02dcf5-aa3a-4a09-a189-f47c397579e0\scratch"
XML_PATH = os.path.join(SCRATCH_DIR, "settings_hierarchy.xml")
BEFORE_PNG = os.path.join(SCRATCH_DIR, "before.png")
AFTER_PNG = os.path.join(SCRATCH_DIR, "after.png")
AFTER_XML = os.path.join(SCRATCH_DIR, "after_hierarchy.xml")

def run_adb(args):
    cmd = ["adb", "-s", DEVICE_SERIAL] + args
    start = time.perf_counter()
    res = subprocess.run(cmd, stdout=subprocess.PIPE, stderr=subprocess.PIPE, text=True, check=True)
    duration_ms = (time.perf_counter() - start) * 1000
    return res.stdout, duration_ms

def parse_bounds(bounds_str):
    # Format: [108,884][185,917]
    match = re.match(r"\[(\d+),(\d+)\]\[(\d+),(\d+)\]", bounds_str)
    if not match:
        raise ValueError(f"Invalid bounds: {bounds_str}")
    return [int(x) for x in match.groups()]

def calculate_safe_tap(x1, y1, x2, y2, margin_ratio=0.15):
    width = x2 - x1
    height = y2 - y1
    margin_x = int(width * margin_ratio)
    margin_y = int(height * margin_ratio)
    safe_x1 = x1 + margin_x
    safe_x2 = x2 - margin_x
    safe_y1 = y1 + margin_y
    safe_y2 = y2 - margin_y
    if safe_x1 >= safe_x2:
        safe_x = (x1 + x2) // 2
    else:
        safe_x = random.randint(safe_x1, safe_x2)
    if safe_y1 >= safe_y2:
        safe_y = (y1 + y2) // 2
    else:
        safe_y = random.randint(safe_y1, safe_y2)
    return safe_x, safe_y

def main():
    print(f"=== FARMERPRO REAL DEVICE LAN INSPECTION & ACTION POC ===")
    print(f"Target Device: {DEVICE_SERIAL}")
    
    # 1. Parse XML and find element "Display"
    t0 = time.perf_counter()
    tree = ET.parse(XML_PATH)
    root = tree.getroot()
    
    # Test selector 1: Text exact match
    matched_nodes = []
    for node in root.iter('node'):
        if node.get('text') == 'Display':
            matched_nodes.append(node)
            
    print(f"Selector 1: text == 'Display' -> Found {len(matched_nodes)} matches")
    for idx, node in enumerate(matched_nodes):
        print(f"  Match #{idx+1}: class={node.get('class')}, id={node.get('resource-id')}, bounds={node.get('bounds')}")
    
    # Test selector 2: XPath
    xpath_nodes = root.findall(".//node[@text='Display']")
    print(f"Selector 2: XPath .//node[@text='Display'] -> Found {len(xpath_nodes)} matches")

    # Test selector 3: Resource-id + text
    rid_nodes = [n for n in root.iter('node') if n.get('resource-id') == 'android:id/title' and n.get('text') == 'Display']
    print(f"Selector 3: id='android:id/title' AND text='Display' -> Found {len(rid_nodes)} matches")
    
    if not matched_nodes:
        print("ERROR: Element 'Display' not found in UI hierarchy!")
        sys.exit(1)
        
    target_node = matched_nodes[0]
    bounds_str = target_node.get('bounds')
    x1, y1, x2, y2 = parse_bounds(bounds_str)
    print(f"Target Element Bounds: [{x1}, {y1}][{x2}, {y2}] (width={x2-x1}, height={y2-y1})")
    
    # Calculate safe tap point adhering to FarmerPro Rule 5 (Safe Random Tap Policy)
    tap_x, tap_y = calculate_safe_tap(x1, y1, x2, y2, margin_ratio=0.15)
    print(f"Safe Random Tap Coords (with 15% inner margin): ({tap_x}, {tap_y})")
    
    # 2. Dispatch tap via ADB
    print(f"Dispatching tap to {DEVICE_SERIAL}...")
    _, tap_duration_ms = run_adb(["shell", "input", "tap", str(tap_x), str(tap_y)])
    print(f"Tap dispatched successfully! Latency: {tap_duration_ms:.2f} ms")
    
    # 3. Wait for UI transition
    time.sleep(1.2)
    
    # 4. Capture after screenshot and dump new hierarchy
    print("Capturing post-action verification screenshot & hierarchy...")
    # screencap
    start_sc = time.perf_counter()
    res = subprocess.run(["adb", "-s", DEVICE_SERIAL, "exec-out", "screencap", "-p"], stdout=subprocess.PIPE, check=True)
    with open(AFTER_PNG, "wb") as f:
        f.write(res.stdout)
    sc_ms = (time.perf_counter() - start_sc) * 1000
    print(f"After screenshot captured: {AFTER_PNG} ({sc_ms:.2f} ms)")
    
    # dump
    _, dump_ms = run_adb(["shell", "uiautomator", "dump", "/sdcard/after_window.xml"])
    _, pull_ms = run_adb(["pull", "/sdcard/after_window.xml", AFTER_XML])
    print(f"After hierarchy dumped ({dump_ms:.2f} ms) & pulled ({pull_ms:.2f} ms)")
    
    # 5. Verify that we are now inside Display Settings
    after_tree = ET.parse(AFTER_XML)
    after_root = after_tree.getroot()
    
    # Check for display setting items such as "Dark theme", "Brightness level", "Wallpaper", "Screen timeout"
    display_page_items = []
    for node in after_root.iter('node'):
        text = node.get('text', '')
        if any(keyword in text.lower() for keyword in ["dark theme", "brightness", "wallpaper", "screen timeout", "styles"]):
            display_page_items.append((text, node.get('bounds')))
            
    print("\n=== POST-ACTION VERIFICATION RESULTS ===")
    if display_page_items:
        print(f"SUCCESS! Verified Display Settings page opened. Found {len(display_page_items)} matching elements:")
        for text, b in display_page_items:
            print(f"  - '{text}' at {b}")
    else:
        print("WARNING: Could not identify typical Display items in after_hierarchy.xml. Listing all visible texts:")
        for node in after_root.iter('node'):
            txt = node.get('text')
            if txt:
                print(f"  - {txt}")
                
if __name__ == "__main__":
    main()
