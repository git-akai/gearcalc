"""
Closed-form single-parameter involute gear profile.
Builds the literal formula strings (allowed op/function set only) in either
radian mode or degree mode, then evaluates those exact strings to validate.

Degree mode assumes the evaluator's trig take degrees and its inverse trig
return degrees.  Every place where an angle is really an arc/radius ratio, or
where the involute function inv(a)=tan(a)-a appears, carries an explicit
_K = 180/pi conversion.
"""
import re, math
import numpy as np

def mx(a, b):   # branchless max
    return "(({0}+{1}+abs({0}-{1}))/2)".format(a, b)
def mn(a, b):   # branchless min
    return "(({0}+{1}-abs({0}-{1}))/2)".format(a, b)


def nstep(p):
    """One Newton step on (D+rho)^2 - Ej/D - Cj = 0, clamped into the valid domain."""
    return mx("({p}-(({p}+_Rho)^2-_Ej/{p}-_Cj)/(2*({p}+_Rho)+_Ej/{p}^2))".format(p=p),
              "(_Bc*1.0000001)")


def build_full(mode="deg"):
    """Return (CONSTS, XFORM, YFORM, PARTS) for the requested angle mode."""
    D = (mode == "deg")
    HALF = "180" if D else "pi"
    a2u  = (lambda e: "(({})*_K)".format(e)) if D else (lambda e: "({})".format(e))
    inv  = (lambda u: "({0}*_K-atan({0}))".format(u)) if D else (lambda u: "({0}-atan({0}))".format(u))

    C = [
     ("_Eps",    "0.000000001"),
     ("_K",      "(180/pi)" if D else "1"),
     ("_PA",     mx("PressureAngle", "0.5")),
     ("_AlphaN", "_PA" if D else "(_PA*pi/180)"),
     ("_Beta",   "HelixAngle" if D else "(HelixAngle*pi/180)"),
     ("_CosB",   "cos(_Beta)"),
     ("_Mt",     "(Module/_CosB)"),
     ("_TanAn",  "tan(_AlphaN)"),
     ("_AlphaT", "atan(_TanAn/_CosB)"),
     ("_CosAt",  "cos(_AlphaT)"),
     ("_SinAt",  "sin(_AlphaT)"),
     ("_TanAt",  "(_SinAt/_CosAt)"),
     ("_R",      "(_Mt*Teeth/2)"),
     ("_Rb",     "(_R*_CosAt)"),
     ("_BdRaw",  "(Module*(Dedendum-ProfileShift))"),
     ("_Bd",     mn(mx("_BdRaw", "(0.05*Module)"), "(0.9*_R)")),
     ("_Rf",     "(_R-_Bd)"),
     ("_StRaw",  "(Module*(pi/2+2*ProfileShift*_TanAn)/_CosB)"),
     ("_St",     mn(mx("_StRaw", "(0.02*Module)"), "(1.9*_R*pi/Teeth)")),
     ("_PsiP",   a2u("_St/(2*_R)")),
     ("_InvAt",  "(_TanAt*_K-_AlphaT)" if D else "(_TanAt-_AlphaT)"),
     ("_PsiB",   mx("(_PsiP+_InvAt)", "0.000001")),
     ("_RhoW",   "(RootRadius*Module/_CosB)"),
     ("_RhoL1",  "(0.95*_Bd)"),
     ("_RhoL2",  "(0.95*(pi*_Mt-_St-2*_Bd*_TanAt)*_CosAt/(2*(1-_SinAt)))"),
     ("_Rho",    mx(mn("_RhoW", mn("_RhoL1", "_RhoL2")), "(0.000001*Module)")),
     ("_Bc",     "(_Bd-_Rho)"),
     ("_Ac",     "(_St/2+_Bc*_TanAt+_Rho/_CosAt)"),
     ("_PsiBr",  "(_PsiB/_K)" if D else "_PsiB"),
     ("_AlphaY", a2u("exp(log(3*_PsiBr)/3)-0.4*_PsiBr")),
     ("_RaMax",  "(_Rb/cos(_AlphaY))"),
     ("_RaN",    "(_R+Module*(Addendum+ProfileShift))"),
     ("_Ra",     mx(mn("_RaN", "_RaMax"), "(_Rb*1.000001)")),
     ("_Utip",   "sqr((_Ra/_Rb)^2-1)"),
     ("_L",      "(_R*_SinAt-_Bc/_SinAt-_Rho)"),
     ("_U0",     mn("((_L+abs(_L))/(2*_Rb))", "(0.98*_Utip)")),
     ("_R2",     "(_Rb*sqr(1+_U0^2))"),
     ("_Th2",    "(_PsiB-" + inv("_U0") + ")"),
     ("_Cj",     "(_R2^2-_R^2+2*_R*_Bc)"),
     ("_Ej",     "(2*_R*_Bc*_Rho)"),
     ("_Dq0",    "(_Bc/_SinAt)"),
     ("_Dn1",    nstep("_Dq0")),
     ("_Dn2",    nstep("_Dn1")),
     ("_Dn3",    nstep("_Dn2")),
     ("_Dn4",    nstep("_Dn3")),
     ("_Dn5",    nstep("_Dn4")),
     ("_Dj",     "_Dn5"),
     ("_S0",     "(0-sqr(_Dj^2-_Bc^2))"),
     ("_K0",     "((_Dj+_Rho)/_Dj)"),
     ("_KS0",    "(_K0*_S0)"),
     ("_KY0",    "(_R-_K0*_Bc)"),
     ("_R1",     "sqr(_KS0^2+_KY0^2)"),
     ("_Rd",     "(_R/_K)" if D else "_R"),
     ("_Th1",    "(atan(_KS0/_KY0)-(_S0-_Ac)/_Rd)"),
     ("_ThA",    "(_PsiB-" + inv("_Utip") + ")"),
     ("_Th0",    "(_Ac/_Rd)"),
     ("_HalfP",  "({}/Teeth)".format(HALF)),
     ("_G1",     mx("(_Ra*_ThA/_K)", "0")),
     ("_G2",     mx("(_Ra-_R2)", "0")),
     ("_G3",     mx("(_R1-_Rf)", "0")),
     ("_G4",     mx("(_Rf*(_HalfP-_Th0)/_K)", "0")),
     ("_Gs",     "(_G1+_G2+_G3+_G4+0.000001)"),
     ("_D1",     "(0.01+0.95*_G1/_Gs)"),
     ("_D2",     "(0.01+0.95*_G2/_Gs)"),
     ("_D3",     "0.01"),
     ("_D4",     "(0.01+0.95*_G3/_Gs)"),
     ("_D5",     "(0.01+0.95*_G4/_Gs)"),
     ("_P2",     "_D1"),
     ("_P3",     "(_D1+_D2)"),
     ("_P4",     "(_D1+_D2+_D3)"),
     ("_P5",     "(_D1+_D2+_D3+_D4)"),
    ]

    A = "atan(cotan({H}*(T+_Eps)))".format(H=HALF)
    W = "(2*abs({A})/{H})".format(A=A, H=HALF)
    c01 = lambda q: "((abs({q})-abs(({q})-1)+1)/2)".format(q=q)
    V1 = c01("({W}/_D1)".format(W=W))
    V2 = c01("(({W}-_P2)/_D2)".format(W=W))
    V3 = c01("(({W}-_P3)/_D3)".format(W=W))
    V4 = c01("(({W}-_P4)/_D4)".format(W=W))
    V5 = c01("(({W}-_P5)/_D5)".format(W=W))

    U  = "(_Utip+{V2}*(_U0-_Utip))".format(V2=V2)
    S  = "(_S0*(1-{V4}))".format(V4=V4)
    KK = "(1+_Rho/sqr(({S})^2+_Bc^2))".format(S=S)
    KS = "(({K})*({S}))".format(K=KK, S=S)
    KY = "(_R-({K})*_Bc)".format(K=KK)

    THETA = ("(_ThA*{V1}"
             "+(_PsiB-{INV}-_ThA)"
             "+{V3}*(_Th1-_Th2)"
             "+(atan(({KS})/({KY}))-(({S})-_Ac)/_Rd-_Th1)"
             "+{V5}*(_HalfP-_Th0))").format(V1=V1, INV=inv("(" + U + ")"),
                                            V3=V3, KS=KS, KY=KY, S=S, V5=V5)
    RAD   = ("(_Rb*sqr(1+({U})^2)"
             "+{V3}*(_R1-_R2)"
             "+(sqr(({KS})^2+({KY})^2)-_R1))").format(U=U, V3=V3, KS=KS, KY=KY)
    PHI   = ("((2*{H}/Teeth)*(T+_Eps-0.5+({A})/{H})-sgn({A})*{TH})"
             ).format(H=HALF, A=A, TH=THETA)

    X = "({R})*cos({P})".format(R=RAD, P=PHI)
    Y = "({R})*sin({P})".format(R=RAD, P=PHI)
    parts = dict(A=A, W=W, V1=V1, V2=V2, V3=V3, V4=V4, V5=V5,
                 U=U, S=S, K=KK, THETA=THETA, RAD=RAD, PHI=PHI)
    return C, X, Y, parts



