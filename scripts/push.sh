#!/usr/bin/env bash
# Push to the remote, then watch every GitHub Actions run triggered by the new
# commit until they all finish, announcing the verdict. A push to main kicks off
# two workflows (CI and Deploy to GitHub Pages), so this waits on both.
set -euo pipefail
cd "$(dirname "$0")/.."

# Gate on the same gauntlet CI runs (format check, clippy, tests, build) before
# pushing, so a failure surfaces here instead of after the push.
"$(dirname "$0")/verify.sh"

git push

sha="$(git rev-parse HEAD)"

# Wait for GitHub to register the workflow runs for this commit. Poll until we
# have seen at least one and the count stops growing between polls.
run_ids=()
prev_count=-1
for _ in $(seq 1 30); do
    if raw="$(gh run list --limit 20 --json databaseId,headSha 2>/dev/null)"; then
        mapfile -t run_ids < <(echo "$raw" | jq -r --arg sha "$sha" \
            '.[] | select(.headSha == $sha) | .databaseId')
        if [ "${#run_ids[@]}" -gt 0 ] && [ "${#run_ids[@]}" -eq "$prev_count" ]; then
            break
        fi
        prev_count="${#run_ids[@]}"
    fi
    sleep 2
done
if [ "${#run_ids[@]}" -eq 0 ]; then
    echo "No workflow run found for $sha after 60s"
    exit 1
fi
echo "Watching ${#run_ids[@]} workflow run(s) for $sha"

# Stream live progress for each run. The watch exit code is deliberately NOT used
# as the verdict: `gh run watch` also exits non-zero on its own transient
# failures while the run is still going.
for run_id in "${run_ids[@]}"; do
    gh run watch "$run_id" || true
done

# The verdict comes only from each run's real conclusion. Poll until every run
# is genuinely "completed" and read success/failure from there.
all_succeeded=1
for run_id in "${run_ids[@]}"; do
    conclusion=""
    name="run $run_id"
    for _ in $(seq 1 60); do
        if raw="$(gh run view "$run_id" --json status,conclusion,name 2>/dev/null)"; then
            status="$(echo "$raw" | jq -r '.status')"
            name="$(echo "$raw" | jq -r '.name')"
            if [ "$status" = "completed" ]; then
                conclusion="$(echo "$raw" | jq -r '.conclusion')"
                break
            fi
        fi
        sleep 10
    done
    if [ "$conclusion" = "success" ]; then
        echo "OK   $name"
    else
        echo "FAIL $name (conclusion: $conclusion)"
        all_succeeded=0
    fi
done

if [ "$all_succeeded" -eq 1 ]; then
    echo "build succeeded"
    exit 0
else
    echo "build failed"
    exit 1
fi
