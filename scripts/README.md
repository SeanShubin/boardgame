# Scripts

Common dev commands so you don't have to remember the cargo invocations. Every
command has a PowerShell (`.ps1`) and a bash (`.sh`) version; both `cd` to the
repo root first, so they work from anywhere. Extra arguments pass straight
through to the underlying cargo command.

| Script         | What it does                                                                                                                                                                                       |
| -------------- | -------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `build`        | Build the whole workspace (debug). `build --release` for release.                                                                                                                                  |
| `run`          | Run the game (`boardgame` launcher).                                                                                                                                                               |
| `combat`       | Combat simulator: play one encounter in a clickable window (`combat 4`). A solo (0-3) fields one kit - its counter by default, or `combat 3 Raider` to pick; a party (4-7) fields the full roster. |
| `test`         | Run the whole test suite.                                                                                                                                                                          |
| `check`        | Fast type-check, no binaries produced.                                                                                                                                                             |
| `fmt`          | Format all code in place. `fmt --check` to verify only.                                                                                                                                            |
| `lint`         | Clippy across the workspace, warnings treated as errors.                                                                                                                                           |
| `verify`       | The pre-push gauntlet: fmt check + clippy + tests + build.                                                                                                                                         |
| `push`         | Push, then watch the CI + Pages runs and announce the verdict.                                                                                                                                     |
| `pad-tables`   | Align all markdown tables in the repo so columns line up.                                                                                                                                          |
| `card-gallery` | Render every card at all sizes and report text overflow.                                                                                                                                           |

## Usage

PowerShell:

```powershell
scripts\run.ps1
scripts\build.ps1 --release
scripts\verify.ps1
```

bash:

```sh
scripts/run.sh
scripts/build.sh --release
scripts/verify.sh
```
