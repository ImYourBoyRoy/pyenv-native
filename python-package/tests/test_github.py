# ./python-package/tests/test_github.py
"""Unit tests for GitHub release resolution helpers."""

from __future__ import annotations

import unittest

from pyenv_native_bootstrap.github import (
    github_release_api_url,
    parse_release_payload,
    resolve_release_asset_urls,
)


class GitHubTests(unittest.TestCase):
    """Covers release API URL and asset lookup behavior."""

    def test_github_release_api_url_supports_latest_and_tagged_endpoints(self) -> None:
        self.assertEqual(
            github_release_api_url("imyourboyroy/pyenv-native"),
            "https://api.github.com/repos/imyourboyroy/pyenv-native/releases/latest",
        )
        self.assertEqual(
            github_release_api_url("imyourboyroy/pyenv-native", "v0.1.0"),
            "https://api.github.com/repos/imyourboyroy/pyenv-native/releases/tags/v0.1.0",
        )

    def test_parse_release_payload_extracts_assets(self) -> None:
        release = parse_release_payload(
            {
                "tag_name": "v0.1.0",
                "assets": [
                    {
                        "name": "pyenv-native-windows-x64.zip",
                        "browser_download_url": "https://example.invalid/bundle.zip",
                    }
                ],
            }
        )
        self.assertEqual(release.tag_name, "v0.1.0")
        self.assertEqual(release.assets[0].name, "pyenv-native-windows-x64.zip")

    def test_resolve_release_asset_urls_picks_bundle_and_checksum(self) -> None:
        def fake_fetcher(_: str) -> dict[str, object]:
            return {
                "tag_name": "v0.1.0",
                "assets": [
                    {
                        "name": "pyenv-native-windows-x64.zip",
                        "browser_download_url": "https://example.invalid/pyenv-native-windows-x64.zip",
                    },
                    {
                        "name": "pyenv-native-windows-x64.zip.sha256",
                        "browser_download_url": "https://example.invalid/pyenv-native-windows-x64.zip.sha256",
                    },
                ],
            }

        bundle_url, checksum_url, tag = resolve_release_asset_urls(
            "imyourboyroy/pyenv-native",
            "pyenv-native-windows-x64.zip",
            fetcher=fake_fetcher,
        )
        self.assertEqual(tag, "v0.1.0")
        self.assertTrue(bundle_url.endswith(".zip"))
        self.assertTrue(checksum_url.endswith(".zip.sha256"))

    def test_resolve_release_asset_urls_supports_linux_tarball_assets(self) -> None:
        def fake_fetcher(_: str) -> dict[str, object]:
            return {
                "tag_name": "v0.1.0",
                "assets": [
                    {
                        "name": "pyenv-native-linux-x64.tar.gz",
                        "browser_download_url": "https://example.invalid/pyenv-native-linux-x64.tar.gz",
                    },
                    {
                        "name": "pyenv-native-linux-x64.tar.gz.sha256",
                        "browser_download_url": "https://example.invalid/pyenv-native-linux-x64.tar.gz.sha256",
                    },
                ],
            }

        bundle_url, checksum_url, tag = resolve_release_asset_urls(
            "imyourboyroy/pyenv-native",
            "pyenv-native-linux-x64.tar.gz",
            fetcher=fake_fetcher,
        )
        self.assertEqual(tag, "v0.1.0")
        self.assertTrue(bundle_url.endswith(".tar.gz"))
        self.assertTrue(checksum_url.endswith(".tar.gz.sha256"))


if __name__ == "__main__":
    unittest.main()
