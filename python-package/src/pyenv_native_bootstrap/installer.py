# ./python-package/src/pyenv_native_bootstrap/installer.py
"""Bundle resolution, extraction, and installer execution for pyenv-native bootstrap flows."""

from __future__ import annotations

import json
import os
import shutil
import stat
import subprocess
import tarfile
import tempfile
from dataclasses import asdict, dataclass
from pathlib import Path
from typing import Optional, Sequence
from zipfile import ZipFile

from .bundle import (
    BundleManifest,
    download_file,
    is_tar_bundle,
    read_manifest_from_bundle,
    verify_bundle_checksum,
)
from .github import resolve_release_asset_urls
from .platforms import PlatformTarget, current_target


@dataclass(frozen=True)
class InstallRequest:
    """Parameters accepted by the bootstrap installer flow."""

    bundle_path: Optional[Path] = None
    bundle_url: Optional[str] = None
    release_base_url: Optional[str] = None
    github_repo: Optional[str] = None
    tag: Optional[str] = None
    checksum: Optional[str] = None
    checksum_path: Optional[Path] = None
    checksum_url: Optional[str] = None
    install_root: Optional[Path] = None
    shell: str = "pwsh"
    add_to_user_path: bool = True
    update_powershell_profile: bool = True
    refresh_shims: bool = True
    force: bool = False
    cache_dir: Optional[Path] = None
    keep_extracted: bool = False
    dry_run: bool = False


@dataclass(frozen=True)
class InstallPlan:
    """Resolved bundle, manifest, and command for an install operation."""

    bundle_path: Path
    bundle_source: str
    bundle_manifest: BundleManifest
    checksum_sha256: str
    extracted_dir: Path
    install_command: list[str]

    def to_dict(self) -> dict[str, object]:
        """Return a JSON-friendly representation."""

        return {
            "bundle_path": str(self.bundle_path),
            "bundle_source": self.bundle_source,
            "bundle_manifest": asdict(self.bundle_manifest),
            "checksum_sha256": self.checksum_sha256,
            "extracted_dir": str(self.extracted_dir),
            "install_command": self.install_command,
        }


def default_cache_dir() -> Path:
    """Return the default bootstrap cache location."""

    if os.name == "nt":
        local_app_data = os.environ.get("LOCALAPPDATA")
        if local_app_data:
            return Path(local_app_data) / "pyenv-native-bootstrap" / "cache"
    return Path.home() / ".cache" / "pyenv-native-bootstrap"


def resolve_release_urls(
    target: PlatformTarget,
    release_base_url: str,
) -> tuple[str, str]:
    """Resolve the bundle and checksum URLs from a release base URL."""

    base = release_base_url.rstrip("/")
    asset_name = target.bundle_file_name
    bundle_url = f"{base}/{asset_name}"
    checksum_url = f"{bundle_url}.sha256"
    return bundle_url, checksum_url


def resolve_bundle_path(
    request: InstallRequest,
    target: Optional[PlatformTarget] = None,
) -> tuple[Path, Optional[str], str]:
    """Resolve the bundle to a local file, downloading it if needed."""

    active_target = target or current_target()
    if request.bundle_path:
        return (
            request.bundle_path.expanduser().resolve(),
            request.checksum_url,
            "local bundle path",
        )

    cache_dir = (request.cache_dir or default_cache_dir()).expanduser().resolve()
    bundle_url = request.bundle_url
    checksum_url = request.checksum_url
    release_base_url = request.release_base_url or os.environ.get(
        "PYENV_NATIVE_BOOTSTRAP_RELEASE_BASE_URL"
    )
    github_repo = request.github_repo or os.environ.get("PYENV_NATIVE_BOOTSTRAP_GITHUB_REPO")
    bundle_source = "explicit bundle url" if bundle_url else "unknown"
    if not bundle_url and release_base_url:
        bundle_url, checksum_url = resolve_release_urls(active_target, release_base_url)
        bundle_source = f"release base url {release_base_url}"
    elif not bundle_url and github_repo:
        bundle_url, checksum_url, resolved_tag = resolve_release_asset_urls(
            github_repo,
            active_target.bundle_file_name,
            request.tag,
        )
        requested_tag = request.tag or resolved_tag
        bundle_source = f"github release {github_repo}@{requested_tag}"

    if bundle_url:
        bundle_path = cache_dir / active_target.bundle_file_name
        download_file(bundle_url, bundle_path)
        return bundle_path, checksum_url, bundle_source

    local_dev_bundle = Path.cwd() / "dist" / active_target.bundle_file_name
    if local_dev_bundle.is_file():
        return local_dev_bundle.resolve(), checksum_url, "local dev dist bundle"

    raise ValueError(
        "unable to resolve a pyenv-native bundle; pass --bundle-path, --bundle-url, --release-base-url, or --github-repo"
    )


