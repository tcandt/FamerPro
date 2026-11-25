#!/usr/bin/env python3
"""
FarmerPro Phase 2.1 — 100-Cycle Hardware Benchmark Runner
Target: Samsung Galaxy S7 (SM-G930F) via ADB TCP/IP LAN (192.168.1.168:5555)

Executes 100 physical hardware interaction cycles:
1. UI hierarchy acquisition from physical phone
2. Target element resolution (com.android.calculator2 keys)
3. Dynamic safe tap point calculation with 15% inner padding margin (Rule 5)
4. Real physical action dispatch (input tap <x> <y>)
5. Telemetry recording & receipt verification

Outputs:
- artifacts/phase2_1_evidence/physical_benchmark_raw.csv
- artifacts/phase2_1_evidence/physical_benchmark_raw.json
- artifacts/phase2_1_evidence/physical_benchmark_summary.json
- artifacts/phase2_1_evidence/execution_receipts.jsonl
"""

import csv
import json
import os
import random
import re
import subprocess
import sys
import time
import xml.etree.ElementTree as ET
from datetime import datetime, timezone

DEVICE_SERIAL = "192.168.1.168:5555"
CALC_PKG = "com.android.calculator2"
CYCLES = 100
MARGIN_RATIO = 0.15

OUTPUT_DIR = os.path.join("artifacts", "phase2_1_evidence")
os.makedirs(OUTPUT_DIR, exist_ok=True)

CSV_PATH = os.path.join(OUTPUT_DIR, "physical_benchmark_raw.csv")
JSON_PATH = os.path.join(OUTPUT_DIR, "physical_benchmark_raw.json")
SUMMARY_PATH = os.path.join(OUTPUT_DIR, "physical_benchmark_summary.json")
RECEIPTS_PATH = os.path.join(OUTPUT_DIR, "execution_receipts.jsonl")

# 14 distinct calculator buttons to rotate through
BENCHMARK_TARGETS = [
    {"id": f"{CALC_PKG}:id/digit_7", "text": "7", "name": "Digit 7"},
    {"id": f"{CALC_PKG}:id/op_add", "text": "+", "name": "Op Add"},
    {"id": f"{CALC_PKG}:id/digit_8", "text": "8", "name": "Digit 8"},
    {"id": f"{CALC_PKG}:id/op_mul", "text": "×", "name": "Op Mul"},
    {"id": f"{CALC_PKG}:id/digit_9", "text": "9", "name": "Digit 9"},
    {"id": f"{CALC_PKG}:id/digit_4", "text": "4", "name": "Digit 4"},
    {"id": f"{CALC_PKG}:id/digit_5", "text": "5", "name": "Digit 5"},
    {"id": f"{CALC_PKG}:id/digit_6", "text": "6", "name": "Digit 6"},
    {"id": f"{CALC_PKG}:id/digit_1", "text": "1", "name": "Digit 1"},
    {"id": f"{CALC_PKG}:id/digit_2", "text": "2", "name": "Digit 2"},
    {"id": f"{CALC_PKG}:id/digit_3", "text": "3", "name": "Digit 3"},
    {"id": f"{CALC_PKG}:id/digit_0", "text": "0", "name": "Digit 0"},
    {"id": f"{CALC_PKG}:id/eq", "text": "=", "name": "Equals"},
    {"id": f"{CALC_PKG}:id/del", "text": "DEL", "name": "Delete"},
]


def run_adb(args, timeout=15):
    cmd = ["adb", "-s", DEVICE_SERIAL] + args
    start = time.perf_counter()
    res = subprocess.run(cmd, capture_output=True, text=True, timeout=timeout)
    elapsed_ms = (time.perf_counter() - start) * 1000.0
    if res.returncode != 0:
        raise RuntimeError(f"ADB error ({cmd}): {res.stderr.strip()}")
    return res.stdout, elapsed_ms


def parse_bounds(bounds_str):
    m = re.match(r"\[(\d+),(\d+)\]\[(\d+),(\d+)\]", bounds_str)
    if not m:
        return {"x": 0, "y": 0, "width": 0, "height": 0}
    left, top, right, bottom = map(int, m.groups())
    return {"x": left, "y": top, "width": right - left, "height": bottom - top}


