#!/usr/bin/env python3
"""Create local wheel fixtures for the cross-platform managed-venv test.

The 1.0.0 wheel is intentionally limited to Python below the requested target
minor, while 2.0.0 requires that target minor or newer. A migration across
that boundary therefore has to reject the exact 1.0.0 pin and recover with
the lower-bound compatibility policy without using the public package index.
"""

from __future__ import annotations

import base64
import hashlib
import re
import sys
import zipfile
from pathlib import Path


DIST_NAME = "migration-compat-demo"
MODULE_NAME = "migration_compat_demo"


def wheel_bytes(version: str, requires_python: str) -> dict[str, bytes]:
    dist_info = f"migration_compat_demo-{version}.dist-info"
    return {
        f"{MODULE_NAME}/__init__.py": (
            f"__version__ = {version!r}\n"
        ).encode(),
        f"{dist_info}/METADATA": (
            "Metadata-Version: 2.1\n"
            f"Name: {DIST_NAME}\n"
            f"Version: {version}\n"
            f"Requires-Python: {requires_python}\n"
            "Summary: local migration compatibility fixture\n"
        ).encode(),
        f"{dist_info}/WHEEL": (
            "Wheel-Version: 1.0\n"
            "Generator: pyenv-native migration test\n"
            "Root-Is-Purelib: true\n"
            "Tag: py3-none-any\n"
        ).encode(),
    }


def record_line(path: str, payload: bytes) -> str:
    digest = base64.urlsafe_b64encode(hashlib.sha256(payload).digest()).rstrip(b"=")
    return f"{path},sha256={digest.decode()},{len(payload)}\n"


def write_wheel(output_dir: Path, version: str, requires_python: str) -> None:
    files = wheel_bytes(version, requires_python)
    dist_info = f"migration_compat_demo-{version}.dist-info"
    record = "".join(record_line(path, payload) for path, payload in files.items())
    record += f"{dist_info}/RECORD,,\n"
    files[f"{dist_info}/RECORD"] = record.encode()

    filename = f"migration_compat_demo-{version}-py3-none-any.whl"
    with zipfile.ZipFile(output_dir / filename, "w", compression=zipfile.ZIP_DEFLATED) as wheel:
        for path, payload in files.items():
            wheel.writestr(path, payload)


def main() -> int:
    if len(sys.argv) not in {2, 3}:
        print(
            f"usage: {Path(sys.argv[0]).name} OUTPUT_DIR [TARGET_RUNTIME]",
            file=sys.stderr,
        )
        return 2

    output_dir = Path(sys.argv[1]).resolve()
    output_dir.mkdir(parents=True, exist_ok=True)
    target_runtime = sys.argv[2] if len(sys.argv) == 3 else "3.15"
    target_match = re.fullmatch(r"(\d+)\.(\d+)(?:\.\d+)?", target_runtime)
    if target_match is None:
        print(f"invalid target runtime: {target_runtime}", file=sys.stderr)
        return 2
    target_minor = f"{target_match.group(1)}.{target_match.group(2)}"
    write_wheel(output_dir, "1.0.0", f">=3.8,<{target_minor}")
    write_wheel(output_dir, "2.0.0", f">={target_minor}")
    print(output_dir)
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
