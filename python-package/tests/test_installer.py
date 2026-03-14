# ./python-package/tests/test_installer.py
"""Unit tests for release URL resolution and install-command planning."""

from __future__ import annotations

import hashlib
import json
import tarfile
import tempfile
import unittest
from unittest import mock
from pathlib import Path

from pyenv_native_bootstrap.bundle import BundleManifest
from pyenv_native_bootstrap.installer import (
    InstallRequest,
    build_posix_install_command,
    build_windows_install_command,
    plan_install,
    resolve_bundle_path,
    resolve_release_urls,
)
from pyenv_native_bootstrap.platforms import PlatformTarget


class InstallerTests(unittest.TestCase):
    """Covers command planning without invoking PowerShell."""

    def test_platform_targets_choose_expected_bundle_extensions(self) -> None:
        self.assertEqual(
            PlatformTarget("windows", "x64").bundle_file_name,
            "pyenv-native-windows-x64.zip",
        )
        self.assertEqual(
            PlatformTarget("linux", "x64").bundle_file_name,
            "pyenv-native-linux-x64.tar.gz",
        )

    def test_resolve_release_urls_uses_inferred_bundle_name(self) -> None:
        bundle_url, checksum_url = resolve_release_urls(
            PlatformTarget("windows", "x64"),
            "https://example.com/releases/download/v0.1.0",
        )
        self.assertEqual(
            bundle_url,
            "https://example.com/releases/download/v0.1.0/pyenv-native-windows-x64.zip",
        )
        self.assertEqual(
            checksum_url,
            "https://example.com/releases/download/v0.1.0/pyenv-native-windows-x64.zip.sha256",
        )

    def test_build_windows_install_command_includes_expected_flags(self) -> None:
        with tempfile.TemporaryDirectory() as temp_dir:
            extracted_dir = Path(temp_dir)
            manifest = BundleManifest(
                bundle_name="pyenv-native-windows-x64",
                bundle_version="0.1.0",
                platform="windows",
                architecture="x64",
                executable="pyenv.exe",
                install_script="install-pyenv-native.ps1",
                uninstall_script="uninstall-pyenv-native.ps1",
            )
            command = build_windows_install_command(
                extracted_dir,
                InstallRequest(
                    install_root=Path(temp_dir) / "portable",
                    shell="cmd",
                    add_to_user_path=False,
                    update_powershell_profile=False,
                    refresh_shims=False,
                    force=True,
                ),
                manifest,
            )
            joined = " ".join(command)
            self.assertIn("-ExecutionPolicy", joined)
            self.assertIn("install-pyenv-native.ps1", joined)
            self.assertIn("-Shell cmd", joined)
            self.assertIn("-AddToUserPath false", joined)
            self.assertIn("-Force", joined)

    def test_build_posix_install_command_includes_expected_flags(self) -> None:
        with tempfile.TemporaryDirectory() as temp_dir:
            extracted_dir = Path(temp_dir)
            install_script = extracted_dir / "install-pyenv-native.sh"
            executable = extracted_dir / "pyenv"
            install_script.write_text("#!/usr/bin/env sh\nexit 0\n", encoding="utf-8")
            executable.write_text("binary", encoding="utf-8")
            manifest = BundleManifest(
                bundle_name="pyenv-native-linux-x64",
                bundle_version="0.1.0",
                platform="linux",
                architecture="x64",
                executable="pyenv",
                install_script="install-pyenv-native.sh",
                uninstall_script="uninstall-pyenv-native.sh",
            )
            command = build_posix_install_command(
                extracted_dir,
                InstallRequest(
                    install_root=Path(temp_dir) / "portable",
                    shell="zsh",
                    add_to_user_path=False,
                    update_powershell_profile=False,
                    refresh_shims=False,
                    force=True,
                ),
                manifest,
            )
            self.assertEqual(command[0], "sh")
            self.assertIn("install-pyenv-native.sh", command[1])
            self.assertIn("--shell", command)
            self.assertIn("zsh", command)
            self.assertIn("--add-to-user-path", command)
            self.assertIn("false", command)
            self.assertIn("--force", command)

    def test_resolve_bundle_path_uses_environment_release_base_url(self) -> None:
        with tempfile.TemporaryDirectory() as temp_dir:
            bundle_path = Path(temp_dir) / "dist" / "pyenv-native-windows-x64.zip"
            bundle_path.parent.mkdir(parents=True, exist_ok=True)
            bundle_path.write_bytes(b"placeholder")

            def fake_download(_: str, destination: Path) -> Path:
                destination.parent.mkdir(parents=True, exist_ok=True)
                destination.write_bytes(b"bundle")
                return destination

            with mock.patch.dict(
                "os.environ",
                {"PYENV_NATIVE_BOOTSTRAP_RELEASE_BASE_URL": "https://example.invalid/releases/v0.1.0"},
                clear=False,
            ), mock.patch(
                "pyenv_native_bootstrap.installer.download_file",
                side_effect=fake_download,
            ):
                resolved_bundle_path, checksum_url, bundle_source = resolve_bundle_path(
                    InstallRequest(cache_dir=Path(temp_dir) / "cache"),
                    PlatformTarget("windows", "x64"),
                )

            self.assertEqual(resolved_bundle_path.name, "pyenv-native-windows-x64.zip")
            self.assertTrue(checksum_url.endswith(".zip.sha256"))
            self.assertIn("release base url", bundle_source)

    def test_resolve_release_urls_supports_linux_bundle_names(self) -> None:
        bundle_url, checksum_url = resolve_release_urls(
            PlatformTarget("linux", "x64"),
            "https://example.com/releases/download/v0.1.0",
        )
        self.assertEqual(
            bundle_url,
            "https://example.com/releases/download/v0.1.0/pyenv-native-linux-x64.tar.gz",
        )
        self.assertEqual(
            checksum_url,
            "https://example.com/releases/download/v0.1.0/pyenv-native-linux-x64.tar.gz.sha256",
        )

    def test_plan_install_supports_linux_bundle_execution(self) -> None:
        with tempfile.TemporaryDirectory() as temp_dir:
            temp_root = Path(temp_dir)
            bundle_path = temp_root / "pyenv-native-linux-x64.tar.gz"
            manifest = {
                "bundle_name": "pyenv-native-linux-x64",
                "bundle_version": "0.1.0",
                "platform": "linux",
                "architecture": "x64",
                "executable": "pyenv",
                "install_script": "install-pyenv-native.sh",
                "uninstall_script": "uninstall-pyenv-native.sh",
            }
            manifest_path = temp_root / "bundle-manifest.json"
            executable_path = temp_root / "pyenv"
            install_path = temp_root / "install-pyenv-native.sh"
            uninstall_path = temp_root / "uninstall-pyenv-native.sh"
            manifest_path.write_text(json.dumps(manifest), encoding="utf-8")
            executable_path.write_text("binary", encoding="utf-8")
            install_path.write_text("#!/usr/bin/env sh\nexit 0\n", encoding="utf-8")
            uninstall_path.write_text("#!/usr/bin/env sh\nexit 0\n", encoding="utf-8")
            with tarfile.open(bundle_path, "w:gz") as archive:
                archive.add(manifest_path, arcname="bundle-manifest.json")
                archive.add(executable_path, arcname="pyenv")
                archive.add(install_path, arcname="install-pyenv-native.sh")
                archive.add(uninstall_path, arcname="uninstall-pyenv-native.sh")

            digest = hashlib.sha256(bundle_path.read_bytes()).hexdigest()
            request = InstallRequest(
                bundle_path=bundle_path,
                checksum=digest,
                install_root=temp_root / "portable",
                shell="bash",
                add_to_user_path=False,
                update_powershell_profile=False,
                refresh_shims=False,
                cache_dir=temp_root / "cache",
            )

            with mock.patch(
                "pyenv_native_bootstrap.installer.current_target",
                return_value=PlatformTarget("linux", "x64"),
            ):
                plan = plan_install(request)

            self.assertEqual(plan.bundle_manifest.platform, "linux")
            self.assertEqual(plan.install_command[0], "sh")
            self.assertIn("install-pyenv-native.sh", plan.install_command[1])
            self.assertIn("--install-root", plan.install_command)


if __name__ == "__main__":
    unittest.main()
