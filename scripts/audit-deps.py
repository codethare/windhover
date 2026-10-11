#!/usr/bin/env python3
"""Verify the locked dependency graph against the RustSec advisory database.

Usage: python3 scripts/audit-deps.py
       RUSTSEC_DB=/path/to/advisory-db python3 scripts/audit-deps.py

Clones the advisory database when it is missing. Exits non-zero when any
advisory applies to a locked version.
"""

import json
import os
import re
import subprocess
import sys
import tomllib
from pathlib import Path

DB = Path(os.environ.get("RUSTSEC_DB", "/tmp/advisory-db"))
REPO = "https://github.com/RustSec/advisory-db"


def ensure_db() -> None:
    if (DB / "crates").is_dir():
        return
    print(f"cloning {REPO} into {DB}")
    subprocess.run(["git", "clone", "--depth", "1", "-q", REPO, str(DB)], check=True)


def parse_version(text: str) -> tuple[int, ...]:
    core = text.strip().split("-")[0].split("+")[0]
    parts = [int(p) if p.isdigit() else 0 for p in core.split(".")]
    return tuple((parts + [0, 0, 0])[:3])


def bump(version: tuple[int, ...], index: int) -> tuple[int, ...]:
    out = list(version)
    out[index] += 1
    for i in range(index + 1, 3):
        out[i] = 0
    return tuple(out)


def matches(version: tuple[int, ...], spec: str) -> bool:
    spec = spec.strip()
    if not spec or spec == "*":
        return True
    if "," in spec:
        return all(matches(version, part) for part in spec.split(","))
    if spec.startswith(">="):
        return version >= parse_version(spec[2:])
    if spec.startswith("<="):
        return version <= parse_version(spec[2:])
    if spec.startswith(">"):
        return version > parse_version(spec[1:])
    if spec.startswith("<"):
        return version < parse_version(spec[1:])
    if spec.startswith("^"):
        base = parse_version(spec[1:])
        index = 0 if base[0] > 0 else (1 if base[1] > 0 else 2)
        return base <= version < bump(base, index)
    if spec.startswith("~"):
        base = parse_version(spec[1:])
        return base <= version < bump(base, 1)
    if spec.startswith("="):
        return version == parse_version(spec[1:])
    return version == parse_version(spec)


def advisory_block(path: Path) -> dict:
    text = path.read_text(encoding="utf-8")
    match = re.search(r"```toml\n(.*?)```", text, re.DOTALL)
    return tomllib.loads(match.group(1)) if match else {}


def audit() -> int:
    ensure_db()
    metadata = json.loads(
        subprocess.run(
            ["cargo", "metadata", "--format-version", "1", "--all-features"],
            check=True,
            capture_output=True,
            text=True,
        ).stdout
    )

    # A set of (name, version) pairs, not a name-keyed mapping: a crate can appear
    # in the graph at two versions at once (zbus pulling quick-xml 0.38 while
    # wayland-scanner pulls 0.41, for instance), and collapsing those hides the
    # vulnerable one behind the patched one.
    packages = sorted(
        {(p["name"], p["version"]) for p in metadata["packages"] if p["source"]}
    )

    applicable = []
    checked = 0
    for name, version in packages:
        directory = DB / "crates" / name
        if not directory.is_dir():
            continue
        for path in sorted(directory.glob("RUSTSEC-*.md")):
            checked += 1
            advisory = advisory_block(path)
            patched = advisory.get("versions", {}).get("patched", [])
            unaffected = advisory.get("versions", {}).get("unaffected", [])
            current = parse_version(version)
            if any(matches(current, spec) for spec in patched + unaffected):
                continue
            applicable.append(
                (
                    name,
                    version,
                    advisory.get("id", path.stem),
                    advisory.get("informational", ""),
                    patched,
                )
            )

    print(f"packages in graph: {len(packages)}")
    print(f"advisory records examined: {checked}")
    if not applicable:
        print("no applicable advisories")
        return 0

    print("\nAPPLICABLE ADVISORIES:")
    for name, version, advisory_id, informational, patched in applicable:
        kind = f" [{informational}]" if informational else ""
        print(f"  {name} {version}: {advisory_id}{kind}")
        print(f"    patched by: {', '.join(patched) or 'no patched release'}")
    return 1


if __name__ == "__main__":
    sys.exit(audit())