def build(mode="deg", nsteps=4, wts=(0.12, 0.60, 0.01, 0.22, 0.05)):
    """Compact variable set: same maths as build_full, fewer named constants.

    Inlined vs build_full: _Eps _PA _AlphaN _Beta _Mt _TanAn _TanAt _BdRaw _StRaw
    _PsiP _InvAt _RhoW _RhoL1 _RhoL2 _PsiBr _AlphaY _RaMax _RaN _Dq0 _K0 _Dj _Rf
    _Rd _Th0 _HalfP _G1.._G4 _Gs _D1.._D5 _P2.._P5.
    Sampling weights are fixed constants rather than arc-length adaptive.
    """
    D = (mode == "deg")
    HALF = "180" if D else "pi"
    K    = "*_K" if D else ""
    PAg  = "((PressureAngle+0.5+abs(PressureAngle-0.5))/2)"
    AN   = PAg if D else "(" + PAg + "*pi/180)"
    BE   = "HelixAngle" if D else "(HelixAngle*pi/180)"
    TAT  = "(_SinAt/_CosAt)"
    inv  = (lambda u: "({0}*_K-atan({0}))".format(u)) if D else (lambda u: "({0}-atan({0}))".format(u))
    perR = (lambda e: "(({0})*_K/_R)".format(e)) if D else (lambda e: "(({0})/_R)".format(e))
    ang  = (lambda e: "(({0})*_K)".format(e)) if D else (lambda e: "({0})".format(e))

    C = []
    if D:
        C.append(("_K", "(180/pi)"))
    C += [
     ("_CosB",   "cos({})".format(BE)),
     ("_AlphaT", "atan(tan({})/_CosB)".format(AN)),
     ("_CosAt",  "cos(_AlphaT)"),
     ("_SinAt",  "sin(_AlphaT)"),
     ("_R",      "(Module*Teeth/(2*_CosB))"),
     ("_Rb",     "(_R*_CosAt)"),
     ("_Bd",     mn(mx("(Module*(Dedendum-ProfileShift))", "(0.05*Module)"), "(0.9*_R)")),
     ("_St",     mn(mx("(Module*(pi/2+2*ProfileShift*tan({}))/_CosB)".format(AN),
                       "(0.02*Module)"), "(1.9*_R*pi/Teeth)")),
     ("_PsiB",   mx("({}+({}*_K-_AlphaT))".format(ang("_St/(2*_R)"), TAT) if D
                    else "({}+({}-_AlphaT))".format(ang("_St/(2*_R)"), TAT), "0.000001")),
     ("_Rho",    mx(mn("(RootRadius*Module/_CosB)",
                       mn("(0.95*_Bd)",
                          "(0.95*(pi*Module/_CosB-_St-2*_Bd*{})*_CosAt/(2*(1-_SinAt)))".format(TAT))),
                    "(0.000001*Module)")),
     ("_Bc",     "(_Bd-_Rho)"),
     ("_Ac",     "(_St/2+_Bc*{}+_Rho/_CosAt)".format(TAT)),
     ("_Ra",     mx(mn("(_R+Module*(Addendum+ProfileShift))",
                       "(_Rb/cos({}))".format(
                           ang("exp(log(3*(_PsiB/_K))/3)-0.4*(_PsiB/_K)") if D
                           else "(exp(log(3*_PsiB)/3)-0.4*_PsiB)")),
                    "(_Rb*1.000001)")),
     ("_Utip",   "sqr((_Ra/_Rb)^2-1)"),
     ("_L",      "(_R*_SinAt-_Bc/_SinAt-_Rho)"),
     ("_U0",     mn("((_L+abs(_L))/(2*_Rb))", "(0.98*_Utip)")),
     ("_R2",     "(_Rb*sqr(1+_U0^2))"),
     ("_Cj",     "(_R2^2-_R^2+2*_R*_Bc)"),
     ("_Ej",     "(2*_R*_Bc*_Rho)"),
    ]
    prev = "(_Bc/_SinAt)"
    for i in range(1, nsteps + 1):
        C.append(("_Dn%d" % i, nstep(prev)))
        prev = "_Dn%d" % i
    Dj = prev
    KF = "(({0}+_Rho)/{0})".format(Dj)
    C += [
     ("_S0",   "(0-sqr({}^2-_Bc^2))".format(Dj)),
     ("_KS0",  "({}*_S0)".format(KF)),
     ("_KY0",  "(_R-{}*_Bc)".format(KF)),
     ("_R1",   "sqr(_KS0^2+_KY0^2)"),
     ("_Th1",  "(atan(_KS0/_KY0)-{})".format(perR("_S0-_Ac"))),
     ("_ThA",  "(_PsiB-" + inv("_Utip") + ")"),
    ]

    w1, w2, w3, w4, w5 = wts
    p2, p3, p4, p5 = w1, w1+w2, w1+w2+w3, w1+w2+w3+w4
    f = lambda v: repr(round(v, 6))
    A = "atan(cotan({H}*(T+0.000000001)))".format(H=HALF)
    W = "(2*abs({A})/{H})".format(A=A, H=HALF)
    c01 = lambda q: "((abs({q})-abs(({q})-1)+1)/2)".format(q=q)
    V1 = c01("({W}/{d})".format(W=W, d=f(w1)))
    V2 = c01("(({W}-{p})/{d})".format(W=W, p=f(p2), d=f(w2)))
    V3 = c01("(({W}-{p})/{d})".format(W=W, p=f(p3), d=f(w3)))
    V4 = c01("(({W}-{p})/{d})".format(W=W, p=f(p4), d=f(w4)))
    V5 = c01("(({W}-{p})/{d})".format(W=W, p=f(p5), d=f(w5)))
    U  = "(_Utip+{V}*(_U0-_Utip))".format(V=V2)
    S  = "(_S0*(1-{V}))".format(V=V4)
    KK = "(1+_Rho/sqr(({S})^2+_Bc^2))".format(S=S)
    KS = "(({K})*({S}))".format(K=KK, S=S)
    KY = "(_R-({K})*_Bc)".format(K=KK)
    TH0 = perR("_Ac")
    THETA = ("(_ThA*{V1}+(_PsiB-{INV}-_ThA)+{V3}*(_Th1-{TH2})"
             "+(atan(({KS})/({KY}))-{TR}-_Th1)+{V5}*({H}/Teeth-{TH0}))").format(
                 V1=V1, INV=inv("(" + U + ")"), V3=V3, KS=KS, KY=KY,
                 TH2="(_PsiB-" + inv("_U0") + ")",
                 TR=perR("(" + S + ")-_Ac"), V5=V5, H=HALF, TH0=TH0)
    RAD = ("(_Rb*sqr(1+({U})^2)+{V3}*(_R1-_R2)+(sqr(({KS})^2+({KY})^2)-_R1))"
           ).format(U=U, V3=V3, KS=KS, KY=KY)
    PHI = ("((2*{H}/Teeth)*(T+0.000000001-0.5+({A})/{H})-sgn({A})*{TH})"
           ).format(H=HALF, A=A, TH=THETA)
    X = "({R})*cos({P})".format(R=RAD, P=PHI)
    Y = "({R})*sin({P})".format(R=RAD, P=PHI)
    return C, X, Y, dict(A=A, W=W, V1=V1, V2=V2, V3=V3, V4=V4, V5=V5,
                         U=U, S=S, K=KK, THETA=THETA, RAD=RAD, PHI=PHI)


