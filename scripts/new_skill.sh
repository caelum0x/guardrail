#!/usr/bin/env bash
# new_skill.sh — scaffold a new Track-2 Strategy Skill from skills/_template.
#
# Usage:
#   bash scripts/new_skill.sh <skill-name>
#   e.g. bash scripts/new_skill.sh funding-skew-bsc
#
# It copies skills/_template to skills/<skill-name>, replaces the placeholder
# name token, refuses to overwrite an existing directory, and prints next steps.
set -euo pipefail

# Resolve repo root from this script's location so it works from any cwd.
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" >/dev/null 2>&1 && pwd)"
REPO_ROOT="$(cd "${SCRIPT_DIR}/.." >/dev/null 2>&1 && pwd)"
TEMPLATE_DIR="${REPO_ROOT}/skills/_template"
PLACEHOLDER="<PLACEHOLDER_SKILL_NAME>"

usage() {
  echo "Usage: bash scripts/new_skill.sh <skill-name>" >&2
  echo "  <skill-name>: kebab-case, e.g. funding-skew-bsc" >&2
}

# --- Validate arguments ------------------------------------------------------
if [[ $# -ne 1 ]]; then
  echo "error: expected exactly one argument (the new skill name)." >&2
  usage
  exit 2
fi

NAME="$1"

if [[ ! "${NAME}" =~ ^[a-z0-9]+(-[a-z0-9]+)*$ ]]; then
  echo "error: skill name must be kebab-case (lowercase letters, digits, hyphens)." >&2
  echo "       got: '${NAME}'" >&2
  exit 2
fi

if [[ "${NAME}" == "_template" ]]; then
  echo "error: '_template' is reserved for the skeleton skill." >&2
  exit 2
fi

# --- Preconditions -----------------------------------------------------------
if [[ ! -d "${TEMPLATE_DIR}" ]]; then
  echo "error: template directory not found at ${TEMPLATE_DIR}" >&2
  exit 1
fi

DEST_DIR="${REPO_ROOT}/skills/${NAME}"
if [[ -e "${DEST_DIR}" ]]; then
  echo "error: refusing to overwrite existing path: ${DEST_DIR}" >&2
  exit 1
fi

# --- Copy + substitute -------------------------------------------------------
cp -R "${TEMPLATE_DIR}" "${DEST_DIR}"

# Safe sed: replace the placeholder name token in every regular file in-place.
# Use a portable approach that works on both GNU and BSD sed.
while IFS= read -r -d '' file; do
  tmp="$(mktemp)"
  sed "s|${PLACEHOLDER}|${NAME}|g" "${file}" > "${tmp}"
  cat "${tmp}" > "${file}"
  rm -f "${tmp}"
done < <(find "${DEST_DIR}" -type f -print0)

# --- Report ------------------------------------------------------------------
echo "Created skill: ${DEST_DIR}"
echo ""
echo "Next steps:"
echo "  1. Edit skills/${NAME}/strategy_spec.yaml — customise section 4 (the signal/tilt)."
echo "  2. Replace every remaining <PLACEHOLDER> in skill.yaml, README.md, SKILL.md, and prompts/."
echo "  3. Update the four examples/*.json with your worked, validator-clean cases."
echo "  4. Validate: bash scripts/lint_skills.sh"
echo "  5. Add an entry to skills/INDEX.json and a row to skills/README.md."
