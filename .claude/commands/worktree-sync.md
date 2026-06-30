---
description: Sync this worktree's branch with main in both directions — merge main in, verify, then fast-forward main up to it (main may be a live checkout)
argument-hint: "[integration branch — omit = main]"
---

Sync the branch checked out in THIS worktree with the integration branch (default
`main`) in **both directions**: first merge the integration branch's latest into this
branch, then fast-forward the integration branch up to this branch. After a clean run
the two point at the **same commit** — everything on this worktree is on `main`, and
this worktree has everything from `main`.

This works whether `main` is floating (checked out nowhere) **or** a live checkout you
edit and run directly. The only requirements at sync time — your discipline, since Git
can't enforce them — are: **no other instance is mid-sync on `main`**, and **`main`'s
checkout has no pending edits** (an uncommitted `main` isn't a true production snapshot,
and a dirty tree can block the fast-forward). Do the steps in order; STOP and report the
moment a gate fails — nothing broken or unresolved ever gets promoted onto `main`.

1. ORIENT — run `git branch --show-current` (call it **BR**) and `git worktree list`.
   - **INT** = the argument **$ARGUMENTS** if one was given, else `main`.
   - If BR == INT, STOP: you're sitting on the integration branch — nothing to sync.
   - Find INT's checkout: the worktree in the list whose branch is INT. Call its path
     **INTDIR**. If INT is checked out somewhere, that's expected (not an error) — INTDIR
     is where you'll promote. If INT is checked out **nowhere**, leave INTDIR empty (the
     promote falls back to a plain ref update in step 5).

2. CLEAN TREES — both the source and the target must be clean:
   - This worktree: `git status --porcelain`. Any output → STOP, tell me to commit (or
     stash) first, then wait.
   - INT's checkout (if INTDIR is set): `git -C <INTDIR> status --porcelain`. Any output →
     STOP and name the dirty files: `main` has pending edits, so it isn't safe to promote
     onto and the fast-forward may be blocked. Ask me to commit/stash them in that
     checkout first.
   - **NEVER** run `git add -A` or stage on my behalf, in either tree: this is a shared
     repo, and a blind add can sweep another instance's uncommitted files into a commit
     (it has happened). Stage only your own files, by explicit path, and only when I ask.

3. PULL INT IN (direction 1) — `git merge --no-edit <INT>` in this worktree. (If INT has
   its own commits — e.g. work done directly on `main` — this makes a merge commit that
   carries both lines; if INT is behind, it's a no-op.)
   - On conflicts: STOP, list the conflicted files, and ask me how to resolve. Do not
     auto-resolve and do not proceed to step 4.

4. VERIFY (the gate before main) — run `scripts/verify` (fmt + clippy + tests + build).
   Report honestly: failures with their output, anything skipped. If it does not pass
   cleanly, STOP — a branch that fails verify must never be promoted to INT.

5. PROMOTE (direction 2) — after step 3, BR contains all of INT, so advancing INT to BR
   is a fast-forward. First prove it: `git merge-base --is-ancestor <INT> HEAD` must
   succeed (INT is fully contained in HEAD). If it does NOT, STOP — something is off; do
   not clobber INT. Then advance INT by whichever applies:
   - **INT is checked out (INTDIR set):** `git -C <INTDIR> merge --ff-only <BR>`. This
     moves INT's ref *and* its working tree together — the safe way to advance a
     checked-out branch (and why `git branch -f` is refused for one). Requires INTDIR
     clean, checked in step 2.
   - **INT floats (INTDIR empty):** `git branch -f <INT> HEAD`.
   Confirm `git rev-parse <INT>` == `git rev-parse <BR>`.

6. REPORT — show `git log --oneline -3` for both BR and INT and confirm they point at
   the same commit. Publishing INT to a remote (`git push origin <INT>`) is a separate,
   explicitly-me-initiated step and is intentionally NOT done here.

Scope: the engine/balance crates are this instance's; never edit another instance's
crates or branch. The step-5 fast-forward only advances INT's ref and updates files that
differ between the old INT and BR — it does not touch unrelated work in INT's checkout.
See the worktree-roles memory and the `needs-merge/` guidance in `.claude/CLAUDE.md`.
