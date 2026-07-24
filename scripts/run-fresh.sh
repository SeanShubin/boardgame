#!/usr/bin/env bash
# Run the card-table app from a CLEAN SLATE - the same pristine table "Start Over"
# resets to, but from launch. It discards the persisted session by deleting the
# active save (the app then loads the fresh sample table); your own ".bak" backups
# next to it are left alone. Extra args pass through to cargo, e.g.
# scripts/run-fresh.sh --release
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

cargo run -p boardgame "$@"
