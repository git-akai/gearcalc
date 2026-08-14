import os, re, textwrap
from gearbuild import build

OUT = "/mnt/user-data/outputs"
os.makedirs(OUT, exist_ok=True)
INPUTS = {"Module","PressureAngle","Teeth","ProfileShift","HelixAngle",
          "Addendum","Dedendum","RootRadius"}

# comments: no apostrophes (would clash with the ' comment symbol), no '='
COMMENT = {
 "_K":"degrees per radian, bridges trig args and arc/radius ratios",
 "_CosB":"cosine of the helix angle, does the normal to transverse conversion",
 "_AlphaT":"TRANSVERSE pressure angle, pressure angle guarded to at least 0.5 deg",
 "_R":"pitch reference radius",
 "_Rb":"base radius",
 "_Bd":"cutter tip depth below the rolling line, guarded positive. Root MINOR radius is _R minus _Bd",
 "_St":"transverse tooth thickness at the pitch circle",
 "_PsiB":"half tooth angle at the base circle",
 "_Rho":"cutter tip radius, clamped so it always fits the tooth space",
 "_Bc":"depth of the tip round CENTRE below the rolling line",
 "_Ac":"lateral offset of the tip round centre",
 "_Ra":"tip MAJOR radius, clamped against a pointed tooth",
 "_Utip":"involute roll parameter at the tip",
 "_L":"undercut indicator: negative means the gear is undercut",
 "_U0":"involute roll parameter at the trochoid junction",
 "_R2":"junction radius on the involute",
 "_Cj":"junction solve: find D whose trochoid radius matches _R2",
 "_Dn1":"Newton step 1, the seed is exact whenever the gear is not undercut",
 "_Dn4":"Newton step 4, junction closes to about 3e-9 mm across the whole input range",
 "_S0":"trochoid generator parameter at the junction",
 "_KS0":"trochoid point at the junction, across the tooth",
 "_KY0":"trochoid point at the junction, along the tooth",
 "_R1":"junction radius reached along the trochoid, matches _R2 to about 3e-9",
 "_Th1":"junction angle on the trochoid",
 "_ThA":"half angular width of the tip arc",
}

SYNTAX = {"atan(": "atn("}          # evaluator's spelling of the inverse tangent

def syn(t):
    for a, b in SYNTAX.items():
        t = t.replace(a, b)
    return t

def quoted(expr, names):
    """Wrap every constant / input variable reference in double quotes."""
    return re.sub(r"[A-Za-z_][A-Za-z0-9_]*",
                  lambda m: '"%s"' % m.group(0) if m.group(0) in names else m.group(0),
                  expr)

def consts_text(C):
    names = {n for n, _ in C} | INPUTS
    out = []
    for n, e in C:
        line = '"%s" = %s' % (n, quoted(e, names))
        c = COMMENT.get(n)
        if c:
            line += " '" + c
        out.append(line)
    return "\n".join(out) + "\n"

built = {m: build(m) for m in ("deg", "rad")}
tags  = {"deg": "degrees", "rad": "radians"}
for m, tag in tags.items():
    C, X, Y, _ = built[m]
    names = {n for n, _ in C} | INPUTS
    open(os.path.join(OUT, "constants_%s.txt" % tag), "w").write(syn(consts_text(C)))
    open(os.path.join(OUT, "x_of_T_%s.txt" % tag), "w").write(syn(quoted(X, names)) + "\n")
    open(os.path.join(OUT, "y_of_T_%s.txt" % tag), "w").write(syn(quoted(Y, names)) + "\n")

Cd, Xd, Yd, Pd = built["deg"]
nd = {n for n, _ in Cd} | INPUTS
L = []
L.append("# Closed-form involute gear cross-section -- x(T), y(T)\n")
L.append("**Angle unit: DEGREES.** Trig calls take degrees, inverse trig calls are assumed to "
         "return degrees. A radian build is included too (`*_radians.txt`); the two agree to "
         "5e-13 mm.\n")
L.append("Independent variable **T**, range **0 -> Teeth**. One unit of T = one tooth: root arc "
         "-> trochoid -> involute flank -> tip arc -> involute flank -> trochoid -> root arc. "
         "Tooth 0 is centred on +X, the curve runs counter-clockwise and closes exactly.\n")
L.append("\n## File format\n")
L.append(textwrap.dedent("""\
    * `constants_*.txt` -- one definition per line, `"Name" = expression 'comment`.
    * `x_of_T_*.txt` / `y_of_T_*.txt` -- exactly one line, no whitespace.
    * Every constant and input reference is quoted, in the definitions and in the
      formulas alike. No alignment padding. No comment contains an apostrophe or an
      `=`, so splitting each line on its first `'` and first `=` is safe.
    * `pi`, `T`, function names and numerals are never quoted -- they are language
      tokens, not referenced variables.
    """))
