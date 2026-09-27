# s1-p3 adversarial check (work/s1-p3, 9 commits on f079797)

Verdict so far: FAIL, on one defect (D1). The rest is minor. The review is otherwise nearly complete.

Method: the 9 commits were replayed onto audit-ablation, first at 97d51ce and then at f392f93, in a scratch worktree that has since been removed.
- At f392f93, 300cf73's wasm_boundary.json conflicted with P8's record. I took P8's version and did not regenerate it.
- Commit 2275f71 does not build on the current base: gear-cli/src/identity.rs:334 calls `gear_to_dxf(g.mean(), o)`, and it must become `gear_to_dxf(&g, o)`.
- With that one-line fix at 97d51ce, `tools/check_all.sh --fast` passed everything: clippy, fmt, nextest, bindings, golden, figures, units, wasm, and validate_dxf.

## Defects

D1 (T03.3, real, unclaimed move). `auto::minimum_profile_shift` (auto.rs ~130-160) looks for a fixed point with `residual(x) = x − x_min_of(tool_at(x))`.
- Once `Rack::deepest` caps the tool, its depth follows x (b_d is fixed), so the residual is constant. It has no root, and the code silently falls back to `unwrap_or(x0)`.
- Evidence: `gear-cli identity` against f392f93, default z43 at β45 (the crossed preset, 95 lines).
  - undercut moves from −5.1186 to −5.4100.
  - sharp_rack_undercut moves from −5.1186 to −7.4823.
  - So sharp < with_cutter_radius, which breaks the invariant asserted at auto.rs:2578 (`r.undercut < r.sharp_rack_undercut`).
- Physically this gear can no longer undercut at any shift, so a threshold should be typed absence, not a fallback number.
- Fix: make MinimumShift `Option` (None when the capped residual has no sign change), drop `unwrap_or(x0)`, and add z43 β45 to the invariant test.

D2 (T03.3, principle). The depth cap is taken at ρ_min/FILLET_FRACTION_OF_MAX, which is 0.95 used as a margin.
- At the cap there is a leftover root arc: half_pitch − θ0 = 3.0e-12 rad, about 2.6e-11 mm long, at z20 α30 h_f1.6 ρ0.
- That is barely above T04.1's zero-span law, which requires more than 1e-12·ra. validate_dxf's zero-length-span check (> 1e-6 mm) would reject a DXF of a capped gear.
- Derived fix: cap exactly at `deepest(st, ρ_min)`. When the cap fires, set θ0 = π/z by construction (the fillets meet on the space centreline, mirroring θ_a = 0 for a pointed tip), and let the walker skip the root arc when θ0 == half_pitch. That needs no factor.

D3 (minor, typed absence). New silent fallbacks in tooth.rs:
- `fillet_travel_at` ends in `.unwrap_or(0.0)`.
- `solve_junction` uses `brent(gap…).unwrap_or(s_base)`.

D4 (minor, validator). validate_dxf's ring mode takes the tip and root from the file's own min and max radius, so it derives neither from the inputs.
- A ring whose radii are wrong consistently still passes.
- Its root-half-angle formula a_c/r repeats the crate's assumption.

## Confirmed OK

- **T03.1 gate:** planted faults in the tooth (st +1e-3, b_d +1e-3, ρ +1e-3) each fail the gate, with penetration 4.1e-5, deviation 1.09e-3 and penetration 6.3e-5 respectively. A tip fault (+1e-3) passes the gate, as expected since the rack does not cut the tip, but 18 other gear-core tests catch it.
- **Continuity:** swept at steps of 1e-4 and 1e-5.
  - Tip through the base circle (z40, h_a) is continuous. `tip_below_form` begins at h_a −0.8024 with no jump. The only steep point is a √-singularity at r_a → r_f, which is out of range.
  - Depth through closure (z20 α30) and k through closure are continuous; r_f changes only by the step times the slope.
  - The z17 shift sweep is continuous.
- **validate_dxf** catches a flipped bulge, a dropped vertex (off-axis arc) and reversed winding, on g17, g9, the eccentric g24 and ring43.
- **Golden and matrix moves:** the worker explained these, and they look plausible.
- **Other identity moves** match the claims: addendum min −1.25, the dedendum ceiling, the Tooth `tool` field, DXF vertices, and changes at ulp level (u_tip, u_j, solve hashes).
- **Ring rb·(1+1e-9):** it already clamps with a note, so decision 1 is met in behaviour. The bare literal is a degeneracy ε that should be named in P4 (T05.15). Acceptable to defer.

## Unrun

- Base-vs-head re-run of every worker proof (I only spot-checked through the mutations).
- Regenerating wasm_boundary on f392f93.
- The external-gear DXF check at a capped depth, which would demonstrate D2.
