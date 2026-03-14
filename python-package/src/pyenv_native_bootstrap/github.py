# ./python-package/src/pyenv_native_bootstrap/github.py
"""GitHub release discovery helpers for locating pyenv-native bundle assets."""

from __future__ import annotations

import json
import urllib.request
from dataclasses import dataclass
from typing import Callable, Optional

from . import __version__


@dataclass(frozen=True)
class GitHubReleaseAsset:
    """A single downloadable asset from a GitHub release payload."""

    name: str
    browser_download_url: str


@dataclass(frozen=True)
class GitHubRelease:
    """Normalized release metadata used by the bootstrap installer."""

    tag_name: str
    assets: tuple[GitHubReleaseAsset, ...]


def github_release_api_url(repo: str, tag: Optional[str] = None) -> str:
    """Return the GitHub API URL for the latest or a tagged release."""

    cleaned_repo = repo.strip().strip("/")
    if not cleaned_repo or "/" not in cleaned_repo:
        raise ValueError("github repo must be in the form owner/name")

    if tag:
        return f"https://api.github.com/repos/{cleaned_repo}/releases/tags/{tag}"
    return f"https://api.github.com/repos/{cleaned_repo}/releases/latest"


def fetch_release_payload(url: str) -> dict[str, object]:
    """Fetch a GitHub release payload from the API."""

    request = urllib.request.Request(
        url,
        headers={
            "Accept": "application/vnd.github+json",
            "User-Agent": f"pyenv-native-bootstrap/{__version__}",
        },
    )
    with urllib.request.urlopen(request) as response:  # noqa: S310
        return json.loads(response.read().decode("utf-8"))


def parse_release_payload(payload: dict[str, object]) -> GitHubRelease:
    """Normalize a GitHub release payload into the local dataclass shape."""

    tag_name = str(payload["tag_name"])
    assets_payload = payload.get("assets", [])
    assets = tuple(
        GitHubReleaseAsset(
            name=str(asset["name"]),
            browser_download_url=str(asset["browser_download_url"]),
        )
        for asset in assets_payload
        if isinstance(asset, dict)
        and "name" in asset
        and "browser_download_url" in asset
    )
    return GitHubRelease(tag_name=tag_name, assets=assets)


def resolve_release_asset_urls(
    repo: str,
    bundle_file_name: str,
    tag: Optional[str] = None,
    fetcher: Optional[Callable[[str], dict[str, object]]] = None,
) -> tuple[str, str, str]:
    """Resolve bundle and checksum URLs from a GitHub release."""

    url = github_release_api_url(repo, tag)
    payload = (fetcher or fetch_release_payload)(url)
    release = parse_release_payload(payload)

    bundle_asset = next(
        (asset for asset in release.assets if asset.name == bundle_file_name),
        None,
    )
    if bundle_asset is None:
        raise ValueError(
            f"release {repo}@{release.tag_name} did not contain asset {bundle_file_name!r}"
        )

    checksum_name = f"{bundle_file_name}.sha256"
    checksum_asset = next(
        (asset for asset in release.assets if asset.name == checksum_name),
        None,
    )
    if checksum_asset is None:
        raise ValueError(
            f"release {repo}@{release.tag_name} did not contain asset {checksum_name!r}"
        )

    return bundle_asset.browser_download_url, checksum_asset.browser_download_url, release.tag_name