L.append("\n## 1. New input variable\n```\n\"RootRadius\" = 0.38\n```")
L.append(textwrap.dedent("""\
    Cutter tip radius as a multiple of the **normal** module. A real cutting-tool parameter:
    it sets the tip rounding of the generating rack, and the root fillet is the trochoid that
    rounding sweeps out. Typical 0.25-0.39 (0.38 is the ISO 53 basic rack); 0 gives a
    sharp-cornered rack. It is clamped automatically so it can never exceed what the tooth
    space accepts, so any value >= 0 is safe.
    """))
L.append("\n## 2. Constants to create (in this order)\n")
L.append("Each references only inputs and earlier entries.\n")
L.append("```\n" + consts_text(Cd) + "```")
L.append("\n## 3. Structure of the final formulas\n")
L.append("These blocks depend on T so they cannot be stored as variables; they are written out "
         "in full in section 4. This section is for reading and debugging.\n")
L.append("```")
L.append("A   sawtooth core, A/180 recovers the position within the tooth")
L.append("      " + quoted(Pd["A"], nd) + "\n")
L.append("W   half-tooth parameter, 0 at the tip centre, 1 at mid tooth-space")
L.append("      " + quoted(Pd["W"], nd) + "\n")
L.append("V1..V5  clamped local parameter of each section, clamp01((W - start)/width)")
for i, k in enumerate(("V1", "V2", "V3", "V4", "V5"), 1):
    L.append("      %s = %s" % (k, quoted(Pd[k].replace(Pd["W"], "W"), nd)))
L.append("")
L.append("U   involute roll parameter  = " + quoted(Pd["U"].replace(Pd["V2"], "V2"), nd))
L.append("S   trochoid generator param = " + quoted(Pd["S"].replace(Pd["V4"], "V4"), nd))
L.append("K   tip round offset factor  = " + quoted(Pd["K"].replace(Pd["S"], "S"), nd) + "\n")
th = Pd["THETA"]
for a, b in (("V1", Pd["V1"]), ("U", Pd["U"]), ("V3", Pd["V3"]), ("K", Pd["K"]),
             ("S", Pd["S"]), ("V5", Pd["V5"])):
    th = th.replace(b, a)
L.append("THETA  half-profile angle, 0 at tip centre and _HalfP at mid-space")
L.append("      " + quoted(th, nd) + "\n")
rd = Pd["RAD"]
for a, b in (("U", Pd["U"]), ("V3", Pd["V3"]), ("K", Pd["K"]), ("S", Pd["S"])):
    rd = rd.replace(b, a)
L.append("RAD    radius")
L.append("      " + quoted(rd, nd) + "\n")
L.append("PHI    global polar angle")
L.append("      " + quoted(Pd["PHI"].replace(Pd["A"], "A").replace(Pd["THETA"], "THETA"), nd) + "\n")
L.append("x(T) = RAD*cos(PHI)")
L.append("y(T) = RAD*sin(PHI)")
L.append("```")
L.append("\n## 4. Final formulas -- DEGREES\n")
L.append("Single line each, identical to the delivered `.txt` files.\n")
L.append("### x(T)\n```\n" + quoted(Xd, nd) + "\n```\n")
L.append("### y(T)\n```\n" + quoted(Yd, nd) + "\n```\n")
L.append("\n## 5. Notes\n")
L.append(textwrap.dedent("""\
    * **T range 0 -> Teeth.** ~200 samples per tooth is smooth, ~60 is the practical floor.
    * The five sections take fixed shares of each unit of T (0.12 tip arc, 0.60 flank,
      0.01 bridge, 0.22 trochoid, 0.05 root arc) -- these are the literal numbers in the
      formulas. Arc-length-adaptive shares would need 5 more variables for a negligible
      gain, so they were dropped to fit the variable budget.
    * Only 30 constants are defined. Quantities you may want for checking are derived:
      root (minor) radius is `_R - _Bd`, tip (major) radius is `_Ra`, base radius `_Rb`,
      pitch radius `_R`. `_L` going negative tells you the gear is undercut.
    * `_Eps` moves the sawtooth poles off the integers. Keep it near 1e-9; never 0, and no
      smaller than about 1e-12.
    * `sgn(0)` is never relied on: wherever the sign flips, the quantity it multiplies is 0.
    * Degrees vs radians is **not** a plain `pi -> 180` substitution. Arc/radius ratios
      (`_PsiP`, `_Th0`, the `/_Rd` terms) and the involute function `tan(a) - a` are
      intrinsically radian quantities and carry explicit `_K = 180/pi` factors. Use the
      supplied radian files rather than editing the degree ones by hand.
    * Normal-module system: `_Mt` and `_AlphaT` do the transverse conversion, so the result
      is the true transverse section of the helical gear.
    * Degenerate inputs are clamped, not rejected: root radius forced positive, tip radius
      capped at the pointed-tooth limit, fillet capped to what the space allows.
    """))
open(os.path.join(OUT, "gear_formulas.md"), "w").write(syn("\n".join(L)))
print("emitted")