def _env(deg):
    if deg:
        f = np.deg2rad; g = np.rad2deg
        return {"np": np,
                "_sin": lambda v: np.sin(f(v)), "_cos": lambda v: np.cos(f(v)),
                "_tan": lambda v: np.tan(f(v)), "_cot": lambda v: 1.0/np.tan(f(v)),
                "_atan": lambda v: g(np.arctan(v))}
    return {"np": np, "_sin": np.sin, "_cos": np.cos, "_tan": np.tan,
            "_cot": lambda v: 1.0/np.tan(v), "_atan": np.arctan}

def to_py(s):
    s = s.replace("^", "**")
    s = re.sub(r"(?<![A-Za-z_])sqr\(",   "np.sqrt(", s)
    s = re.sub(r"(?<![A-Za-z_])cotan\(", "_cot(",    s)
    s = re.sub(r"(?<![A-Za-z_])atan\(",  "_atan(",   s)
    s = re.sub(r"(?<![A-Za-z_])sin\(",   "_sin(",    s)
    s = re.sub(r"(?<![A-Za-z_])cos\(",   "_cos(",    s)
    s = re.sub(r"(?<![A-Za-z_])tan\(",   "_tan(",    s)
    s = re.sub(r"(?<![A-Za-z_])exp\(",   "np.exp(",  s)
    s = re.sub(r"(?<![A-Za-z_])log\(",   "np.log(",  s)
    s = re.sub(r"(?<![A-Za-z_])abs\(",   "np.abs(",  s)
    s = re.sub(r"(?<![A-Za-z_])sgn\(",   "np.sign(", s)
    s = re.sub(r"\bpi\b", "np.pi", s)
    return s

