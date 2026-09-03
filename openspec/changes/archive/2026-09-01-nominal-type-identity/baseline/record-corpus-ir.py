#!/usr/bin/env python3
"""Records the NX IR the example corpus generates, as a baseline this change must preserve.

Each example is run through `nx codegen --target nx-ir`. Examples that generate contribute their
IR with the volatile fields stripped — spans, node ids, slots, the program fingerprint, and the
embedded source, all of which legitimately differ between two spellings of the same program.
Examples that do not generate contribute their diagnostic codes instead, so a change that turns a
diagnostic into a different one is as visible as a change to the IR.

Usage:
    python3 record-corpus-ir.py [--check] [--out corpus-ir.json]
"""

import argparse
import json
import pathlib
import re
import subprocess
import sys
import tempfile

REPO = pathlib.Path(__file__).resolve().parents[4]
VOLATILE = {"start", "end", "id", "slot", "programFingerprint", "source", "identity"}
DIAGNOSTIC = re.compile(r"^(error|warning) ([^:]+):(\d+):(\d+): (.*)$")


def strip_volatile(value):
    if isinstance(value, dict):
        return {k: strip_volatile(v) for k, v in value.items() if k not in VOLATILE}
    if isinstance(value, list):
        return [strip_volatile(item) for item in value]
    return value


def record_example(example: pathlib.Path) -> dict:
    with tempfile.TemporaryDirectory() as out_dir:
        result = subprocess.run(
            [
                "cargo", "run", "--quiet", "-p", "nx-cli", "--",
                "codegen", str(example), "--target", "nx-ir", "--output", out_dir,
            ],
            cwd=REPO,
            capture_output=True,
            text=True,
        )
        combined = result.stdout + result.stderr
        diagnostics = [
            {"severity": m.group(1), "message": m.group(5)}
            for line in combined.splitlines()
            if (m := DIAGNOSTIC.match(line.strip()))
        ]
        emitted = {}
        for path in sorted(pathlib.Path(out_dir).glob("*.json")):
            emitted[path.name] = strip_volatile(json.loads(path.read_text()))

    return {"diagnostics": diagnostics, "ir": emitted}


def record_corpus() -> dict:
    examples = sorted(
        str(path.relative_to(REPO)) for path in (REPO / "examples").rglob("*.nx")
    )
    return {example: record_example(REPO / example) for example in examples}


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--check", action="store_true", help="compare against the recorded baseline")
    parser.add_argument("--out", default=str(pathlib.Path(__file__).with_name("corpus-ir.json")))
    args = parser.parse_args()

    recorded = record_corpus()
    out = pathlib.Path(args.out)

    if not args.check:
        out.write_text(json.dumps(recorded, indent=2, sort_keys=True) + "\n")
        print(f"recorded {len(recorded)} examples to {out}")
        return 0

    baseline = json.loads(out.read_text())
    drift = [name for name in sorted(set(baseline) | set(recorded))
             if baseline.get(name) != recorded.get(name)]
    if drift:
        print("corpus IR differs from the baseline for:")
        for name in drift:
            print(f"  {name}")
        return 1
    print(f"corpus IR matches the baseline for all {len(recorded)} examples")
    return 0


if __name__ == "__main__":
    sys.exit(main())
