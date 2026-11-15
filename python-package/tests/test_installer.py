# ./python-package/tests/test_installer.py
"""Unit tests for release URL resolution and install-command planning."""

from __future__ import annotations

import hashlib
import json
import os
import tarfile
import tempfile
import unittest
from unittest import mock
from pathlib import Path

from enchi_bootstrap.bundle import BundleManifest
from enchi_bootstrap.installer import (
    InstallRequest,
    build_posix_install_command,
    build_self_install_command,
    build_windows_install_command,
    default_cache_dir,
    plan_install,
    resolve_bundle_path,
    resolve_release_urls,
)
from enchi_bootstrap.platforms import PlatformTarget, normalize_operating_system


class InstallerTests(unittest.TestCase):
    """Covers command planning without invoking PowerShell."""

    def test_platform_targets_choose_expected_bundle_extensions(self) -> None:
        self.assertEqual(
            PlatformTarget("windows", "x64").bundle_file_name,
            "enchi-windows-x64.zip",
        )
        self.assertEqual(
            PlatformTarget("linux", "x64").bundle_file_name,
            "enchi-linux-x64.tar.gz",
        )
        self.assertEqual(
            PlatformTarget("android", "arm64").bundle_file_name,
            "enchi-android-arm64.tar.gz",
        )

    def test_platform_targets_know_the_pre_rename_bundle_names(self) -> None:
        self.assertEqual(
            PlatformTarget("windows", "x64").legacy_bundle_file_name,
            "pyenv-native-windows-x64.zip",
        )
        self.assertEqual(
            PlatformTarget("linux", "arm64").legacy_bundle_file_name,
            "pyenv-native-linux-arm64.tar.gz",
        )

    def test_normalize_operating_system_detects_termux_as_android(self) -> None:
        with mock.patch.dict(
            "os.environ",
            {"TERMUX_VERSION": "0.118.0", "PREFIX": "/data/data/com.termux/files/usr"},
            clear=False,
        ):
            self.assertEqual(normalize_operating_system("linux"), "android")

    def test_resolve_release_urls_uses_inferred_bundle_name(self) -> None:
        bundle_url, checksum_url = resolve_release_urls(
            PlatformTarget("windows", "x64"),
            "https://example.com/releases/download/v0.1.0",
        )
        self.assertEqual(
            bundle_url,
            "https://example.com/releases/download/v0.1.0/enchi-windows-x64.zip",
        )
        self.assertEqual(
            checksum_url,
            "https://example.com/releases/download/v0.1.0/enchi-windows-x64.zip.sha256",
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

    def test_build_self_install_command_runs_the_bundled_binary(self) -> None:
        with tempfile.TemporaryDirectory() as temp_dir:
            extracted_dir = Path(temp_dir)
            executable = extracted_dir / "enchi"
            executable.write_text("binary", encoding="utf-8")
            manifest = BundleManifest(
                bundle_name="enchi-linux-x64",
                bundle_version="0.8.0",
                platform="linux",
                architecture="x64",
                executable="enchi",
            )
            self.assertIsNone(manifest.install_script)
            command = build_self_install_command(
                extracted_dir,
                InstallRequest(
                    install_root=Path(temp_dir) / "portable",
                    shell="zsh",
                    add_to_user_path=False,
                    update_powershell_profile=False,
                    refresh_shims=True,
                    force=True,
                ),
                manifest,
            )
            self.assertEqual(command[0], str(executable))
            self.assertEqual(command[1], "self-install")
            self.assertEqual(command[command.index("--source-dir") + 1], str(extracted_dir))
            self.assertEqual(command[command.index("--shell") + 1], "zsh")
            self.assertEqual(command[command.index("--add-to-user-path") + 1], "false")
            self.assertEqual(command[command.index("--refresh-shims") + 1], "true")
            self.assertIn("--force", command)
            self.assertIn("--install-root", command)

    def test_resolve_bundle_path_uses_environment_release_base_url(self) -> None:
        with tempfile.TemporaryDirectory() as temp_dir:
            bundle_path = Path(temp_dir) / "dist" / "enchi-windows-x64.zip"
            bundle_path.parent.mkdir(parents=True, exist_ok=True)
            bundle_path.write_bytes(b"placeholder")

            def fake_download(_: str, destination: Path) -> Path:
                destination.parent.mkdir(parents=True, exist_ok=True)
                destination.write_bytes(b"bundle")
                return destination

            with mock.patch.dict(
                "os.environ",
                {"ENCHI_BOOTSTRAP_RELEASE_BASE_URL": "https://example.invalid/releases/v0.1.0"},
                clear=False,
            ), mock.patch(
                "enchi_bootstrap.installer.download_file",
                side_effect=fake_download,
            ):
                resolved_bundle_path, checksum_url, bundle_source = resolve_bundle_path(
                    InstallRequest(cache_dir=Path(temp_dir) / "cache"),
                    PlatformTarget("windows", "x64"),
                )

            self.assertEqual(resolved_bundle_path.name, "enchi-windows-x64.zip")
            self.assertTrue(checksum_url.endswith(".zip.sha256"))
            self.assertIn("release base url", bundle_source)

    def test_pre_rename_environment_variable_is_still_honoured(self) -> None:
        with tempfile.TemporaryDirectory() as temp_dir:

            def fake_download(_: str, destination: Path) -> Path:
                destination.parent.mkdir(parents=True, exist_ok=True)
                destination.write_bytes(b"bundle")
                return destination

            environment = {
                "PYENV_NATIVE_BOOTSTRAP_RELEASE_BASE_URL": "https://example.invalid/releases/v0.1.0"
            }
            with mock.patch.dict("os.environ", environment, clear=False), mock.patch(
                "enchi_bootstrap.installer.download_file",
                side_effect=fake_download,
            ):
                os.environ.pop("ENCHI_BOOTSTRAP_RELEASE_BASE_URL", None)
                resolved_bundle_path, _, bundle_source = resolve_bundle_path(
                    InstallRequest(cache_dir=Path(temp_dir) / "cache"),
                    PlatformTarget("linux", "x64"),
                )
            self.assertEqual(resolved_bundle_path.name, "enchi-linux-x64.tar.gz")
            self.assertIn("release base url", bundle_source)

    def test_default_cache_dir_is_named_enchi(self) -> None:
        self.assertEqual(default_cache_dir().parts[-1] in {"enchi", "cache"}, True)
        self.assertIn("enchi", default_cache_dir().parts)
        self.assertNotIn("pyenv-native", default_cache_dir().parts)

    def test_github_resolution_prefers_the_enchi_asset(self) -> None:
        with tempfile.TemporaryDirectory() as temp_dir:
            calls: list[str] = []

            def fake_resolve(repo: str, name: str, tag: object = None) -> tuple[str, str, str]:
                calls.append(name)
                return f"https://example.invalid/{name}", f"https://example.invalid/{name}.sha256", "v0.8.0"

            def fake_download(_: str, destination: Path) -> Path:
                destination.parent.mkdir(parents=True, exist_ok=True)
                destination.write_bytes(b"bundle")
                return destination

            with mock.patch(
                "enchi_bootstrap.installer.resolve_release_asset_urls", side_effect=fake_resolve
            ), mock.patch("enchi_bootstrap.installer.download_file", side_effect=fake_download):
                path, checksum_url, source = resolve_bundle_path(
                    InstallRequest(github_repo="o/r", cache_dir=Path(temp_dir) / "cache"),
                    PlatformTarget("macos", "arm64"),
                )
            self.assertEqual(calls, ["enchi-macos-arm64.tar.gz"])
            self.assertEqual(path.name, "enchi-macos-arm64.tar.gz")
            self.assertIn("github release o/r@v0.8.0", source)
            self.assertTrue(checksum_url.endswith(".tar.gz.sha256"))

    def test_github_resolution_falls_back_to_the_pre_rename_asset(self) -> None:
        with tempfile.TemporaryDirectory() as temp_dir:
            calls: list[str] = []

            def fake_resolve(repo: str, name: str, tag: object = None) -> tuple[str, str, str]:
                calls.append(name)
                if name.startswith("enchi-"):
                    raise ValueError(f"release {repo}@v0.7.2 did not contain asset {name!r}")
                return f"https://example.invalid/{name}", f"https://example.invalid/{name}.sha256", "v0.7.2"

            def fake_download(_: str, destination: Path) -> Path:
                destination.parent.mkdir(parents=True, exist_ok=True)
                destination.write_bytes(b"bundle")
                return destination

            with mock.patch(
                "enchi_bootstrap.installer.resolve_release_asset_urls", side_effect=fake_resolve
            ), mock.patch("enchi_bootstrap.installer.download_file", side_effect=fake_download):
                path, _, _ = resolve_bundle_path(
                    InstallRequest(
                        github_repo="o/r", tag="v0.7.2", cache_dir=Path(temp_dir) / "cache"
                    ),
                    PlatformTarget("windows", "x64"),
                )
            self.assertEqual(calls, ["enchi-windows-x64.zip", "pyenv-native-windows-x64.zip"])
            self.assertEqual(path.name, "pyenv-native-windows-x64.zip")

    def test_resolve_release_urls_supports_linux_bundle_names(self) -> None:
        bundle_url, checksum_url = resolve_release_urls(
            PlatformTarget("linux", "x64"),
            "https://example.com/releases/download/v0.1.0",
        )
        self.assertEqual(
            bundle_url,
            "https://example.com/releases/download/v0.1.0/enchi-linux-x64.tar.gz",
        )
        self.assertEqual(
            checksum_url,
            "https://example.com/releases/download/v0.1.0/enchi-linux-x64.tar.gz.sha256",
        )

    def test_plan_install_supports_linux_bundle_execution(self) -> None:
        with tempfile.TemporaryDirectory() as temp_dir:
            temp_root = Path(temp_dir)
            bundle_path = temp_root / "pyenv-native-linux-x64.tar.gz"  # a pre-rename bundle
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
                "enchi_bootstrap.installer.current_target",
                return_value=PlatformTarget("linux", "x64"),
            ):
                plan = plan_install(request)

            self.assertEqual(plan.bundle_manifest.platform, "linux")
            self.assertEqual(plan.install_command[0], "sh")
            self.assertIn("install-pyenv-native.sh", plan.install_command[1])
            self.assertIn("--install-root", plan.install_command)


if __name__ == "__main__":
    unittest.main()
