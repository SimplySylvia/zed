#!/usr/bin/env bash
#
# Weekly upstream sync helper for the Plan fork (PRD Part II §5).
#
# FETCH + REPORT only. This script never merges, never checks out, and never
# rewrites history. It fetches upstream, reports how far the current branch has
# diverged, lists the new upstream commits, and flags any files touched by BOTH
# upstream and the fork (potential merge conflicts) — cross-checked against
# FORK_DIFF.md so a conflict outside that manifest is visible as a "leak".
#
# Perform the actual merge by hand after reviewing this report.
#
# Env overrides: UPSTREAM_REMOTE (default "upstream"),
#                UPSTREAM_BRANCH (default "main"),
#                INTEGRATION_BRANCH (default "plan").
set -euo pipefail

UPSTREAM_REMOTE="${UPSTREAM_REMOTE:-upstream}"
UPSTREAM_BRANCH="${UPSTREAM_BRANCH:-main}"
INTEGRATION_BRANCH="${INTEGRATION_BRANCH:-plan}"
FORK_DIFF="FORK_DIFF.md"

cd "$(git rev-parse --show-toplevel)"

if ! git remote get-url "${UPSTREAM_REMOTE}" >/dev/null 2>&1; then
    echo "error: remote '${UPSTREAM_REMOTE}' not configured." >&2
    echo "       add it with: git remote add ${UPSTREAM_REMOTE} https://github.com/zed-industries/zed.git" >&2
    exit 1
fi

echo "==> Fetching ${UPSTREAM_REMOTE}/${UPSTREAM_BRANCH}"
git fetch --quiet "${UPSTREAM_REMOTE}" "${UPSTREAM_BRANCH}"

UPSTREAM_REF="${UPSTREAM_REMOTE}/${UPSTREAM_BRANCH}"
CURRENT_BRANCH="$(git rev-parse --abbrev-ref HEAD)"

ahead="$(git rev-list --count "${UPSTREAM_REF}..HEAD")"
behind="$(git rev-list --count "HEAD..${UPSTREAM_REF}")"

echo "==> ${CURRENT_BRANCH} is ${ahead} ahead / ${behind} behind ${UPSTREAM_REF}"

if [ "${behind}" -eq 0 ]; then
    echo "==> Up to date with upstream. Nothing to merge."
    exit 0
fi

echo "==> New upstream commits (${behind}):"
git --no-pager log --oneline --no-decorate "HEAD..${UPSTREAM_REF}" | head -30
if [ "${behind}" -gt 30 ]; then
    echo "      … and $((behind - 30)) more."
fi

echo "==> Checking for potential conflicts (dry run — no changes made)…"
merge_base="$(git merge-base HEAD "${UPSTREAM_REF}")"
overlap="$(comm -12 \
    <(git diff --name-only "${merge_base}" "${UPSTREAM_REF}" | sort -u) \
    <(git diff --name-only "${merge_base}" HEAD | sort -u) || true)"

if [ -z "${overlap}" ]; then
    echo "==> No overlapping file changes. Merge expected to be clean."
else
    echo "==> Files changed by BOTH upstream and the fork (potential conflicts):"
    echo "${overlap}" | sed 's/^/      /'
    if [ -f "${FORK_DIFF}" ]; then
        echo "==> Cross-check each file above against ${FORK_DIFF}."
        echo "    Any file NOT listed there is a LEAK — the fork should touch only"
        echo "    the manifest's upstream files. Fix the leak, don't route around it."
    else
        echo "==> (${FORK_DIFF} not found — create it to classify expected vs. leaked touches.)"
    fi
fi

echo
echo "==> Report only. When ready to merge, review the above then run:"
echo "      git switch ${INTEGRATION_BRANCH} && git merge ${UPSTREAM_REF}"
