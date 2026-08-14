"""Emit evaluator-friendly constant files: quoted variable names, no padding, ' comments."""
import os, re
from gearbuild import build

OUT = "/mnt/user-data/outputs"
INPUTS = ["Module", "PressureAngle", "Teeth", "ProfileShift", "HelixAngle",
          "Addendum", "Dedendum", "RootFilletCoef"]
FUNCS = {"sin","cos","tan","sec","cosec","cotan","arcsin","arccos","atan",
         "arcsec","arccosec","arccotan","abs","exp","log","sqr","sgn","pi"}

def quoter(names):
    """Wrap every variable reference in quotes; leave functions, pi, T and numbers alone."""
    vs = set(names) | set(INPUTS)
    pat = re.compile(r"[A-Za-z_][A-Za-z0-9_]*")
    def go(expr):
        return pat.sub(lambda m: '"%s"' % m.group(0) if m.group(0) in vs else m.group(0), expr)
    return go

COMMENT = {
 "_Eps":"keeps the sawtooth off its poles at integer T - do not set to 0",
 "_K":"degrees per radian - bridges trig args and arc/radius ratios",
 "_PA":"pressure angle guarded to >= 0.5 (a 0 deg gear is undefined)",
 "_AlphaN":"normal pressure angle",
 "_Beta":"helix angle",
 "_Mt":"TRANSVERSE module",
 "_AlphaT":"TRANSVERSE pressure angle",
 "_R":"pitch (reference) radius",
 "_Rb":"base radius",
 "_Bd":"cutter tip depth below the rolling line (guarded > 0)",
 "_Rf":"root / MINOR radius",
 "_St":"transverse tooth thickness at the pitch circle",
 "_PsiP":"half tooth angle at the pitch circle",
 "_InvAt":"involute function inv(alpha_t) = tan - alpha",
 "_PsiB":"half tooth angle at the base circle",
 "_RhoW":"wanted cutter tip radius",
 "_Rho":"cutter tip radius, clamped so it always fits the space",
 "_Bc":"depth of the tip-round CENTRE below the rolling line",
 "_Ac":"lateral offset of the tip-round centre",
 "_AlphaY":"inverse involute (approx) for the pointed-tooth limit",
 "_Ra":"tip / MAJOR radius, clamped against a pointed tooth",
 "_Utip":"involute roll parameter at the tip",
 "_L":"base-tangent distance to the involute lower limit (< 0 = undercut)",
 "_U0":"involute roll parameter at the trochoid junction",
 "_R2":"junction radius, on the involute",
 "_Cj":"junction solve: find D where r(D) = _R2",
 "_Dq0":"seed - exact whenever the gear is not undercut",
 "_Dn1":"Newton step 1",
 "_Dn2":"Newton step 2",
 "_Dn3":"Newton step 3",
 "_Dn4":"Newton step 4",
 "_Dn5":"Newton step 5 - converged to ~1e-11 across the input range",
 "_S0":"trochoid generator parameter at the junction",
 "_R1":"junction radius along the trochoid (= _R2 to ~1e-10)",
 "_Rd":"pitch radius scaled so length/_Rd lands in the angle unit",
 "_Th1":"junction angle on the trochoid",
 "_ThA":"half angular width of the tip arc",
 "_Th0":"angle where the trochoid meets the root circle",
 "_HalfP":"half angular pitch",
 "_G1":"sampling budget: approximate arc length of each section",
 "_D1":"share of each unit of T: tip arc",
 "_D2":"share of each unit of T: involute flank",
 "_D3":"share of each unit of T: bridge",
 "_D4":"share of each unit of T: trochoid",
 "_D5":"share of each unit of T: root arc",
}

def consts_text(C, q):
    out = []
    for n, e in C:
        line = '"%s" = %s' % (n, q(e))
        c = COMMENT.get(n)
        if c:
            line += " '" + c
        out.append(line)
    return "\n".join(out) + "\n"

for tag in ("degrees", "radians"):
    C, X, Y, _ = build("deg" if tag == "degrees" else "rad")
    q = quoter([n for n, _ in C])
    open(os.path.join(OUT, "constants_%s.txt" % tag), "w").write(consts_text(C, q))
    # bare-reference formulas stay exactly as validated
    open(os.path.join(OUT, "x_of_T_%s.txt" % tag), "w").write(X + "\n")
    open(os.path.join(OUT, "y_of_T_%s.txt" % tag), "w").write(Y + "\n")
    if tag == "degrees":   # extra: same formulas with quoted references
        open(os.path.join(OUT, "x_of_T_degrees_quotedrefs.txt"), "w").write(q(X) + "\n")
        open(os.path.join(OUT, "y_of_T_degrees_quotedrefs.txt"), "w").write(q(Y) + "\n")
print(open(os.path.join(OUT, "constants_degrees.txt")).read()[:700])

# ---- spec document -----------------------------------------------------------
import textwrap
Cd, Xd, Yd, Pd = build("deg")
qd = quoter([n for n, _ in Cd])
L = []
L.append("# Closed-form involute gear cross-section -- x(T), y(T)\n")
L.append("**Angle unit: DEGREES.** A radian build is included too (`*_radians.txt`); the two "
         "agree to 5e-13 mm. Constants files use quoted variable names and `'` comments.\n")