def calculate_safe_point(bounds, margin_ratio=MARGIN_RATIO):
    x, y, w, h = bounds["x"], bounds["y"], bounds["width"], bounds["height"]
    pad_x = w * margin_ratio
    pad_y = h * margin_ratio
    min_x = x + pad_x
    max_x = x + w - pad_x
    min_y = y + pad_y
    max_y = y + h - pad_y
    if min_x > max_x:
        min_x = max_x = x + w / 2.0
    if min_y > max_y:
        min_y = max_y = y + h / 2.0
    safe_x = random.uniform(min_x, max_x)
    safe_y = random.uniform(min_y, max_y)
    return round(safe_x, 1), round(safe_y, 1)


def find_matching_elements(root, target_id, target_text):
    candidates = []
    for elem in root.iter("node"):
        res_id = elem.attrib.get("resource-id", "")
        text = elem.attrib.get("text", "")
        bounds_str = elem.attrib.get("bounds", "")
        clickable = elem.attrib.get("clickable", "false") == "true"
        bounds = parse_bounds(bounds_str)

        match = False
        if target_id and res_id == target_id:
            match = True
        elif target_text and text == target_text:
            match = True

        if match:
            candidates.append(
                {
                    "resource_id": res_id,
                    "text": text,
                    "class_name": elem.attrib.get("class", ""),
                    "package": elem.attrib.get("package", ""),
                    "clickable": clickable,
                    "bounds": bounds,
                }
            )
    return candidates


