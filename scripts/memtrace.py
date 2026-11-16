#!/usr/bin/env python3
# SPDX-License-Identifier: GPL-3.0-or-later
"""Prints a `run mem` run's watched bytes as a timeline, one-sample
flickers dropped (capture.md 10.14.3), with names for the bytes:
`--name E1B0=FFx,E1B1=FFy`. Used for capture.md 10.41.

    python scripts/memtrace.py RUNS.jsonl [--run N] [--name A=NAME,..] [--only A,B]
"""
import argparse
import json
import os
import sys

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from adjcheck import Timeline  # noqa: E402


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("runs")
    ap.add_argument("--run", type=int, default=None)
    ap.add_argument("--name", default="")
    ap.add_argument("--only", default="")
    a = ap.parse_args()
    names = dict(kv.split("=") for kv in a.name.split(",") if kv)
    only = set(a.only.split(",")) if a.only else None
    for line in open(a.runs, encoding="utf-8"):
        run = json.loads(line)
        if a.run is not None and run["run"] != a.run:
            continue
        tl = Timeline(run)
        print(f"run {run['run']}: start " + " ".join(f"{names.get(k, k)}={v}" for k, v in tl.start.items()))
        ev = sorted((t, k, v) for k, cs in tl.ch.items() for t, v in cs if only is None or k in only)
        for t, k, v in ev:
            print(f"  {t:>10} {names.get(k, k)} {v}")


if __name__ == "__main__":
    main()
