# ./python-package/src/pyenv_native_bootstrap/cli.py
"""Command-line entrypoint for the pyenv-native Python bootstrap wrapper."""

from __future__ import annotations

import argparse
import json
from pathlib import Path

from .bundle import read_manifest_from_bundle, verify_bundle_checksum
from .installer import InstallRequest, format_command, format_plan_json, run_install, resolve_bundle_path
from .platforms import current_target


def default_install_shell() -> str:
    """Return the default shell choice for the current platform."""

    return "pwsh" if current_target().operating_system == "windows" else "bash"


def str_to_bool(value: str) -> bool:
    """Parse common boolean command-line strings."""

    normalized = value.strip().lower()
    if normalized in {"1", "true", "yes", "on"}:
        return True
    if normalized in {"0", "false", "no", "off"}:
        return False
    raise argparse.ArgumentTypeError(
        f"invalid boolean value {value!r}; use true/false, yes/no, on/off, or 1/0"
    )


def build_parser() -> argparse.ArgumentParser:
    """Create the top-level argument parser."""

    parser = argparse.ArgumentParser(prog="pyenv-native-bootstrap")
    subparsers = parser.add_subparsers(dest="command", required=True)

    install = subparsers.add_parser("install", help="install a native pyenv-native release bundle")
    install.add_argument("--bundle-path", type=Path)
    install.add_argument("--bundle-url")
    install.add_argument("--release-base-url")
    install.add_argument("--github-repo")
    install.add_argument("--tag")
    install.add_argument("--checksum")
    install.add_argument("--checksum-path", type=Path)
    install.add_argument("--checksum-url")
    install.add_argument("--install-root", type=Path)
    install.add_argument(
        "--shell",
        choices=["pwsh", "cmd", "bash", "zsh", "fish", "sh", "none"],
        default=default_install_shell(),
    )
    install.add_argument("--add-to-user-path", type=str_to_bool, default=True)
    install.add_argument(
        "--update-shell-profile",
        "--update-powershell-profile",
        dest="update_shell_profile",
        type=str_to_bool,
        default=True,
    )
    install.add_argument("--refresh-shims", type=str_to_bool, default=True)
    install.add_argument("--force", action="store_true")
    install.add_argument("--cache-dir", type=Path)
    install.add_argument("--keep-extracted", action="store_true")
    install.add_argument("--dry-run", action="store_true")
    install.add_argument("--json", action="store_true")

    download = subparsers.add_parser("download", help="download and optionally verify a release bundle")
    download.add_argument("--bundle-path", type=Path)
    download.add_argument("--bundle-url")
    download.add_argument("--release-base-url")
    download.add_argument("--github-repo")
    download.add_argument("--tag")
    download.add_argument("--checksum")
    download.add_argument("--checksum-path", type=Path)
    download.add_argument("--checksum-url")
    download.add_argument("--cache-dir", type=Path)
    download.add_argument("--json", action="store_true")

    verify = subparsers.add_parser("verify", help="verify a local release bundle")
    verify.add_argument("bundle_path", type=Path)
    verify.add_argument("--checksum")
    verify.add_argument("--checksum-path", type=Path)
    verify.add_argument("--checksum-url")
    verify.add_argument("--json", action="store_true")

    return parser


def request_from_args(args: argparse.Namespace) -> InstallRequest:
    """Convert argparse output into an install request."""

    return InstallRequest(
        bundle_path=args.bundle_path,
        bundle_url=getattr(args, "bundle_url", None),
        release_base_url=getattr(args, "release_base_url", None),
        github_repo=getattr(args, "github_repo", None),
        tag=getattr(args, "tag", None),
        checksum=getattr(args, "checksum", None),
        checksum_path=getattr(args, "checksum_path", None),
        checksum_url=getattr(args, "checksum_url", None),
        install_root=getattr(args, "install_root", None),
        shell=getattr(args, "shell", default_install_shell()),
        add_to_user_path=getattr(args, "add_to_user_path", True),
        update_powershell_profile=getattr(args, "update_shell_profile", True),
        refresh_shims=getattr(args, "refresh_shims", True),
        force=getattr(args, "force", False),
        cache_dir=getattr(args, "cache_dir", None),
        keep_extracted=getattr(args, "keep_extracted", False),
        dry_run=getattr(args, "dry_run", False),
    )


def cmd_install(args: argparse.Namespace) -> int:
    """Handle the install subcommand."""

    plan = run_install(request_from_args(args))
    if args.json:
        print(format_plan_json(plan))
        return 0

    print(f"Bundle: {plan.bundle_path}")
    print(f"Source: {plan.bundle_source}")
    print(f"Version: {plan.bundle_manifest.bundle_version}")
    print(f"Target: {plan.bundle_manifest.platform}/{plan.bundle_manifest.architecture}")
    print(f"Checksum: {plan.checksum_sha256}")
    print(f"Installer: {format_command(plan.install_command)}")
    if args.dry_run:
        print("Mode: dry-run")
    else:
        print("Status: installed")
    return 0


def cmd_download(args: argparse.Namespace) -> int:
    """Handle the download subcommand."""

    request = request_from_args(args)
    bundle_path, resolved_checksum_url, bundle_source = resolve_bundle_path(request, current_target())
    digest = verify_bundle_checksum(
        bundle_path,
        checksum=request.checksum,
        checksum_path=request.checksum_path,
        checksum_url=resolved_checksum_url,
    )
    manifest = read_manifest_from_bundle(bundle_path)
    payload = {
        "bundle_path": str(bundle_path),
        "bundle_source": bundle_source,
        "checksum_sha256": digest,
        "bundle_manifest": {
            "bundle_name": manifest.bundle_name,
            "bundle_version": manifest.bundle_version,
            "platform": manifest.platform,
            "architecture": manifest.architecture,
        },
    }
    if args.json:
        print(json.dumps(payload, indent=2))
    else:
        print(f"Bundle: {payload['bundle_path']}")
        print(f"Source: {payload['bundle_source']}")
        print(f"Checksum: {payload['checksum_sha256']}")
        print(f"Version: {manifest.bundle_version}")
    return 0


def cmd_verify(args: argparse.Namespace) -> int:
    """Handle the verify subcommand."""

    digest = verify_bundle_checksum(
        args.bundle_path,
        checksum=args.checksum,
        checksum_path=args.checksum_path,
        checksum_url=args.checksum_url,
    )
    manifest = read_manifest_from_bundle(args.bundle_path)
    payload = {
        "bundle_path": str(args.bundle_path),
        "checksum_sha256": digest,
        "bundle_manifest": {
            "bundle_name": manifest.bundle_name,
            "bundle_version": manifest.bundle_version,
            "platform": manifest.platform,
            "architecture": manifest.architecture,
        },
    }
    if args.json:
        print(json.dumps(payload, indent=2))
    else:
        print(f"Bundle: {payload['bundle_path']}")
        print(f"Checksum: {payload['checksum_sha256']}")
        print(f"Manifest: {manifest.bundle_name} {manifest.bundle_version}")
    return 0


def main() -> int:
    """Program entrypoint."""

    parser = build_parser()
    args = parser.parse_args()
    if args.command == "install":
        return cmd_install(args)
    if args.command == "download":
        return cmd_download(args)
    if args.command == "verify":
        return cmd_verify(args)
    parser.error(f"unknown command {args.command!r}")
    return 2


if __name__ == "__main__":
    raise SystemExit(main())
