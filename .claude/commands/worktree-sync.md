---
description: Sync this worktree's branch with main in both directions — merge main in, verify, then fast-forward main up to it
argument-hint: "[integration branch — omit = main]"
---

Sync the branch checked out in THIS worktree with the integration branch (default
`main`) in **both directions**: first merge the integration branch's latest into this
branch, then fast-forward the integration branch up to this branch. This is how an
instance that owns a worktree picks up what others have already landed AND lands its
own work on `main`. Do the steps in order; STOP and report the moment a gate fails —
nothing broken or unresolved ever gets promoted onto the integration branch.

1. ORIENT — run `git branch --show-current` (call it **BR**) and `git worktree list`.
   - **INT** = the argument **$ARGUMENTS** if one was given, else `main`.
   - If BR == INT, STOP: you're sitting on the integration branch — nothing to sync.
   - If INT is checked out in another worktree (it appears in the list), STOP and name
     that worktree: INT can't be fast-forwarded while another worktree holds it.

2. CLEAN TREE — run `git status --porcelain`. If there is ANY output, STOP and tell me
   to commit (or stash) first, then wait. **NEVER** run `git add -A` or stage on my
   behalf: this is a shared repo, and a blind add can sweep another instance's
   uncommitted files into your commit (it has happened). Stage only your own files, by
   explicit path, and only when I ask.

3. PULL INT IN (direction 1) — `git merge --no-edit INT`.
   - On conflicts: STOP, list the conflicted files, and ask me how to resolve. Do not
     auto-resolve and do not proceed to step 4.

4. VERIFY (the gate before main) — run `scripts/verify` (fmt + clippy + tests + build).
   Report honestly: failures with their output, anything skipped. If it does not pass
   cleanly, STOP — a branch that fails verify must never be promoted to INT.

5. PROMOTE (direction 2) — after the merge, BR contains all of INT, so this is a
   fast-forward. First prove it: `git merge-base --is-ancestor <INT> HEAD` must succeed
   (INT is fully contained in HEAD). If it does NOT, STOP — something is off; do not
   clobber INT. If it does, run `git branch -f <INT> HEAD`, then confirm
   `git rev-parse <INT>` == `git rev-parse HEAD`.

6. REPORT — show `git log --oneline -3` for both BR and INT and confirm they point at
   the same commit. Publishing INT to a remote (`git push origin <INT>`) is a separate,
   explicitly-me-initiated step and is intentionally NOT done here.

Scope: the engine/balance crates are this instance's; never edit another instance's
crates or branch. See the worktree-roles memory and the `needs-merge/` guidance in
`.claude/CLAUDE.md`.
