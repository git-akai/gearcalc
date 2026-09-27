# Worker conventions (gearcalc)

Read /home/user/gearcalc/CLAUDE.md and /home/user/gearcalc/work/plan.md (esp. §2 decisions) first.
The audit tasks you cite are in /home/user/gearcalc/audit/workstreams/Tnn-*.md; review reports in work/review/.

- Work ONLY in your own worktree: `git -C /home/user/gearcalc worktree add -b work/<name> /home/user/.cache/gearcalc-work/wt/<name> audit-ablation`.
  Never edit /home/user/gearcalc itself; never push; never merge. Commit on your branch.
- Every cargo/npm/nix command goes through `/home/user/.cache/gearcalc-work/gc <dir> <cmd...>` (two build
  slots, capped threads; RAM is limited; a watchdog kills processes >5 GB). Use a separate target dir per
  worktree is NOT needed — cargo in the worktree uses its own ./target (first build ~ a few minutes).
- Proof first: write each law/test, run it at the base (your branch start) and SEE IT FAIL (or, for a pure
  instrument, show it detects a planted fault), then make it pass. Record the before/after in the commit message.
- Principles: no magic numbers (derive or name with basis), minimal branches, typed absence (Option/named
  refusal, never 0/NaN/±inf standing for "none"), refusals as Note keys present in all 5
  crates/gear-io/data/strings_*.toml, no English in gear-core, no engineering in TS. Comments: short, plain,
  present tense, say what and why for any reader; no narrative.
- Before finishing: cargo nextest run, cargo clippy --all-targets -- --deny warnings, cargo fmt --check,
  and any tools/check_* your change touches (tools/check_golden.sh; --write only if a moved number is
  intended and explained). Commit messages end with:
  Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>
  Claude-Session: https://claude.ai/code/session_01Kg7MJVizaYjStsN4g5BXFw
- Leave the worktree in place (the orchestrator integrates it). Return <=300 words: commits (hash + subject),
  what failed before / passes after, anything deferred, and any new finding (with evidence).
- Never symlink anything (e.g. web/node_modules) from /home/user/gearcalc into your worktree: tools like
  `npm ci` delete through the link. Run `npm ci` inside your own worktree's web/ instead.
