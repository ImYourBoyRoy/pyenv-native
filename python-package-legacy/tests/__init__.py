"""Test package bootstrap: the wrapper under test plus the `enchi` package it forwards to."""

from __future__ import annotations

import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
for source in (ROOT / "python-package-legacy" / "src", ROOT / "python-package" / "src"):
    if str(source) not in sys.path:
        sys.path.insert(0, str(source))
