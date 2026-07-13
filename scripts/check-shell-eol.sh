#!/usr/bin/env sh
# ./scripts/check-shell-eol.sh
# Purpose: Fail CI when POSIX shell scripts contain Windows CRLF line endings.
# How to run: sh ./scripts/check-shell-eol.sh
# Inputs: Repository tree.
# Outputs: Non-zero exit when any tracked *.sh file contains carriage returns.
# Notes: Complements .gitattributes eol=lf enforcement for shell installers.

set -eu

failed=0
cr=$(printf '\r')

for script in $(find . -name '*.sh' ! -path './.git/*' | sort); do
  if grep -q "$cr" "$script"; then
    printf 'error: %s contains CRLF line endings\n' "$script" >&2
    failed=1
  fi
done

if [ "$failed" -ne 0 ]; then
  exit 1
fi

printf 'All shell scripts use LF line endings.\n'
