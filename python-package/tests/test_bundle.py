# ./python-package/tests/test_bundle.py
"""Unit tests for bundle manifest and checksum helpers."""

from __future__ import annotations

import json
import tarfile
import tempfile
import unittest
from pathlib import Path
from zipfile import ZipFile

from pyenv_native_bootstrap.bundle import (
    parse_checksum_text,
    read_manifest_from_bundle,
    read_manifest_from_zip,
    sha256_file,
)


class BundleTests(unittest.TestCase):
    """Covers manifest reading and checksum parsing behavior."""

    def test_parse_checksum_text_supports_common_formats(self) -> None:
        self.assertEqual(
            parse_checksum_text("ABCDEF" * 10 + "abcd"[:4]),
            ("ABCDEF" * 10 + "abcd"[:4]).lower(),
        )
        digest = "a" * 64
        text = f"{digest}  pyenv-native-windows-x64.zip"
        self.assertEqual(parse_checksum_text(text, "pyenv-native-windows-x64.zip"), digest)

    def test_read_manifest_from_zip_returns_expected_fields(self) -> None:
        with tempfile.TemporaryDirectory() as temp_dir:
            bundle_path = Path(temp_dir) / "bundle.zip"
            manifest = {
                "bundle_name": "pyenv-native-windows-x64",
                "bundle_version": "0.1.0",
                "platform": "windows",
                "architecture": "x64",
                "executable": "pyenv.exe",
                "install_script": "install-pyenv-native.ps1",
                "uninstall_script": "uninstall-pyenv-native.ps1",
            }
            with ZipFile(bundle_path, "w") as archive:
                archive.writestr(
                    "bundle-manifest.json",
                    json.dumps(manifest).encode("utf-8-sig"),
                )

            parsed = read_manifest_from_zip(bundle_path)
            self.assertEqual(parsed.bundle_name, manifest["bundle_name"])
            self.assertEqual(parsed.install_script, manifest["install_script"])

    def test_read_manifest_from_tar_bundle_returns_expected_fields(self) -> None:
        with tempfile.TemporaryDirectory() as temp_dir:
            bundle_path = Path(temp_dir) / "bundle.tar.gz"
            manifest = {
                "bundle_name": "pyenv-native-linux-x64",
                "bundle_version": "0.1.0",
                "platform": "linux",
                "architecture": "x64",
                "executable": "pyenv",
                "install_script": "install-pyenv-native.sh",
                "uninstall_script": "uninstall-pyenv-native.sh",
            }
            manifest_path = Path(temp_dir) / "bundle-manifest.json"
            manifest_path.write_text(json.dumps(manifest), encoding="utf-8")
            with tarfile.open(bundle_path, "w:gz") as archive:
                archive.add(manifest_path, arcname="bundle-manifest.json")

            parsed = read_manifest_from_bundle(bundle_path)
            self.assertEqual(parsed.bundle_name, manifest["bundle_name"])
            self.assertEqual(parsed.install_script, manifest["install_script"])

    def test_sha256_file_returns_hex_digest(self) -> None:
        with tempfile.TemporaryDirectory() as temp_dir:
            payload = Path(temp_dir) / "payload.txt"
            payload.write_text("hello", encoding="utf-8")
            digest = sha256_file(payload)
            self.assertEqual(len(digest), 64)
            self.assertTrue(all(character in "0123456789abcdef" for character in digest))


if __name__ == "__main__":
    unittest.main()
