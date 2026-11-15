"""Module runner for `python -m pyenv_native_bootstrap` (forwards to Enchi)."""

from . import main


if __name__ == "__main__":
    raise SystemExit(main())