L.append("Independent variable **T**, range **0 -> Teeth**. One unit of T = one tooth: "
         "root arc -> trochoid -> involute flank -> tip arc -> involute flank -> trochoid "
         "-> root arc. Tooth 0 is centred on +X, the curve runs counter-clockwise and "
         "closes exactly.\n")
L.append("\n## Files\n")
L.append("| file | contents |\n|---|---|")
L.append("| `constants_degrees.txt` | 71 constants, in dependency order |")
L.append("| `x_of_T_degrees.txt` / `y_of_T_degrees.txt` | the two formulas, single line each |")
L.append("| `*_quotedrefs.txt` | same formulas, variable references quoted (see note 6) |")
L.append("| `constants_radians.txt`, `x/y_of_T_radians.txt` | radian build |")

L.append("\n## New input variable\n```")
L.append('"RootFilletCoef" = 0.38')
L.append("```")
L.append(textwrap.dedent("""\
    Cutter tip radius as a multiple of the **normal** module -- a real cutting-tool
    parameter. It sets the tip rounding of the generating rack, and the root fillet is the
    trochoid that rounding sweeps out. Typical 0.25-0.39 (0.38 = ISO 53 basic rack); `0`
    gives a sharp-cornered rack. Clamped automatically to what the tooth space can accept,
    so any value >= 0 is safe.
    """))

L.append("\n## Constants (create in this order)\n```\n" + consts_text(Cd, qd) + "```")

L.append("\n## Structure of the formulas\n")
L.append("These depend on T so they cannot be stored as variables; they appear expanded in "
         "the formula files. Shown here unquoted, for reading.\n")
L.append("```")
L.append("A   sawtooth core; A/180 gives position within the tooth")
L.append("      " + Pd["A"] + "\n")
L.append("W   half-tooth parameter: 0 at tip centre, 1 at mid tooth-space")
L.append("      " + Pd["W"] + "\n")
L.append("V1..V5   clamp01((W - section start)/section width)")
L.append("      V1 = " + Pd["V1"])
L.append("      V2 = " + Pd["V2"].replace(Pd["W"], "W"))
L.append("      V3 = " + Pd["V3"].replace(Pd["W"], "W"))
L.append("      V4 = " + Pd["V4"].replace(Pd["W"], "W"))
L.append("      V5 = " + Pd["V5"].replace(Pd["W"], "W") + "\n")
L.append("U   involute roll parameter   = " + Pd["U"].replace(Pd["V2"], "V2"))
L.append("S   trochoid generator param  = " + Pd["S"].replace(Pd["V4"], "V4"))
L.append("K   tip-round offset factor   = " + Pd["K"].replace(Pd["S"], "S") + "\n")
L.append("THETA   half-profile angle (0 at tip centre, _HalfP at mid-space)")
L.append("      " + Pd["THETA"].replace(Pd["V1"],"V1").replace(Pd["U"],"U")
         .replace(Pd["V3"],"V3").replace(Pd["K"],"K").replace(Pd["S"],"S")
         .replace(Pd["V5"],"V5") + "\n")
L.append("RAD     radius")
L.append("      " + Pd["RAD"].replace(Pd["U"],"U").replace(Pd["V3"],"V3")
         .replace(Pd["K"],"K").replace(Pd["S"],"S") + "\n")
L.append("PHI     global polar angle")
L.append("      " + Pd["PHI"].replace(Pd["A"],"A").replace(Pd["THETA"],"THETA") + "\n")
L.append("x(T) = RAD*cos(PHI)")
L.append("y(T) = RAD*sin(PHI)")
L.append("```")

L.append("\n## Notes\n")
L.append(textwrap.dedent("""\
    1. **T range 0 -> Teeth.** ~200 samples per tooth is smooth; ~60 is the practical floor.
    2. Each section gets a share of every unit of T proportional to its approximate arc
       length (`_D1.._D5`), so point density stays even around the profile.
    3. `_Eps` moves the sawtooth's poles off the integers. Keep it near 1e-9; never 0, and
       no smaller than ~1e-12.
    4. `sgn(0)` is never relied on: wherever the sign flips, the quantity it multiplies is 0.
    5. Degrees vs radians is **not** a `pi -> 180` substitution. Arc/radius ratios
       (`_PsiP`, `_Th0`, the `/_Rd` terms) and the involute function `tan(a) - a` are
       intrinsically radian quantities and carry explicit `_K = 180/pi` factors. Use the
       supplied radian files rather than editing the degree ones.
    6. Quoting: constants files quote every variable reference, including inside
       expressions. The formula files are supplied both bare and with quoted references;
       `T`, `pi` and function names are never quoted.
    7. Normal-module system: `_Mt` and `_AlphaT` do the transverse conversion, so this is
       the true transverse section of the helical gear.
    8. Degenerate inputs are clamped rather than rejected -- root radius forced positive,
       tip radius capped at the pointed-tooth limit, fillet capped to the available space.
       The curve stays valid and closed.
    """))
open(os.path.join(OUT, "gear_formulas.md"), "w").write("\n".join(L))
print("md written")
