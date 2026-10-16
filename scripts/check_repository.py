#!/usr/bin/env python3
# SPDX-License-Identifier: GPL-3.0-or-later
"""Audit tracked publication content. No device access, network calls or state mutations."""
from __future__ import annotations

import pathlib
import posixpath
import re
import struct
import subprocess
import sys

ROOT = pathlib.Path(__file__).resolve().parents[1]
FORBIDDEN_PARTS = {"research", "target", ".pi", "__pycache__", "node_modules"}
FORBIDDEN_SUFFIXES = {".apk", ".ipa", ".btsnoop", ".pcap", ".pcapng", ".log", ".jsonl", ".pyc", ".exe", ".dll", ".pdb", ".so", ".dylib", ".a", ".lib", ".zip", ".gz", ".7z"}
MAC = re.compile(r"(?i)(?<![0-9a-f])(?:[0-9a-f]{2}:){5}[0-9a-f]{2}(?![0-9a-f])")
# Only deliberately synthetic byte-order fixtures, and only inside the test tree.
SYNTHETIC_TEST_MACS = {":".join(f"{i:02X}" for i in values) for values in (range(1, 7), range(6, 0, -1))}
HOME_PATH = re.compile(r"(?i)(?:[a-z]:[/\\]Users[/\\][^\s/\\\"<>]+|/(?:home|Users)/[^\s/\"<>]+)")
SECRET = re.compile(r"(?:gh[pousr]_[A-Za-z0-9_]{30,}|github_pat_[A-Za-z0-9_]{30,}|AKIA[0-9A-Z]{16}|-----BEGIN (?:RSA |OPENSSH |EC )?PRIVATE KEY-----)")
LINK = re.compile(r"!?\[[^\]]*\]\(([^\s)]+)(?:\s+[^)]*)?\)")
TEXT_SUFFIXES = {".rs", ".md", ".toml", ".yml", ".yaml", ".json", ".py", ".svg", ".txt", ".hbs", ".h", ".hpp", ".c", ".cc", ".cpp"}


def text_issues(path: str, text: str) -> list[str]:
    errors = []
    for match in MAC.finditer(text):
        if not (path.startswith("crates/realme-buds/tests/") and match.group().upper() in SYNTHETIC_TEST_MACS):
            errors.append("device-address literal")
    if HOME_PATH.search(text):
        errors.append("developer home-directory path")
    if SECRET.search(text):
        errors.append("credential/private-key pattern")
    return errors


def png_issues(data: bytes) -> list[str]:
    if not data.startswith(b"\x89PNG\r\n\x1a\n"):
        return ["invalid PNG"]
    offset = 8
    while offset + 12 <= len(data):
        size = struct.unpack_from(">I", data, offset)[0]
        kind = data[offset + 4:offset + 8]
        if offset + size + 12 > len(data):
            return ["truncated PNG"]
        if kind in {b"eXIf", b"tEXt", b"iTXt", b"zTXt"}:
            return ["image text/EXIF metadata; strip before publication"]
        offset += size + 12
        if kind == b"IEND":
            return []
    return ["PNG missing end chunk"]


def main() -> int:
    try:
        names = subprocess.check_output(["git", "ls-files", "-z"], cwd=ROOT).decode().split("\0")
    except (subprocess.CalledProcessError, FileNotFoundError):
        print("Run this gate from a Git checkout with publication files staged/tracked.", file=sys.stderr)
        return 2
    paths = [p for p in names if p]
    if not paths:
        print("No tracked publication files; stage the reviewed tree first.", file=sys.stderr)
        return 2
    errors = []
    for name in paths:
        path = pathlib.PurePosixPath(name)
        if any(part in FORBIDDEN_PARTS for part in path.parts) or path.suffix.lower() in FORBIDDEN_SUFFIXES:
            errors.append(f"{name}: forbidden artifact/path")
            continue
        # Audit the actual index/commit candidate, not a different working-tree version.
        data = subprocess.check_output(["git", "show", f":{name}"], cwd=ROOT)
        if path.suffix.lower() == ".png":
            errors.extend(f"{name}: {issue}" for issue in png_issues(data))
        elif path.suffix.lower() in TEXT_SUFFIXES or path.suffix == "":
            try:
                text = data.decode("utf-8")
            except UnicodeDecodeError:
                errors.append(f"{name}: non-UTF-8 publication text")
                continue
            errors.extend(f"{name}: {issue}" for issue in text_issues(name, text))
            if path.suffix.lower() == ".md":
                for target in LINK.findall(text):
                    if target.startswith(("https://", "http://", "mailto:", "#")):
                        continue
                    target = target.split("#", 1)[0]
                    resolved = posixpath.normpath(str(path.parent / target))
                    if target and resolved not in paths and not any(p.startswith(resolved + "/") for p in paths):
                        errors.append(f"{name}: local link not included in publication {target}")
    if errors:
        print("Publication audit failed:")
        for error in errors:
            print(f"- {error}")
        return 1
    print(f"Publication audit passed: {len(paths)} tracked files. Binary pixels still require manual review.")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