def extract_bundle(bundle_path: Path, target_dir: Path) -> Path:
    """Extract a release bundle into a target directory."""

    target_dir.mkdir(parents=True, exist_ok=True)
    if is_tar_bundle(bundle_path):
        with tarfile.open(bundle_path, "r:*") as archive:
            try:
                archive.extractall(target_dir, filter="data")
            except TypeError:
                archive.extractall(target_dir)
    else:
        with ZipFile(bundle_path, "r") as archive:
            archive.extractall(target_dir)
    return target_dir


def powershell_executable() -> str:
    """Return the PowerShell executable used to run the bundled installer."""

    return os.environ.get("PYENV_NATIVE_BOOTSTRAP_POWERSHELL", "powershell")


def shell_executable() -> str:
    """Return the shell executable used for POSIX install scripts."""

    return os.environ.get("PYENV_NATIVE_BOOTSTRAP_SHELL", "sh")


def build_windows_install_command(
    extracted_dir: Path,
    request: InstallRequest,
    manifest: BundleManifest,
) -> list[str]:
    """Build the PowerShell command used to invoke the bundled installer."""

    install_script = extracted_dir / manifest.install_script
    executable = extracted_dir / manifest.executable
    command = [
        powershell_executable(),
        "-ExecutionPolicy",
        "Bypass",
        "-File",
        str(install_script),
        "-SourcePath",
        str(executable),
        "-Shell",
        request.shell,
        "-AddToUserPath",
        str(request.add_to_user_path).lower(),
        "-UpdatePowerShellProfile",
        str(request.update_powershell_profile).lower(),
        "-RefreshShims",
        str(request.refresh_shims).lower(),
    ]
    if request.install_root:
        command.extend(["-InstallRoot", str(request.install_root.expanduser().resolve())])
    if request.force:
        command.append("-Force")
    return command


def build_posix_install_command(
    extracted_dir: Path,
    request: InstallRequest,
    manifest: BundleManifest,
) -> list[str]:
    """Build the shell command used to invoke the bundled POSIX installer."""

    install_script = extracted_dir / manifest.install_script
    executable = extracted_dir / manifest.executable
    install_script.chmod(install_script.stat().st_mode | stat.S_IXUSR)
    executable.chmod(executable.stat().st_mode | stat.S_IXUSR)
    command = [
        shell_executable(),
        str(install_script),
        "--source-path",
        str(executable),
        "--shell",
        request.shell,
        "--add-to-user-path",
        str(request.add_to_user_path).lower(),
        "--update-shell-profile",
        str(request.update_powershell_profile).lower(),
        "--refresh-shims",
        str(request.refresh_shims).lower(),
    ]
    if request.install_root:
        command.extend(["--install-root", str(request.install_root.expanduser().resolve())])
    if request.force:
        command.append("--force")
    return command


def plan_install(request: InstallRequest) -> InstallPlan:
    """Resolve, verify, and prepare a bundle install plan."""

    bundle_path, resolved_checksum_url, bundle_source = resolve_bundle_path(request)
    checksum_sha256 = verify_bundle_checksum(
        bundle_path,
        checksum=request.checksum,
        checksum_path=request.checksum_path,
        checksum_url=resolved_checksum_url,
    )
    manifest = read_manifest_from_bundle(bundle_path)
    cache_root = (request.cache_dir or default_cache_dir()).expanduser().resolve()
    cache_root.mkdir(parents=True, exist_ok=True)
    extracted_dir = Path(
        tempfile.mkdtemp(prefix=f"{manifest.bundle_name}-", dir=str(cache_root))
    ).resolve()
    extract_bundle(bundle_path, extracted_dir)
    host_target = current_target()
    if manifest.platform != host_target.operating_system:
        raise RuntimeError(
            f"bundle platform {manifest.platform!r} does not match the current host platform {host_target.operating_system!r}"
        )

    if manifest.platform == "windows":
        install_command = build_windows_install_command(extracted_dir, request, manifest)
    elif manifest.platform in {"linux", "macos"}:
        install_command = build_posix_install_command(extracted_dir, request, manifest)
    else:
        raise RuntimeError(f"unsupported bundle platform {manifest.platform!r}")

    return InstallPlan(
        bundle_path=bundle_path,
        bundle_source=bundle_source,
        bundle_manifest=manifest,
        checksum_sha256=checksum_sha256,
        extracted_dir=extracted_dir,
        install_command=install_command,
    )


def run_install(request: InstallRequest) -> InstallPlan:
    """Execute the bootstrap install flow and return the resolved plan."""

    plan = plan_install(request)
    if request.dry_run:
        return plan

    subprocess.run(plan.install_command, check=True)
    if not request.keep_extracted:
        shutil.rmtree(plan.extracted_dir, ignore_errors=True)
    return plan


def format_plan_json(plan: InstallPlan) -> str:
    """Serialize an install plan to indented JSON."""

    return json.dumps(plan.to_dict(), indent=2)


def format_command(command: Sequence[str]) -> str:
    """Render a subprocess command in a readable shell form."""

    return subprocess.list2cmdline(list(command))
