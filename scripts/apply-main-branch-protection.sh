#!/usr/bin/env bash
# ./scripts/apply-main-branch-protection.sh
# Purpose: Apply the repository ruleset that protects main from deletion and force-push.
# How to run: bash ./scripts/apply-main-branch-protection.sh
# Inputs: Requires GitHub CLI (gh) authenticated as a repo admin.
# Outputs/side effects: Creates or updates the "Protect main" ruleset via GitHub REST API.
# Notes: Cloud agents cannot set branch protection; run this locally with admin credentials.

set -eu

SCRIPT_DIR="$(CDPATH= cd -- "$(dirname -- "$0")" && pwd)"
REPO_ROOT="$(CDPATH= cd -- "${SCRIPT_DIR}/.." && pwd)"
RULESET_FILE="${REPO_ROOT}/.github/rulesets/protect-main.json"
RULESET_NAME="Protect main"

if ! command -v gh >/dev/null 2>&1; then
  echo "error: GitHub CLI (gh) is required." >&2
  exit 1
fi

if [[ ! -f "$RULESET_FILE" ]]; then
  echo "error: ruleset file not found: ${RULESET_FILE}" >&2
  exit 1
fi

REPO="$(gh repo view --json nameWithOwner --jq .nameWithOwner)"
echo "Applying ruleset to ${REPO}..."

EXISTING_ID="$(
  gh api "repos/${REPO}/rulesets" --jq ".[] | select(.name == \"${RULESET_NAME}\") | .id" 2>/dev/null || true
)"

if [[ -n "$EXISTING_ID" ]]; then
  echo "Updating existing ruleset id=${EXISTING_ID}"
  gh api \
    -X PUT \
    "repos/${REPO}/rulesets/${EXISTING_ID}" \
    --input "$RULESET_FILE"
else
  echo "Creating new ruleset"
  gh api \
    -X POST \
    "repos/${REPO}/rulesets" \
    --input "$RULESET_FILE"
fi

echo "Done. Verify at: https://github.com/${REPO}/settings/rules"
