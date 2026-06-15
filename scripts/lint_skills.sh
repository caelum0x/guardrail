#!/usr/bin/env bash
# lint_skills.sh — validate every skill's examples with the REAL validator.
#
# It runs guardrail_lab.skill.load_skill_examples + validate_example (from
# python-lab/) over each skills/*/examples directory, prints per-skill PASS/FAIL
# with the specific issues, and exits non-zero if any example is invalid (or a
# skill that ships an examples/ dir has no loadable examples).
#
# Usage:
#   bash scripts/lint_skills.sh
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" >/dev/null 2>&1 && pwd)"
REPO_ROOT="$(cd "${SCRIPT_DIR}/.." >/dev/null 2>&1 && pwd)"
PYTHON_LAB="${REPO_ROOT}/python-lab"
SKILLS_DIR="${REPO_ROOT}/skills"

PYTHON_BIN="${PYTHON_BIN:-python3}"

if [[ ! -d "${PYTHON_LAB}" ]]; then
  echo "error: python-lab not found at ${PYTHON_LAB}" >&2
  exit 1
fi
if [[ ! -d "${SKILLS_DIR}" ]]; then
  echo "error: skills directory not found at ${SKILLS_DIR}" >&2
  exit 1
fi

# Collect every skill directory that ships an examples/ folder.
example_dirs=()
while IFS= read -r -d '' dir; do
  example_dirs+=("${dir}")
done < <(find "${SKILLS_DIR}" -mindepth 2 -maxdepth 2 -type d -name examples -print0 | sort -z)

if [[ ${#example_dirs[@]} -eq 0 ]]; then
  echo "error: no skills/*/examples directories found under ${SKILLS_DIR}" >&2
  exit 1
fi

# Drive the real validator from Python. We pass the example dirs as argv and let
# Python own the PASS/FAIL logic and the process exit code.
PYTHONPATH="${PYTHON_LAB}" "${PYTHON_BIN}" - "${example_dirs[@]}" <<'PYEOF'
import os
import sys

from guardrail_lab.skill import load_skill_examples, validate_example

example_dirs = sys.argv[1:]
failures = 0

for examples_dir in example_dirs:
    skill_name = os.path.basename(os.path.dirname(examples_dir))
    examples = load_skill_examples(examples_dir)

    if not examples:
        print(f"FAIL  {skill_name}: no loadable examples in {examples_dir}")
        failures += 1
        continue

    skill_issues = []
    for example in examples:
        source = example.get("_source", "<unknown>")
        issues = validate_example(example)
        for issue in issues:
            skill_issues.append(f"{source}: {issue}")

    if skill_issues:
        print(f"FAIL  {skill_name} ({len(examples)} examples)")
        for entry in skill_issues:
            print(f"        - {entry}")
        failures += 1
    else:
        print(f"PASS  {skill_name} ({len(examples)} examples)")

print("")
if failures:
    print(f"{failures} skill(s) failed validation.")
    sys.exit(1)

print(f"All {len(example_dirs)} skill(s) passed validation.")
PYEOF
