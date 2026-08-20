#!/usr/bin/env python3
"""Startup micro-benchmark for the yaks CLI.

Compares the Rust release binary against the Python `yaks` (if on PATH) on a
couple of commands. Prefers `hyperfine` for rigorous stats when available;
otherwise falls back to a perf_counter loop. Run from inside a .yaks/ project
(defaults to timing against whatever herd the cwd resolves to).
"""
from __future__ import annotations

import shutil
import statistics
import subprocess
import sys
import time
from pathlib import Path

REPO = Path(__file__).resolve().parent.parent
RUST = REPO / "target" / "release" / "yaks"


def bench(cmd: list[str], n: int = 25) -> tuple[float, float, float]:
    for _ in range(3):  # warm caches
        subprocess.run(cmd, stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL)
    samples = []
    for _ in range(n):
        start = time.perf_counter()
        subprocess.run(cmd, stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL)
        samples.append((time.perf_counter() - start) * 1000)
    return statistics.median(samples), min(samples), max(samples)


def main() -> None:
    if not RUST.exists():
        sys.exit(f"build first: cargo build --release  (missing {RUST})")
    targets = [
        ("rust  yaks --help", [str(RUST), "--help"]),
        ("rust  yaks list", [str(RUST), "list"]),
    ]
    py = shutil.which("yaks")
    if py:
        targets.append(("python yaks list", [py, "list"]))
    if shutil.which("hyperfine"):
        print(f"(hyperfine available — for rigorous stats: hyperfine -N '{RUST} list')\n")
    print(f"{'command':22s} {'median':>9} {'min':>8} {'max':>8}   (ms)")
    for label, cmd in targets:
        med, lo, hi = bench(cmd)
        print(f"{label:22s} {med:9.2f} {lo:8.2f} {hi:8.2f}")


if __name__ == "__main__":
    main()
