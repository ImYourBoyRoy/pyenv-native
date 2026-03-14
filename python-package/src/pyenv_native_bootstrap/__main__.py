# ./python-package/src/pyenv_native_bootstrap/__main__.py
"""Module runner for `python -m pyenv_native_bootstrap`."""

from .cli import main


if __name__ == "__main__":
    raise SystemExit(main())
