# Push to the remote, then watch every GitHub Actions run triggered by the new
# commit until they all finish, announcing the verdict out loud. A push to main
# kicks off two workflows (CI and Deploy to GitHub Pages), so this waits on both.
Set-StrictMode -Version Latest
$ErrorActionPreference = "Stop"

Push-Location (Split-Path -Parent $PSScriptRoot)
try {
    git push
    if ($LASTEXITCODE -ne 0) { exit $LASTEXITCODE }

    $sha = (git rev-parse HEAD).Trim()

    # Wait for GitHub to register the workflow runs for this commit. Both the CI
    # and Deploy workflows trigger on push to main, so poll until we have seen at
    # least one and the count stops growing.
    $runIds = @()
    for ($i = 0; $i -lt 30; $i++) {
        try {
            $raw = gh run list --limit 20 --json databaseId,headSha 2>$null
            if ($raw) {
                $runs = $raw | ConvertFrom-Json
                $ids = @($runs | Where-Object { $_.headSha -eq $sha } | ForEach-Object { $_.databaseId })
                # Settle once we have runs and a stable count between polls.
                if ($ids.Count -gt 0 -and $ids.Count -eq $runIds.Count) {
                    $runIds = $ids
                    break
                }
                $runIds = $ids
            }
        } catch {
            # transient gh / API error — keep polling
        }
        Start-Sleep -Seconds 2
    }
    if ($runIds.Count -eq 0) {
        Write-Host "No workflow run found for $sha after 60s"
        exit 1
    }
    Write-Host "Watching $($runIds.Count) workflow run(s) for $sha"

    # Stream live progress for each run. The watch exit code is deliberately NOT
    # used as the verdict: `gh run watch` also exits non-zero on its own
    # transient failures (a GitHub API blip, a dropped connection) while the run
    # is still going — which is exactly how a still-running deploy got announced
    # as "deploy failed".
    foreach ($runId in $runIds) {
        gh run watch $runId
    }

    # The verdict comes only from each run's real conclusion. Poll until every
    # run is genuinely "completed" — covering the case where `gh run watch`
    # bailed out early — and read success/failure from there.
    $allSucceeded = $true
    foreach ($runId in $runIds) {
        $conclusion = $null
        $name = "run $runId"
        for ($i = 0; $i -lt 60; $i++) {
            try {
                $raw = gh run view $runId --json status,conclusion,name 2>$null
                if ($raw) {
                    $run = $raw | ConvertFrom-Json
                    if ($run.name) { $name = $run.name }
                    if ($run.status -eq 'completed') {
                        $conclusion = $run.conclusion
                        break
                    }
                }
            } catch {
                # transient gh / API error — keep polling
            }
            Start-Sleep -Seconds 10
        }
        if ($conclusion -eq 'success') {
            Write-Host "OK   $name"
        } else {
            Write-Host "FAIL $name (conclusion: $conclusion)"
            $allSucceeded = $false
        }
    }

    Add-Type -AssemblyName System.Speech
    $speak = New-Object System.Speech.Synthesis.SpeechSynthesizer
    if ($allSucceeded) {
        $speak.Speak('build succeeded')
        exit 0
    } else {
        $speak.Speak('build failed')
        exit 1
    }
} finally {
    Pop-Location
}
