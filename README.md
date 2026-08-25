# Gears

A browser tool for designing gears and geartrains: parameter calculation and
optimisation, 2D profile visualisation, DXF export, and geartrains saved to and
loaded from human-readable TOML.

External, **internal**, **worm** and **planetary** gearing, with the mathematics
derived rather than quoted — and the derivations checked against simulations that share no code
with them.

All mathematics is Rust, compiled to WebAssembly. TypeScript and Svelte do
layout and event handling only.

> **Project rule: no engineering calculation in TypeScript.** If a number appears
> in the UI, Rust computed it. TypeScript may format it for display and nothing
> else. That is what keeps the Rust test suite meaningful. Defaults count as
> numbers: they cross the boundary from Rust, because the one that was written
> down in both languages drifted — see DESIGN §12.

The architecture, the mathematics behind every formula, and the verification log
are in [`docs/DESIGN.md`](docs/DESIGN.md). Read that before changing anything in
`gear-core`. [`docs/HANDOFF.md`](docs/HANDOFF.md) is the shorter route in: current
state, the rules and why they are rules, the traps, and what the next milestone
needs.

## Layout

| Path | Role |
|---|---|
| `crates/gear-core` | All mathematics. No I/O, no UI, no wasm. Depends only on `serde`. |
| `crates/gear-io` | File formats: DXF export, and the TOML material library and geartrain documents. |
| `crates/gear-wasm` | The WebAssembly boundary. JSON in, JSON out. |
| `crates/gear-cli` | Development harness — drive the mathematics without a browser. |
| `web/` | Svelte 5 + TypeScript + Vite front end. |
| `docs/` | Design document, handoff, the initial specification, and the JGMA 116-02 tables. |
| `handoff_inbound/` | Prior Python work. **Reference only** — do not build on it. |

## Getting started

Everything runs inside the Nix dev shell, which pins the Rust toolchain, Node,
and `wasm-bindgen-cli` together.

```bash
nix develop              # or `direnv allow` once, for automatic entry

cargo nextest run        # the full test suite, 351 tests, ~27 s
cargo clippy --all-targets -- --deny warnings
cargo fmt

nix flake check          # everything CI checks: build, clippy, fmt, tests
```

### Driving the mathematics without a browser

This is the fastest way to see what the core is doing.

```bash
cargo run --bin gear-cli -- show 17 0.2   # derived geometry for z=17, x=+0.2
cargo run --bin gear-cli -- sweep         # scan a grid for undercut and clamps
cargo run --bin gear-cli -- materials     # the material library and its provenance
cargo run --bin gear-cli -- strength 17 43 2.0   # a worked mesh: bending, contact, efficiency
cargo run --bin gear-cli -- strength 17 43 2.0 '4340 Hardened Steel' 20   # the same, helical
cargo run --bin gear-cli -- train                  # a geartrain, end to end
cargo run --bin gear-cli -- trainfile              # a geartrain to TOML and back
cargo run --bin gear-cli -- train mixed            # ...with a worm stage in it
cargo run --bin gear-cli -- worm 1 40 7 90         # a worm pair, both directions
cargo run --bin gear-cli -- wormstage 1 40 7 2     # a worm stage, end to end
cargo run --bin gear-cli -- planetary 17 17 3      # every ring count that can work
cargo run --bin gear-cli -- planetstage 24 18 60 3 # a planetary stage, all six modes
cargo run --release --bin gear-cli -- verify 100   # two-sided cutter check
```

`gear-cli strength 17 43 2.0` is the project's regression canary: its figures
have not moved since milestone 5, through every refactor since.

### A note on the material library

`crates/gear-io/data/materials_default.toml` is the one place in this project
where numbers are not derived from first principles, and it says so. Published
data does not exist for every property of every material — no polyamide
datasheet gives Poisson's ratio, and none gives any fatigue figure at all — so
some values are estimates. **Every value carries a `basis`** recording whether
it was read from a datasheet, derived from other published values, read off a
chart, or estimated. `gear-cli materials` prints that as a column; the UI is
expected to surface it too. Treat the estimates as starting points to be
overridden, not as authority.

### The web application

```bash
cd web
npm install
npm run dev              # rebuilds the wasm, then serves with hot reload
```

`npm run build:wasm` alone regenerates `web/src/wasm/` after a change to
`gear-core` or `gear-wasm`; `npm run dev` and `npm run build` do it for you.

For a reproducible production build:

```bash
nix build .#web          # deployable static site in ./result
```

## Notes for anyone changing the geometry

`gear-core::profile` is a port of a Python implementation that was validated to
5e-4 mm against a full simulation of the generating rack, and the port reproduces
it to 7.5e-14 mm over a 1188-case grid. Three things in it look like they could
be tidied and must not be:

1. **The flank continues below the base circle** to its true intersection with the
   trochoid. Clamping it there and bridging the gap — the obvious-looking
   approach — leaves a visible 0.3 mm step on undercut gears.
   `Gear::with_legacy_clamp` reproduces that fault on purpose, as a negative test
   fixture; if `legacy_clamp_still_shows_the_junction_step…` ever passes trivially,
   the *detection* has broken.
2. **The fillet fit cap** is `w_tip·cos α / (2(1 − sin α))`. The plausible
   `w_tip / (2 cos α)` is wrong and silently shrinks the fillet on every
   profile-shifted gear.
3. **`theta` is not monotone** along the profile. Undercut gears are legitimately
   re-entrant. The correct invariant is monotone *radius*.

4. **A rack's figures do not carry to a pinion cutter.** A 0.38-module tip round
   is comfortable on a rack, whose tooth is wide at its tip; on a 20-tooth shaper
   with a 1.25 addendum the tip is 0.377 mm wide and two such rounds cannot both
   live on it. `ShaperCut` refuses that tool rather than clamping it.

Any change here must keep `cargo nextest run` green — in particular the two
simulations, which are the ones that catch what self-consistent tests miss:
`profile_is_bounded_from_both_sides_by_the_cutter`, which checks 1080 external
gears against the cutter that would make them from both sides at once, and
`the_generated_profile_is_the_shape_the_cutter_would_leave`, which does the same
for an internal gear by sweeping a pinion cutter through the rolling motion and
comparing the envelope it leaves.

## Verification tooling

Two scripts exist to check the Rust against something that shares no code with
it, and both are run by hand rather than in CI:

```bash
python3 tools/validate_dxf.py <file.dxf> ...   # read an export back with ezdxf
python3 tools/worm_flank_curvature.py          # worm flank curvature from the surface itself
```

The second also answers a design question — what choosing a ZI, ZN or ZA worm
flank actually costs — and its answer is in `docs/DESIGN.md` §4.5.1.
