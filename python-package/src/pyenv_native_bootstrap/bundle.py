# ./python-package/src/pyenv_native_bootstrap/bundle.py
"""Bundle metadata, checksum, and download helpers for pyenv-native release assets."""

from __future__ import annotations

import hashlib
import json
import tarfile
import urllib.request
from dataclasses import dataclass
from pathlib import Path
from typing import Optional
from zipfile import ZipFile

from . import __version__


@dataclass(frozen=True)
class BundleManifest:
    """Describes a built release bundle and the scripts it carries."""

    bundle_name: str
    bundle_version: str
    platform: str
    architecture: str
    executable: str
    install_script: str
    uninstall_script: str


def sha256_file(path: Path) -> str:
    """Return the SHA-256 digest for a file."""

    digest = hashlib.sha256()
    with path.open("rb") as handle:
        for chunk in iter(lambda: handle.read(1024 * 1024), b""):
            digest.update(chunk)
    return digest.hexdigest()


def parse_checksum_text(text: str, expected_file_name: Optional[str] = None) -> str:
    """Extract a SHA-256 value from a checksum file or raw string."""

    lines = [line.strip() for line in text.splitlines() if line.strip()]
    if not lines:
        raise ValueError("checksum text did not contain any usable lines")

    for line in lines:
        parts = line.split()
        if len(parts) == 1 and len(parts[0]) == 64:
            return parts[0].lower()
        if len(parts) >= 2 and len(parts[0]) == 64:
            candidate_name = parts[-1].lstrip("*")
            if expected_file_name is None or candidate_name == expected_file_name:
                return parts[0].lower()

    raise ValueError("checksum text did not contain a valid SHA-256 entry")


def is_tar_bundle(bundle_path: Path) -> bool:
    """Return true when a bundle path uses a tar-based archive format."""

    suffixes = [suffix.lower() for suffix in bundle_path.suffixes]
    return suffixes[-2:] == [".tar", ".gz"] or bundle_path.suffix.lower() == ".tgz"


def read_manifest_from_bundle(bundle_path: Path) -> BundleManifest:
    """Read and parse the bundle manifest embedded inside a release archive."""

    if is_tar_bundle(bundle_path):
        with tarfile.open(bundle_path, "r:*") as archive:
            member = next(
                (
                    candidate
                    for candidate in archive.getmembers()
                    if Path(candidate.name).name == "bundle-manifest.json"
                ),
                None,
            )
            if member is None:
                raise FileNotFoundError("bundle-manifest.json was not found in the bundle archive")
            handle = archive.extractfile(member)
            if handle is None:
                raise FileNotFoundError("bundle-manifest.json could not be extracted")
            payload = json.loads(handle.read().decode("utf-8-sig"))
    else:
        with ZipFile(bundle_path, "r") as archive:
            with archive.open("bundle-manifest.json", "r") as handle:
                payload = json.loads(handle.read().decode("utf-8-sig"))

    return BundleManifest(
        bundle_name=payload["bundle_name"],
        bundle_version=payload["bundle_version"],
        platform=payload["platform"],
        architecture=payload["architecture"],
        executable=payload["executable"],
        install_script=payload["install_script"],
        uninstall_script=payload["uninstall_script"],
    )


def read_manifest_from_zip(bundle_path: Path) -> BundleManifest:
    """Backward-compatible wrapper for zip-based bundle manifest reads."""

    return read_manifest_from_bundle(bundle_path)


def download_file(url: str, destination: Path) -> Path:
    """Download a file to a destination path, replacing any prior file."""

    destination.parent.mkdir(parents=True, exist_ok=True)
    request = urllib.request.Request(
        url,
        headers={"User-Agent": f"pyenv-native-bootstrap/{__version__}"},
    )
    with urllib.request.urlopen(request) as response, destination.open("wb") as handle:  # noqa: S310
        handle.write(response.read())
    return destination


def load_checksum_text(
    checksum: Optional[str],
    checksum_path: Optional[Path],
    checksum_url: Optional[str],
    cache_path: Path,
) -> Optional[str]:
    """Load checksum content from an inline value, local file, or remote URL."""

    if checksum:
        return checksum.strip()
    if checksum_path:
        return checksum_path.read_text(encoding="utf-8")
    if checksum_url:
        checksum_target = cache_path.with_suffix(cache_path.suffix + ".sha256")
        download_file(checksum_url, checksum_target)
        return checksum_target.read_text(encoding="utf-8")
    return None


def verify_bundle_checksum(
    bundle_path: Path,
    checksum: Optional[str] = None,
    checksum_path: Optional[Path] = None,
    checksum_url: Optional[str] = None,
) -> str:
    """Verify the bundle against a supplied checksum source and return the digest."""

    checksum_text = load_checksum_text(
        checksum=checksum,
        checksum_path=checksum_path,
        checksum_url=checksum_url,
        cache_path=bundle_path,
    )
    actual = sha256_file(bundle_path)
    if checksum_text is None:
        return actual

    expected = parse_checksum_text(checksum_text, bundle_path.name)
    if actual.lower() != expected.lower():
        raise ValueError(
            f"checksum mismatch for {bundle_path}: expected {expected}, got {actual}"
        )
    return actual
