# Brief for fresh-eyes reviewers (gearcalc, branch audit-ablation @ 03f9823)

Repo: /home/user/gearcalc. Read CLAUDE.md first (the map + 6 standing rules). Code is unchanged
since the audit (a2f2234). The audit: audit/plan.md (6 phases, patterns P1–P11), audit/workstreams/T01–T21
(tasks with Change/Proof), audit/findings.md (809 findings table), audit/ledger.json (full records,
keys: id, scope, title, severity, locations, claim, evidence, proposal, verdict...).

The owner's goals: implement the audit and ablate every aspect; find what the audit MISSED; trust
but verify (be skeptical of both the code and the audit's claims, including its "What not to touch"
list); compact clean code with minimal branches; robust math with minimal discontinuities, closed
forms over numerical procedures, no magic numbers; unify functions toward universal models; rules of
thumb only as user-visible options; docs/comments clear to ANY reader, concise, accurate.

Rules for you:
- DO NOT edit the repo at /home/user/gearcalc. For experiments, `git -C /home/user/gearcalc worktree add
  --detach /home/user/.cache/gearcalc-work/wt/<name>` and remove it when done.
- Any cargo/npm build or test MUST go through the slot helper:
  `/home/user/.cache/gearcalc-work/gc <dir> <command...>` (it caps parallelism; RAM is limited; a
  watchdog kills processes >5 GB). Prefer reading and small probes over full-suite runs. Python scripts fine.
- Before reporting something as NEW, grep audit/ledger.json for it (by file, symbol, and keyword). Report
  only what is new, or where the audit is WRONG / its proposed fix is flawed. Cite ledger ids when relevant.
- Each finding: id, title, file:line, claim, evidence (what you ran or read), severity, proposed fix
  (structural cure preferred), and confidence. Write full results to
  /home/user/.cache/gearcalc-work/notes/<your-name>.md and return a summary of at most 400 words.
