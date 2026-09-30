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
- Never leave a server (vite preview/dev) running: anything started under `gc` inherits its slot lock
  and blocks every other build until killed. Stop what you start before you finish.

## Rules added after Stage 1 (from work/stage1-exit.md §3; the checkers' recurring findings)
1. **A gate ships its own plants.** Its --self-test or law includes at least one NEAR-MISS fault that a
   naive or the previous version of the gate would pass. An assertion inside a loop or branch also
   asserts how many times it ran.
2. **A tolerance has two laws:** a rounding-level input passes; a fault at 10× the tolerance fails.
   Its value is an ε·operation-count expression with its derivation, a named user input, or a constant
   whose basis is stated. "Measured + x %" only in #[ignore]d canaries.
3. **Absence is typed.** No new `unwrap_or(<literal>)`, `map_or(<literal>, …)`, or INFINITY/NAN used as
   a value unless the line says `// absence: <why>`. A solver Option that "cannot" be None gets a
   debug_assert!.
4. **Land on a bound by construction** (the bracket end with the needed sign, or tagging by
   construction). No retry, nudge or grow loop; a loop that is truly needed has a derived trip count
   and a law that it exits on the first pass.
5. **Kinds.** A new `match kind`, ring branch or per-kind default ships a law over both kinds and a
   continuity law across the switch. A default is a rule per gear, never per kind or birth site.
6. **Close each package with a proof table** (law | fails at base (test file only) | passes at head).
   A fixture that turns a shipped default off names it, and the law also runs at the defaults. Each
   exception list names the stage that empties it.
7. **Before handing to the checker:** report the work-count diff and run a small
   `cargo mutants --in-diff` sample on the package's own lines (cargo-mutants is in the dev shell once
   Stage 2's tooling lands; until then use /nix/store's cargo-mutants 27.1.0 — find it with
   `ls -d /nix/store/*cargo-mutants*`).
- cargo-mutants: run it through tools/mutants.sh (1 job) or with `--jobs 1` and NEXTEST_TEST_THREADS=1 plus
  a per-test timeout. Mutants that break termination grow without bound; running several at once
  starves the machine. A mutant killed by the watchdog counts as caught.
- **Owner's principle on validators:** an independent check has lasting value only if it applies a
  fundamentally DIFFERENT method (a BEM against a closed form, an exact solution, published data, a
  different parser). A same-method mirror in another language only checks language consistency. It
  carries its own upkeep and passes its flaws on. Do not add mirrors; prefer conventional laws. When
  porting from a prototype, never loosen the port to match a prototype flaw: decide from the physics.
- Never wait with `until ! pgrep -f "<pattern>"` where the pattern appears in the waiting shell's own
  command line: the loop matches itself and never exits. Run the command in the background and wait
  on its completion notification instead.
