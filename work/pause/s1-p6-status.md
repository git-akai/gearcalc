# s1-p6 status (worktree /home/user/.cache/gearcalc-work/wt/s1-p6, branch work/s1-p6)

## Done
- T08.1 — 80668aa. LoadCase::application_factor (K_A, serde default 1; held at 1 below 1 with
  train.application_factor_held); rated torque = mesh torque × K_A in shape.rs `rate` (reported
  torques unchanged); train.stresses_nominal on every enabled solved case; docs (strength.rs
  RootStressModel/bending_stress docs, rationale.md#no-isoagma-correction-factors, state.md
  Known-approximate). Panel field, strings ×5, gear-io change log, bindings, golden + wasm record
  re-written (only the note lines and trainfile's application_factor lines moved). Full suite,
  clippy, fmt, check_strings/doc_links/units/figures green.
  NB: rationale.md quotes the canary pair 8.48 -> 10.60 mm (fatigue contact vs the 750 fatigue
  allowable). T08.6 changes the contact allowable: update that sentence and pin the test
  `the_canary_at_a_light_application_factor_asks_more_than_its_face` with an override.

## In progress: T08.2 (WIP commit on top of 80668aa)
- Only `explore_t08_2` (print-only) in crates/gear-core/src/train/rating_laws.rs. Delete it and
  write the real failing fixtures. Findings so far at base: pair([12,40]) member 1 (h_a 1.6,
  x fixed 0.5, pressure_angle 25 on both, no_sharp_tip=false, LinearRamp) -> Some(NaN).
  pair([300,300]) with h_a 1.9 / h_f 2.15 on one member -> the OTHER member gets -0.26 MPa and
  negative min width (audit says -1.30; settings differ — maybe h_a 1.9 on a member whose mate
  is the unmodified one, or sharing/torque; not resolved). pair([20,80]) h_a 1.4/28°/x 1.0 did
  not reproduce 4.29e15 (try on member 1, or x on both, or sharp tip).
- Remaining for T08.2: Bending::new fires out-of-band note at eps_n>=2 regardless of model
  (reword mesh.load_sharing_out_of_band ×5); bending_factor -> None unless finite and > 0
  (moment arm <= 0 -> None in stress_correction); shape.rs ~3236 NoRootSection refusal ->
  per-gear None + new note (then drop TrainError::NoRootSection + error.train_no_root_section ×5
  if no other site); pointed-tip sweep (blow-up is likely the flank-tangency candidate near the
  apex: root_chord -> 0) — start sweep where land finite or rate on fillet tangency with a note;
  worst_over_cycle and Lewis `reduce` skip non-finite, use total_cmp. Laws: every preset and
  arrangement with addendum 1.0–1.8 on one member -> every bending stress/min width None or
  finite > 0; pointed teeth below eps 2: shared/unshared <= 1.002 and converges with samples.

## Not started: T08.3, T08.4, T08.6
- Follow audit/workstreams/T08-strength-allowables.md; plan decisions 2,3,5,8; PC-9 (C(κ) not
  closed form: bracketed 1-D max in solve.rs or tabulate; keep 1.79/1.60 limits as the test).
- web/node_modules installed by `npm ci` in this worktree (not a symlink).
