"""Tests for the legacy `pyenv-native` wrapper."""

from __future__ import annotations

import contextlib
import io
import re
import unittest
from pathlib import Path
from unittest import mock

import enchi_bootstrap
import pyenv_native_bootstrap

PYPROJECT = Path(__file__).resolve().parents[1] / "pyproject.toml"


class ForwardingTests(unittest.TestCase):
    """The wrapper prints one notice, then behaves exactly like `enchi-bootstrap`."""

    def test_main_prints_a_one_line_notice_and_forwards(self) -> None:
        stderr = io.StringIO()
        with mock.patch("enchi_bootstrap.cli.main", return_value=7) as forwarded:
            with contextlib.redirect_stderr(stderr):
                result = pyenv_native_bootstrap.main()
        self.assertEqual(result, 7)
        forwarded.assert_called_once_with()
        lines = stderr.getvalue().splitlines()
        self.assertEqual(len(lines), 1, lines)
        self.assertIn("Enchi", lines[0])
        self.assertIn("enchi-bootstrap", lines[0])

    def test_the_console_script_targets_the_wrapper(self) -> None:
        text = PYPROJECT.read_text(encoding="utf-8")
        self.assertIn('pyenv-native = "pyenv_native_bootstrap:main"', text)
        self.assertIn('name = "pyenv-native"', text)

    def test_the_enchi_pin_and_version_match_the_enchi_package(self) -> None:
        text = PYPROJECT.read_text(encoding="utf-8")
        pin = re.search(r'"enchi==([^"]+)"', text)
        version = re.search(r'^version = "([^"]+)"', text, re.MULTILINE)
        self.assertIsNotNone(pin)
        self.assertIsNotNone(version)
        self.assertEqual(pin.group(1), enchi_bootstrap.__version__)
        self.assertEqual(version.group(1), enchi_bootstrap.__version__)


if __name__ == "__main__":
    unittest.main()
