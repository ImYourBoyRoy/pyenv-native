# ./python-package/src/pyenv_native_bootstrap/platforms.py
"""Platform helpers for selecting and naming pyenv-native release bundles."""

from __future__ import annotations

import platform
import sys
from dataclasses import dataclass


@dataclass(frozen=True)
class PlatformTarget:
    """Normalized OS and architecture labels used by release bundle naming."""

    operating_system: str
    architecture: str

    @property
    def bundle_stem(self) -> str:
        return f"pyenv-native-{self.operating_system}-{self.architecture}"

    @property
    def bundle_extension(self) -> str:
        return ".zip" if self.operating_system == "windows" else ".tar.gz"

    @property
    def bundle_file_name(self) -> str:
        return f"{self.bundle_stem}{self.bundle_extension}"


def normalize_operating_system(value: str | None = None) -> str:
    """Return a stable operating-system label for release asset naming."""

    candidate = (value or sys.platform).strip().lower()
    if candidate.startswith("win"):
        return "windows"
    if candidate.startswith("linux"):
        return "linux"
    if candidate in {"darwin", "macos", "mac", "osx"}:
        return "macos"
    return candidate


def normalize_architecture(value: str | None = None) -> str:
    """Return a stable CPU architecture label for release asset naming."""

    candidate = (value or platform.machine()).strip().lower()
    if candidate in {"amd64", "x86_64", "x64"}:
        return "x64"
    if candidate in {"arm64", "aarch64"}:
        return "arm64"
    if candidate in {"x86", "i386", "i686"}:
        return "x86"
    return candidate


def current_target() -> PlatformTarget:
    """Return the normalized target for the current Python process."""

    return PlatformTarget(
        operating_system=normalize_operating_system(),
        architecture=normalize_architecture(),
    )
