# Working rule for this directory

These design notes are markdown-heavy and full of tables.

**After completing an edit task here — once no pending edit tasks remain — run the
table-padding script** so table columns stay aligned in monospace editors:

- PowerShell: `scripts/pad-tables.ps1`
- Bash: `scripts/pad-tables.sh`

(Repo root is `D:\keep\github\sean\boardgame`; the script locates itself, so the path
above works from any cwd.)

Run it **once at the end of a batch**, not after each individual edit. It recursively
re-aligns every markdown table in the repo
(`cargo run -p mdtable --example pad_tables`).
