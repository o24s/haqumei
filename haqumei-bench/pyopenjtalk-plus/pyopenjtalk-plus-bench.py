import time
import argparse
import json
import platform
import statistics
import multiprocessing
import os
import sys
from contextlib import contextmanager
from importlib.metadata import version
from pathlib import Path

try:
    import pyopenjtalk
except ImportError:
    print("Error: pyopenjtalk is not installed. Please install it to run benchmarks.")
    sys.exit(1)

try:
    import haqumei
except ImportError:
    print("Error: haqumei is not installed. Please build and install it via maturin.")
    sys.exit(1)

ITERATIONS = 5
WARMUP = 2

@contextmanager
def suppress_stderr():
    original_stderr_fd = sys.stderr.fileno()
    saved_stderr_fd = os.dup(original_stderr_fd)

    try:
        devnull = os.open(os.devnull, os.O_WRONLY)
        os.dup2(devnull, original_stderr_fd)
        os.close(devnull)
        yield
    finally:
        os.dup2(saved_stderr_fd, original_stderr_fd)
        os.close(saved_stderr_fd)

def load_data():
    base_dir = os.path.dirname(os.path.abspath(__file__))
    path = os.path.join(base_dir, "../../resources/waganeko.txt")

    if not os.path.exists(path):
        path = "resources/waganeko.txt"

    if not os.path.exists(path):
        print(f"Error: Could not find benchmark text file at {path}")
        sys.exit(1)

    with open(path, "r", encoding="utf-8") as f:
        lines = [line.strip() for line in f if line.strip()]
    return lines

def measure(name, func, data, iterations=ITERATIONS, warmup=WARMUP, results=None):
    print(f"Running: {name:<35} ... ", end="", flush=True)

    with suppress_stderr():
        # Warmup
        for _ in range(warmup):
            func(data)

        times = []
        for _ in range(iterations):
            start = time.perf_counter()
            func(data)
            end = time.perf_counter()
            times.append(end - start)

    mean_time = statistics.mean(times)
    stdev = statistics.stdev(times) if len(times) > 1 else 0.0

    total_chars = sum(len(line) for line in data)
    throughput = total_chars / mean_time

    print(f"{mean_time:.4f} s ± {stdev:.4f} s | {throughput:,.0f} chars/s")
    if results is not None:
        results.append({"name": name, "seconds": times, "mean_seconds": mean_time,
                        "stdev_seconds": stdev, "chars_per_second": throughput})
    return mean_time

def run_pyopenjtalk_single(lines, use_tsqyomi=False):
    for line in lines:
        pyopenjtalk.g2p(line, use_tsqyomi=use_tsqyomi)

def run_openjtalk_single(ojt_instance, lines):
    for line in lines:
        ojt_instance.g2p(line)

def run_openjtalk_batch(ojt_instance, lines):
    ojt_instance.g2p_batch(lines)

def run_haqumei_single(haqumei_instance, lines):
    for line in lines:
        haqumei_instance.g2p(line)

def run_haqumei_batch(haqumei_instance, lines):
    haqumei_instance.g2p_batch(lines)


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("--json", type=Path)
    args = parser.parse_args()
    results = []
    lines = load_data()
    print(f"Loaded {len(lines)} lines ({sum(len(line) for line in lines):,} chars) of text.\n")
    print("-" * 80)
    print(f"{'Benchmark Name':<35} | {'Time (Mean)':<18} | {'Throughput'}")
    print("-" * 80)

    t_py = measure("pyopenjtalk-plus (tsqyomi=False)", run_pyopenjtalk_single, lines, results=results)

    pyopenjtalk.tsqyomi.load_model(onnx_providers=["CPUExecutionProvider"])
    measure("pyopenjtalk-plus (tsqyomi=True)",
            lambda d: run_pyopenjtalk_single(d, use_tsqyomi=True), lines, results=results)

    ojt = haqumei.OpenJTalk()
    hq = haqumei.Haqumei()

    t_ojt = measure("OpenJTalk (Single)", lambda d: run_openjtalk_single(ojt, d), lines, results=results)
    t_ojt_batch = measure("OpenJTalk.g2p_batch", lambda d: run_openjtalk_batch(ojt, d), lines, results=results)

    t_hq = measure("haqumei (Default)", lambda d: run_haqumei_single(hq, d), lines, results=results)

    t_hq_batch = measure("haqumei.g2p_batch (Default)", lambda d: run_haqumei_batch(hq, d), lines, results=results)

    print("-" * 80)
    print("\n[Speedup vs pyopenjtalk-plus]")
    print(f"OpenJTalk (Single):          x{t_py / t_ojt:.2f}")
    print(f"OpenJTalk.g2p_batch:         x{t_py / t_ojt_batch:.2f}")
    print(f"haqumei (Default):           x{t_py / t_hq:.2f}")
    print(f"haqumei.g2p_batch (Default): x{t_py / t_hq_batch:.2f}")
    if args.json:
        args.json.write_text(json.dumps({
            "platform": platform.platform(), "python": platform.python_version(),
            "packages": {name: version(name) for name in ("haqumei", "pyopenjtalk-plus")},
            "cpu_count": os.cpu_count(), "rayon_threads": os.environ.get("RAYON_NUM_THREADS"),
            "tsqyomi_provider": "CPUExecutionProvider",
            "lines": len(lines), "chars": sum(map(len, lines)),
            "iterations": ITERATIONS, "warmup": WARMUP,
            "results": results,
        }, indent=2) + "\n", encoding="utf-8")


if __name__ == "__main__":
    multiprocessing.freeze_support()
    main()
