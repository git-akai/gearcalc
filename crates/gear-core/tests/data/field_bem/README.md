# The across strip by boundary elements: a reference that shares no method with the strip

`across_bem.json` holds, for nine strips, the peak pressure and the load share on the edge as a
boundary-element solver computes them: contact-verify5's own 2-D frictionless half-plane solver
(`bem2d.py`, copied here byte for byte: piecewise-constant pressure on a uniform grid, midpoint
collocation, the exact log-kernel panel integral, the Toeplitz system by Levinson's recursion, a
contiguous active set grown and shrunk until the pressure is non-negative inside and nothing
penetrates outside). It shares no code and no method with `crates/gear-core/src/field/across.rs`,
which solves the same problem in closed form piece by piece.

`bem_across.py` wrote the file, once (`python3 bem_across.py across_bem.json`, about two minutes
on nine processes): each strip's gap is the exact piecewise-quadratic `h(t)` of its curvature
steps, solved at 1000, 2000, 4000 and 8000 panels over `[m − 1.25 c, m + 1.25 c]` (the
prototype's `m` and `c` only place the window; the solver's active set is its own, and it refuses
a contact that reaches the window's edge). Each figure is extrapolated by Richardson from the last
three resolutions at the order they show (or is the finest value where they show none); its
`uncertainty` is the change of that extrapolation from the first three resolutions to the last
three, the solver's own measure of how far it is from converged. The strips: eight of the oracle's
`across.json` (Hertz, Hertz with a flank, one or two rounds with tip relief, all-edge strips, the
verifier's two-round strip) and one section that is concave beyond its round (`k₀ < 0`, the ring's
valley in `gap.json`).

The test (`the_boundary_elements_agree`, in `across.rs`) holds the strip to each figure within ten
times that figure's uncertainty, and reads no figure whose uncertainty is above `1e-6` of it (the
two-round strip's peak, `1e-5`, where the peak sits on a curvature step and the three-panel parabola
that reads it converges slowly).

| File | md5 |
|---|---|
| `across_bem.json` | `bbd889c3d7f9d4d5bba0736ffb3b79db` |
| `bem2d.py` | `deb017f9271b0c641c77b33a29e914fb` |
| `bem_across.py` | `59ec7781cf3d9640a7d8978b987d47ab` |

`bem_across.py` reads `bem2d.py` from contact-verify5's directory and the prototype's `trace.py`
(for the window only) from `contact-proto/`, both outside the repository; it does not run from here.