def main():
    print(f"=== FARMERPRO PHASE 2.1 — 100-CYCLE HARDWARE BENCHMARK ===")
    print(f"Device: {DEVICE_SERIAL}")
    print(f"Package: {CALC_PKG}")
    print(f"Total Cycles: {CYCLES}")
    print(f"Safe Margin Policy: {int(MARGIN_RATIO * 100)}% inner padding\n")

    # Step 0: Ensure device is online & bring calculator to foreground
    try:
        run_adb(["shell", "monkey", "-p", CALC_PKG, "-c", "android.intent.category.LAUNCHER", "1"])
        time.sleep(1.0)
        print("[INIT] Calculator launched and focused.\n")
    except Exception as e:
        print(f"[FATAL] Failed to launch calculator: {e}")
        sys.exit(1)

    raw_records = []
    receipts = []

    total_start = time.perf_counter()
    success_count = 0
    first_pass_count = 0
    retry_total = 0
    fail_count = 0

    dump_latencies = []
    parse_latencies = []
    plan_latencies = []
    dispatch_latencies = []
    cycle_latencies = []

    csv_fieldnames = [
        "cycle_id",
        "timestamp_iso",
        "target_name",
        "target_id",
        "strategy_used",
        "dump_ms",
        "parse_ms",
        "plan_ms",
        "dispatch_ms",
        "total_ms",
        "bounds_x",
        "bounds_y",
        "bounds_w",
        "bounds_h",
        "tap_x",
        "tap_y",
        "margin_applied",
        "is_first_pass",
        "retry_count",
        "status",
        "receipt_id",
    ]

    csv_file = open(CSV_PATH, "w", newline="", encoding="utf-8")
    csv_writer = csv.DictWriter(csv_file, fieldnames=csv_fieldnames)
    csv_writer.writeheader()

    receipts_file = open(RECEIPTS_PATH, "w", encoding="utf-8")

    try:
        for i in range(1, CYCLES + 1):
            cycle_start = time.perf_counter()
            target_spec = BENCHMARK_TARGETS[(i - 1) % len(BENCHMARK_TARGETS)]
            now_iso = datetime.now(timezone.utc).isoformat()
            receipt_id = f"rcpt_p21_{int(time.time()*1000)}_{i:04d}"

            retries = 0
            cycle_success = False
            last_err = None

            dump_ms = 0.0
            parse_ms = 0.0
            plan_ms = 0.0
            dispatch_ms = 0.0
            tap_x, tap_y = 0.0, 0.0
            bounds = {"x": 0, "y": 0, "width": 0, "height": 0}

            # Up to 2 retries per cycle if ADB hiccups
            for attempt in range(3):
                try:
                    # 1. Hardware Dump
                    dump_out, d_ms = run_adb(["shell", "uiautomator", "dump", "/sdcard/ui_bench.xml"])
                    cat_out, c_ms = run_adb(["shell", "cat", "/sdcard/ui_bench.xml"])
                    dump_ms = d_ms + c_ms

                    # 2. Parse XML Hierarchy
                    p_start = time.perf_counter()
                    root = ET.fromstring(cat_out)
                    parse_ms = (time.perf_counter() - p_start) * 1000.0

                    # 3. Resolve Target with fp-element rules
                    pl_start = time.perf_counter()
                    candidates = find_matching_elements(root, target_spec["id"], target_spec["text"])
                    if not candidates:
                        raise ValueError(f"Target '{target_spec['name']}' not found in hierarchy")

                    chosen = candidates[0]
                    bounds = chosen["bounds"]
                    tap_x, tap_y = calculate_safe_point(bounds, MARGIN_RATIO)
                    plan_ms = (time.perf_counter() - pl_start) * 1000.0

                    # 4. Dispatch Physical Touch
                    _, dispatch_ms = run_adb(["shell", "input", "tap", str(int(tap_x)), str(int(tap_y))])

                    cycle_success = True
                    break
                except Exception as ex:
                    last_err = str(ex)
                    retries += 1
                    retry_total += 1
                    time.sleep(0.3)

            total_cycle_ms = (time.perf_counter() - cycle_start) * 1000.0

            is_first_pass = (retries == 0 and cycle_success)
            if cycle_success:
                success_count += 1
                if is_first_pass:
                    first_pass_count += 1
                status = "SUCCESS"
            else:
                fail_count += 1
                status = f"FAILED: {last_err}"

            dump_latencies.append(dump_ms)
            parse_latencies.append(parse_ms)
            plan_latencies.append(plan_ms)
            dispatch_latencies.append(dispatch_ms)
            cycle_latencies.append(total_cycle_ms)

            record = {
                "cycle_id": i,
                "timestamp_iso": now_iso,
                "target_name": target_spec["name"],
                "target_id": target_spec["id"],
                "strategy_used": "Native",
                "dump_ms": round(dump_ms, 2),
                "parse_ms": round(parse_ms, 2),
                "plan_ms": round(plan_ms, 2),
                "dispatch_ms": round(dispatch_ms, 2),
                "total_ms": round(total_cycle_ms, 2),
                "bounds_x": bounds["x"],
                "bounds_y": bounds["y"],
                "bounds_w": bounds["width"],
                "bounds_h": bounds["height"],
                "tap_x": tap_x,
                "tap_y": tap_y,
                "margin_applied": MARGIN_RATIO,
                "is_first_pass": is_first_pass,
                "retry_count": retries,
                "status": status,
                "receipt_id": receipt_id,
            }

            raw_records.append(record)
            csv_writer.writerow(record)
            csv_file.flush()

            # Write Receipt JSONL
            receipt_entry = {
                "receipt_id": receipt_id,
                "cycle": i,
                "device": DEVICE_SERIAL,
                "timestamp": now_iso,
                "target": target_spec,
                "plan": {
                    "mode": "DynamicBoundsTap",
                    "target_bounds": bounds,
                    "safe_tap_point": {"x": tap_x, "y": tap_y},
                    "margin_ratio": MARGIN_RATIO,
                },
                "execution": {
                    "dump_ms": round(dump_ms, 2),
                    "dispatch_ms": round(dispatch_ms, 2),
                    "total_ms": round(total_cycle_ms, 2),
                    "success": cycle_success,
                    "retries": retries,
                },
            }
            receipts_file.write(json.dumps(receipt_entry) + "\n")
            receipts_file.flush()

            if i % 10 == 0 or i == CYCLES:
                avg_so_far = sum(cycle_latencies) / len(cycle_latencies)
                print(
                    f"[{i:03d}/{CYCLES}] Target: {target_spec['name']:<10} | "
                    f"Tap: ({int(tap_x):>3}, {int(tap_y):>3}) | "
                    f"Dump: {dump_ms:>6.1f}ms | Tap: {dispatch_ms:>5.1f}ms | "
                    f"Total: {total_cycle_ms:>6.1f}ms | Status: {status}"
                )

    finally:
        csv_file.close()
        receipts_file.close()

    total_wall_time_s = time.perf_counter() - total_start

    # Save full raw JSON
    with open(JSON_PATH, "w", encoding="utf-8") as f:
        json.dump(raw_records, f, indent=2)

    def stats(arr):
        if not arr:
            return {"min": 0, "max": 0, "avg": 0, "p95": 0}
        s = sorted(arr)
        p95_idx = int(len(s) * 0.95)
        return {
            "min": round(min(s), 2),
            "max": round(max(s), 2),
            "avg": round(sum(s) / len(s), 2),
            "p95": round(s[min(p95_idx, len(s) - 1)], 2),
        }

    summary = {
        "benchmark_name": "FarmerPro Phase 2.1 - 100-Cycle Hardware Physical Benchmark",
        "device_serial": DEVICE_SERIAL,
        "device_model": "Samsung Galaxy S7 (SM-G930F)",
        "android_version": "10 (LineageOS 17.1)",
        "transport": "ADB TCP/IP LAN",
        "package_tested": CALC_PKG,
        "total_cycles_planned": CYCLES,
        "total_cycles_executed": len(raw_records),
        "successful_cycles": success_count,
        "failed_cycles": fail_count,
        "success_rate_percent": round((success_count / CYCLES) * 100, 2),
        "first_pass_success_percent": round((first_pass_count / CYCLES) * 100, 2),
        "retry_rate_percent": round((retry_total / CYCLES) * 100, 2),
        "total_duration_seconds": round(total_wall_time_s, 2),
        "safe_margin_policy": f"{int(MARGIN_RATIO*100)}% inner padding",
        "telemetry_stats": {
            "ui_tree_dump_ms": stats(dump_latencies),
            "xml_parse_ms": stats(parse_latencies),
            "safe_plan_ms": stats(plan_latencies),
            "action_dispatch_ms": stats(dispatch_latencies),
            "total_cycle_ms": stats(cycle_latencies),
        },
        "artifacts": {
            "csv": CSV_PATH,
            "json_raw": JSON_PATH,
            "receipts_jsonl": RECEIPTS_PATH,
        },
    }

    with open(SUMMARY_PATH, "w", encoding="utf-8") as f:
        json.dump(summary, f, indent=2)

    print("\n========================================================")
    print("           BENCHMARK RUN COMPLETED                      ")
    print("========================================================")
    print(f"Total Cycles Completed:   {len(raw_records)} / {CYCLES}")
    print(f"Physical Success Rate:    {summary['success_rate_percent']}% ({success_count}/{CYCLES})")
    print(f"First-Pass Success Rate:  {summary['first_pass_success_percent']}%")
    print(f"Retry Rate:               {summary['retry_rate_percent']}% ({retry_total} retries)")
    print(f"Total Run Duration:       {summary['total_duration_seconds']}s")
    print(f"Avg UI Dump Latency:      {summary['telemetry_stats']['ui_tree_dump_ms']['avg']}ms")
    print(f"Avg Action Dispatch:      {summary['telemetry_stats']['action_dispatch_ms']['avg']}ms")
    print(f"Avg Cycle Total:          {summary['telemetry_stats']['total_cycle_ms']['avg']}ms")
    print(f"P95 Cycle Total:          {summary['telemetry_stats']['total_cycle_ms']['p95']}ms")
    print("Evidence written to:")
    print(f" - CSV:      {CSV_PATH}")
    print(f" - JSON:     {JSON_PATH}")
    print(f" - Summary:  {SUMMARY_PATH}")
    print(f" - Receipts: {RECEIPTS_PATH}")
    print("========================================================")


if __name__ == "__main__":
    main()
