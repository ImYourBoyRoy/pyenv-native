# pyenv-native (now Enchi)

**pyenv-native has been renamed to Enchi.** This is the final `pyenv-native` release on PyPI. It
only depends on [`enchi`](https://pypi.org/project/enchi/) (pinned to the same version) and keeps
the `pyenv-native` console script working: it prints a one-line notice on stderr and forwards to
`enchi-bootstrap`.

```sh
pipx install enchi          # the new package; adds the `enchi-bootstrap` command
enchi-bootstrap install --github-repo imyourboyroy/pyenv-native
```

Existing installs keep working: `pyenv` stays available as an alias for `enchi`, your `~/.pyenv`
root is left where it is, and `PYENV_ROOT` is still honoured. See `docs/MIGRATING.md` in the
repository for the details.

- <https://github.com/imyourboyroy/pyenv-native>
