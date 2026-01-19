#!/usr/bin/env bash
set -euo pipefail

# Generic GitHub issue importer (labels + milestones + issues) from JSON.
#
# Requires:
#   - gh (authenticated)
#   - jq
#
# Usage:
#   ./import-gh-issues.sh issues.json
#   REPO=owner/name ./import-gh-issues.sh issues.json
#   DRY_RUN=1 ./import-gh-issues.sh issues.json
#
# JSON format is described below.

if ! command -v gh >/dev/null 2>&1; then
  echo "ERROR: gh CLI not found."
  exit 1
fi
if ! command -v jq >/dev/null 2>&1; then
  echo "ERROR: jq not found."
  exit 1
fi

INPUT="${1:-}"
if [[ -z "${INPUT}" ]]; then
  echo "ERROR: missing input json file."
  echo "Usage: $0 issues.json"
  exit 1
fi
if [[ ! -f "${INPUT}" ]]; then
  echo "ERROR: file not found: ${INPUT}"
  exit 1
fi

DRY_RUN="${DRY_RUN:-0}"

# If REPO is not set, infer from current directory (same approach as your bootstrap script). :contentReference[oaicite:2]{index=2}
REPO="${REPO:-$(gh repo view --json nameWithOwner -q .nameWithOwner)}"
if [[ -z "${REPO}" ]]; then
  echo "ERROR: could not determine repo. Set REPO=owner/name"
  exit 1
fi

echo "Repo: ${REPO}"
echo "Input: ${INPUT}"
[[ "${DRY_RUN}" == "1" ]] && echo "DRY_RUN: enabled (no writes)"

# ----------------------------
# Helpers
# ----------------------------
gh_api() {
  # Wrapper to support DRY_RUN for mutating calls.
  # Usage: gh_api <gh args...>
  if [[ "${DRY_RUN}" == "1" ]]; then
    echo "+ gh ${*}"
    return 0
  fi
  gh "$@"
}

label_exists() {
  local label="$1"
  # This matches the pattern you used in issue.sh. :contentReference[oaicite:3]{index=3}
  gh api -H "Accept: application/vnd.github+json" "repos/$REPO/labels/$label" >/dev/null 2>&1
}

create_label_if_missing() {
  local name="$1"
  local color="$2"
  local desc="$3"

  if label_exists "$name"; then
    echo "Label exists: $name"
  else
    echo "Creating label: $name"
    gh_api label create "$name" --color "$color" --description "$desc" >/dev/null
  fi
}

milestone_number_by_title() {
  local title="$1"
  gh api -H "Accept: application/vnd.github+json" "repos/$REPO/milestones?state=all&per_page=100" \
    | jq -r --arg t "$title" '.[] | select(.title==$t) | .number' | head -n 1
}

create_milestone_if_missing() {
  local title="$1"
  local description="$2"
  local number
  number="$(milestone_number_by_title "$title" || true)"
  if [[ -n "${number:-}" ]]; then
    echo "Milestone exists: $title (#$number)"
  else
    echo "Creating milestone: $title"
    gh_api api -X POST -H "Accept: application/vnd.github+json" "repos/$REPO/milestones" \
      -f title="$title" \
      -f description="$description" >/dev/null
  fi
}

issue_exists_by_title() {
  local title="$1"
  # Same strategy as your issue.sh: search open + closed; exact-title match. :contentReference[oaicite:4]{index=4}
  gh issue list --repo "$REPO" --state all --search "\"$title\" in:title" --json title -q '.[].title' \
    | grep -Fxq "$title"
}

create_issue_if_missing() {
  local title="$1"
  local body="$2"
  local milestone="${3:-}"
  local labels_csv="${4:-}"
  local assignees_csv="${5:-}"

  if issue_exists_by_title "$title"; then
    echo "Issue exists: $title"
    return 0
  fi

  # labels: comma-separated -> repeated --label
  local label_args=()
  if [[ -n "${labels_csv}" ]]; then
    local labels=()
    IFS=',' read -r -a labels <<< "$labels_csv"
    for l in "${labels[@]}"; do
      l="$(echo "$l" | xargs)"
      [[ -n "$l" ]] && label_args+=(--label "$l")
    done
  fi

  # assignees: comma-separated -> repeated --assignee
  local assignee_args=()
  if [[ -n "${assignees_csv}" ]]; then
    local as=()
    IFS=',' read -r -a as <<< "$assignees_csv"
    for a in "${as[@]}"; do
      a="$(echo "$a" | xargs)"
      [[ -n "$a" ]] && assignee_args+=(--assignee "$a")
    done
  fi

  echo "Creating issue: $title"
  if [[ -n "${milestone}" ]]; then
    gh_api issue create --repo "$REPO" --title "$title" --body "$body" --milestone "$milestone" \
      "${label_args[@]}" "${assignee_args[@]}" >/dev/null
  else
    gh_api issue create --repo "$REPO" --title "$title" --body "$body" \
      "${label_args[@]}" "${assignee_args[@]}" >/dev/null
  fi
}

# ----------------------------
# Validate input shape (lightweight)
# ----------------------------
jq -e '
  (has("labels") and (.labels|type=="array")) and
  (has("milestones") and (.milestones|type=="array")) and
  (has("issues") and (.issues|type=="array"))
' "$INPUT" >/dev/null \
  || { echo "ERROR: input JSON must contain arrays: labels, milestones, issues"; exit 1; }

# ----------------------------
# 1) Labels
# ----------------------------
echo "== Creating labels =="
jq -c '.labels[]?' "$INPUT" | while read -r item; do
  name="$(jq -r '.name' <<<"$item")"
  color="$(jq -r '.color // "ededed"' <<<"$item")"
  desc="$(jq -r '.description // ""' <<<"$item")"
  if [[ -z "$name" || "$name" == "null" ]]; then
    echo "WARN: skipping label with missing name"
    continue
  fi
  create_label_if_missing "$name" "$color" "$desc"
done

# ----------------------------
# 2) Milestones
# ----------------------------
echo "== Creating milestones =="
jq -c '.milestones[]?' "$INPUT" | while read -r item; do
  title="$(jq -r '.title' <<<"$item")"
  desc="$(jq -r '.description // ""' <<<"$item")"
  if [[ -z "$title" || "$title" == "null" ]]; then
    echo "WARN: skipping milestone with missing title"
    continue
  fi
  create_milestone_if_missing "$title" "$desc"
done

# ----------------------------
# 3) Issues
# ----------------------------
echo "== Creating issues =="
jq -c '.issues[]?' "$INPUT" | while read -r item; do
  title="$(jq -r '.title' <<<"$item")"
  body="$(jq -r '.body // ""' <<<"$item")"
  milestone="$(jq -r '.milestone // ""' <<<"$item")"
  labels_csv="$(jq -r '(.labels // []) | join(",")' <<<"$item")"
  assignees_csv="$(jq -r '(.assignees // []) | join(",")' <<<"$item")"

  if [[ -z "$title" || "$title" == "null" ]]; then
    echo "WARN: skipping issue with missing title"
    continue
  fi

  # normalize empty strings
  [[ "$milestone" == "null" ]] && milestone=""
  [[ "$milestone" == "" ]] && milestone=""

  create_issue_if_missing "$title" "$body" "$milestone" "$labels_csv" "$assignees_csv"
done

echo "Done."
