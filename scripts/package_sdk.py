# SPDX-License-Identifier: GPL-3.0-or-later
"""Package a built native SDK, not a frontend. Run from the repository root."""
import argparse
import hashlib
import json
from pathlib import Path
import shutil
import subprocess
import tomllib


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--target", required=True, help="Rust target triple used to build the library")
    parser.add_argument("--profile", choices=("debug", "release"), default="release")
    parser.add_argument("--notices", type=Path, required=True, help="Generated cargo-about HTML report")
    parser.add_argument("--output", type=Path, default=Path("target/sdk"))
    parser.add_argument("--allow-dirty", action="store_true", help="Development only; mark the archive as not matching its source revision")
    args = parser.parse_args()
    dirty = bool(subprocess.check_output(["git", "status", "--porcelain", "--untracked-files=normal"], text=True).strip())
    if dirty and not args.allow_dirty:
        parser.error("source tree is dirty; commit/audit it before packaging, or explicitly use --allow-dirty for local testing")
    # Target is used as a directory/filename, never accept traversal or shell fragments.
    if not args.target or any(c not in "abcdefghijklmnopqrstuvwxyz0123456789_-" for c in args.target):
        parser.error("invalid target triple")
    version = tomllib.loads(Path("Cargo.toml").read_text(encoding="utf-8"))["workspace"]["package"]["version"]
    build = Path("target") / args.target / args.profile
    name = f"openbuds-sdk-{version}-{args.target}"
    output = args.output / name
    if output.exists():
        shutil.rmtree(output)
    (output / "include").mkdir(parents=True)
    (output / "lib").mkdir()
    shutil.copy2("include/openbuds.h", output / "include")
    if "windows-msvc" in args.target:
        (output / "bin").mkdir()
        shutil.copy2(build / "openbuds.dll", output / "bin")
        shutil.copy2(build / "openbuds.dll.lib", output / "lib")
    else:
        library = "libopenbuds.dylib" if "apple-darwin" in args.target else "libopenbuds.so"
        shutil.copy2(build / library, output / "lib")
    for file in ("LICENSE", "THIRD_PARTY_NOTICES.md"):
        shutil.copy2(file, output)
    shutil.copy2(args.notices, output / "THIRD_PARTY_LICENSES.html")
    shutil.copy2("docs/sdk-package.md", output / "README.md")
    shutil.copytree("examples/c", output / "examples/c")
    shutil.copytree("examples/python", output / "examples/python", ignore=shutil.ignore_patterns("__pycache__"))
    revision = subprocess.check_output(["git", "rev-parse", "HEAD"], text=True).strip()
    manifest = {"sdk_version": version, "abi_version": 1, "target": args.target, "build_profile": args.profile,
                "git_revision": revision, "dirty": dirty, "license": "GPL-3.0-or-later",
                "source": f"https://github.com/khanra17/openbuds-air7/tree/{revision}"}
    (output / "manifest.json").write_text(json.dumps(manifest, indent=2) + "\n", encoding="utf-8")
    archive = Path(shutil.make_archive(str(output), "zip", output.parent, output.name))
    digest = hashlib.sha256(archive.read_bytes()).hexdigest()
    archive.with_suffix(".zip.sha256").write_text(f"{digest}  {archive.name}\n", encoding="ascii")
    print(output.as_posix())


if __name__ == "__main__":
    main()