def evaluate(inputs, T, mode="deg", which="compact", **kw):
    C, X, Y, _ = (build(mode, **kw) if which == "compact" else build_full(mode))
    env = _env(mode == "deg"); env.update(inputs); env["T"] = T
    for name, expr in C:
        env[name] = eval(to_py(expr), {"__builtins__": {}}, env)
    return (eval(to_py(X), {"__builtins__": {}}, env),
            eval(to_py(Y), {"__builtins__": {}}, env), env)


if __name__ == "__main__":
    inp = dict(Module=1.0, PressureAngle=20.0, Teeth=17, ProfileShift=0.2,
               HelixAngle=0.0, Addendum=1.0, Dedendum=1.25, RootRadius=0.38)
    T = np.linspace(0, inp["Teeth"], 17*400+1)
    for m in ("rad", "deg"):
        x, y, env = evaluate(inp, T, m); r = np.hypot(x, y)
        print("%s: nan=%d rmin=%.10f rmax=%.10f closure=%.2e chars=%d"
              % (m, np.isnan(x).sum(), r.min(), r.max(),
                 math.hypot(x[0]-x[-1], y[0]-y[-1]), len(build(m)[1])))
    xr, yr, _ = evaluate(inp, T, "rad"); xd, yd, _ = evaluate(inp, T, "deg")
    print("max |deg - rad| =", np.nanmax(np.hypot(xr-xd, yr-yd)))
