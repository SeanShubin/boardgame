#!/usr/bin/env bash
# Run the card-table app in DEBUG mode: a clean slate AND the doom oracle turned on.
# Two things in one: it discards the persisted session by deleting the active save
# (the app then loads the fresh sample table, exactly like "Start Over"), and it sets
# BOARDGAME_ORACLE so the System deck's "Doom oracle" toggle opens ON - the combat
# foresight badges (winnable / doomed) are visible from launch. A plain scripts/run.sh
# opens with the oracle off; you can also flip the toggle in-app at any time. Your own
# ".bak" backups next to the save are left alone. Extra args pass through to cargo, e.g.
# scripts/run-debug.sh --release
set -euo pipefail
cd "$(dirname "$0")/.."

# The native save location (see crates/boardgame/src/persistence.rs): the OS data
# dir. The path differs per platform - Windows puts a `data` subdir under the app
# folder, Linux/macOS do not. Only the active .ron is removed; the .bak siblings
# are the player's own backups.
case "$(uname -s)" in
  MINGW* | MSYS* | CYGWIN*) save="$APPDATA/boardgame/data/boardgame.tableau.ron" ;;
  Darwin) save="$HOME/Library/Application Support/boardgame/boardgame.tableau.ron" ;;
  *) save="${XDG_DATA_HOME:-$HOME/.local/share}/boardgame/boardgame.tableau.ron" ;;
esac

if [ -f "$save" ]; then
  rm -f "$save"
  echo "cleared save: $save"
else
  echo "no save to clear (already a clean slate): $save"
fi

BOARDGAME_ORACLE=1 cargo run -p boardgame "$@"
