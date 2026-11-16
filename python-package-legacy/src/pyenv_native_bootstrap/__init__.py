"""The final `pyenv-native` release: forwards to Enchi's `enchi-bootstrap`.

The project was renamed to Enchi. This distribution only depends on `enchi` (pinned to the same
version) and keeps the `pyenv-native` console script and the `pyenv_native_bootstrap` module name
working, so existing `pipx install pyenv-native` setups upgrade into Enchi.
"""

from __future__ import annotations

import sys

NOTICE = (
    "pyenv-native is now Enchi: this command forwards to `enchi-bootstrap` "
    "(install the `enchi` package directly to drop this notice)."
)


def main() -> int:
    """Print the rename notice on stderr, then run Enchi's bootstrap CLI with the same arguments."""

    print(NOTICE, file=sys.stderr)
    from enchi_bootstrap.cli import main as enchi_main

    return enchi_main()


__all__ = ["NOTICE", "main"]
