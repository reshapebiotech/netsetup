#!/usr/bin/env python3
"""Collect dependency license texts for the release bundle."""

import json
from pathlib import Path
import subprocess
import sys

repo = Path(__file__).resolve().parents[2]
metadata = json.loads(subprocess.check_output([
    "cargo", "metadata", "--locked", "--format-version=1",
    "--filter-platform=aarch64-unknown-linux-gnu",
], cwd=repo))
packages = []
for package in metadata["packages"]:
    if package["source"] is not None:
        packages.append((
            f'Rust: {package["name"]} {package["version"]}',
            Path(package["manifest_path"]).parent,
            package.get("license"),
            package.get("license_file"),
        ))
modules = repo / "web/node_modules"
if not modules.is_dir():
    sys.exit("Install web dependencies before packaging a release")
for manifest in sorted([*modules.glob("*/package.json"), *modules.glob("@*/*/package.json")]):
    package = json.loads(manifest.read_text())
    packages.append((
        f'JavaScript: {package.get("name", manifest.parent.name)} {package.get("version", "")}',
        manifest.parent, package.get("license"), None,
    ))

with Path(sys.argv[1]).open("w") as output:
    output.write("Third-party dependency licenses\n\n")
    for name, root, license_id, license_file in sorted(packages):
        output.write(f"{'=' * 72}\n{name}\nLicense: {license_id or 'See package notices'}\n\n")
        files = set()
        for entry in root.iterdir():
            if entry.name.lower().startswith(("license", "licence", "copying", "copyright", "notice")):
                files.update([entry] if entry.is_file() else (p for p in entry.rglob("*") if p.is_file()))
        if license_file:
            files.add(root / license_file)
        if not files:
            # Some packages keep their license notice in the README.
            files.update(p for p in root.iterdir() if p.is_file() and p.name.lower().startswith("readme"))
        if not files:
            sys.exit(f"No license text or README found for {name}")
        for file in sorted(files):
            output.write(f"--- {file.relative_to(root)} ---\n")
            output.write(file.read_text(errors="replace").rstrip() + "\n\n")
