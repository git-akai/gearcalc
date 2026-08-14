"""
Closed-form single-parameter involute gear profile.
Builds the literal formula strings (allowed op/function set only),
then evaluates those exact strings to validate.
"""
import re, math
import numpy as np

# ---------- allowed-syntax helpers -------------------------------------------
def mx(a, b):   # branchless max
    return "(({0}+{1}+abs({0}-{1}))/2)".format(a, b)
def mn(a, b):   # branchless min
    return "(({0}+{1}-abs({0}-{1}))/2)".format(a, b)

# ---------- constant table (ordered, dependency-safe) ------------------------
CONSTS = [
 ("_Eps",     "0.000000001"),
 # --- guarded raw inputs ---
 ("_PA",      mx("PressureAngle", "0.5")),                       # deg, keep >0
 ("_AlphaN",  "(_PA*pi/180)"),
 ("_Beta",    "(HelixAngle*pi/180)"),
 ("_CosB",    "cos(_Beta)"),
 ("_Mt",      "(Module/_CosB)"),                                  # transverse module
 ("_TanAn",   "tan(_AlphaN)"),
 ("_AlphaT",  "atan(_TanAn/_CosB)"),                              # transverse press. angle
 ("_CosAt",   "cos(_AlphaT)"),
 ("_SinAt",   "sin(_AlphaT)"),
 ("_TanAt",   "(_SinAt/_CosAt)"),
 # --- primary radii ---
 ("_R",       "(_Mt*Teeth/2)"),                                   # pitch radius
 ("_Rb",      "(_R*_CosAt)"),                                     # base radius
 ("_BdRaw",   "(Module*(Dedendum-ProfileShift))"),                # cutter depth below roll line
 ("_Bd",      mn(mx("_BdRaw", "(0.05*Module)"), "(0.9*_R)")),
 ("_Rf",      "(_R-_Bd)"),                                        # root radius
 # --- tooth thickness ---
 ("_StRaw",   "(Module*(pi/2+2*ProfileShift*_TanAn)/_CosB)"),
 ("_St",      mn(mx("_StRaw", "(0.02*Module)"), "(1.9*_R*pi/Teeth)")),
 ("_PsiP",    "(_St/(2*_R))"),
 ("_InvAt",   "(_TanAt-_AlphaT)"),
 ("_PsiB",    mx("(_PsiP+_InvAt)", "0.000001")),
 # --- cutter tip radius, clamped so it always fits the space ---
 ("_RhoW",    "(RootFilletCoef*Module/_CosB)"),
 ("_RhoL1",   "(0.95*_Bd)"),
 ("_RhoL2",   "(0.95*(pi*_Mt-_St-2*_Bd*_TanAt)/(2*_CosAt))"),
 ("_Rho",     mx(mn("_RhoW", mn("_RhoL1", "_RhoL2")), "(0.001*Module)")),
 ("_Bc",      "(_Bd-_Rho)"),                                      # tip-round centre depth
 ("_Ac",      "(_St/2+_Bc*_TanAt+_Rho/_CosAt)"),                  # tip-round centre offset
 # --- tip radius, clamped against a pointed tooth ---
 ("_AlphaY",  "(exp(log(3*_PsiB)/3)-0.4*_PsiB)"),                 # inverse involute (approx)
 ("_RaMax",   "(_Rb/cos(_AlphaY))"),
 ("_RaN",     "(_R+Module*(Addendum+ProfileShift))"),
 ("_Ra",      mx(mn("_RaN", "_RaMax"), "(_Rb*1.000001)")),
 ("_Utip",    "sqr((_Ra/_Rb)^2-1)"),
 # --- lower limit of the generated involute ---
 ("_L",       "(_R*_SinAt-_Bc/_SinAt-_Rho)"),
 ("_U0",      mn("((_L+abs(_L))/(2*_Rb))", "(0.98*_Utip)")),
 ("_R2",      "(_Rb*sqr(1+_U0^2))"),                              # junction radius
 ("_Th2",     "(_PsiB-(_U0-atan(_U0)))"),
 # --- trochoid parameter at the junction: solve r(D)=_R2 (exact when not undercut) ---
 ("_Cj",      "(_R2^2-_R^2+2*_R*_Bc)"),
 ("_Ej",      "(2*_R*_Bc*_Rho)"),
 ("_Dq0",     "(_Bc/_SinAt)"),
 ("_Dq1",     "(0-_Rho+sqr(" + mx("(_Cj+_Ej/_Dq0)", "0.000001") + "))"),
 ("_Dq2",     "(0-_Rho+sqr(" + mx("(_Cj+_Ej/_Dq1)", "0.000001") + "))"),
 ("_Dq3",     "(0-_Rho+sqr(" + mx("(_Cj+_Ej/_Dq2)", "0.000001") + "))"),
 ("_Dj",      mx("_Dq3", "(_Bc*1.0000001)")),
 ("_S0",      "(0-sqr(_Dj^2-_Bc^2))"),                            # trochoid start
 ("_K0",      "((_Dj+_Rho)/_Dj)"),
 ("_KS0",     "(_K0*_S0)"),
 ("_KY0",     "(_R-_K0*_Bc)"),
 ("_R1",      "sqr(_KS0^2+_KY0^2)"),
 ("_Th1",     "(atan(_KS0/_KY0)-(_S0-_Ac)/_R)"),
 ("_ThA",     "(_PsiB-(_Utip-atan(_Utip)))"),                     # half tip-arc angle
 ("_Th0",     "(_Ac/_R)"),                                        # angle where trochoid meets root
 ("_HalfP",   "(pi/Teeth)"),
 # --- parameter budget per section (arc-length weighted) ---
 ("_G1",      mx("(_Ra*_ThA)", "0"),),
 ("_G2",      mx("(_Ra-_R2)", "0"),),
 ("_G3",      mx("(_R1-_Rf)", "0"),),
 ("_G4",      mx("(_Rf*(_HalfP-_Th0))", "0"),),
 ("_Gs",      "(_G1+_G2+_G3+_G4+0.000001)"),
 ("_D1",      "(0.01+0.95*_G1/_Gs)"),   # tip arc
 ("_D2",      "(0.01+0.95*_G2/_Gs)"),   # involute flank
 ("_D3",      "0.01"),                  # bridge
 ("_D4",      "(0.01+0.95*_G3/_Gs)"),   # trochoid
 ("_D5",      "(0.01+0.95*_G4/_Gs)"),   # root arc
 ("_P2",      "_D1"),
 ("_P3",      "(_D1+_D2)"),
 ("_P4",      "(_D1+_D2+_D3)"),
 ("_P5",      "(_D1+_D2+_D3+_D4)"),
]

# ---------- the T-dependent expression ---------------------------------------
A  = "atan(cotan(pi*(T+_Eps)))"                 # sawtooth core
W  = "(2*abs({A})/pi)".format(A=A)              # half-tooth parameter, 0=tip 1=mid-space

def clamp01(q):
    return "((abs({q})-abs(({q})-1)+1)/2)".format(q=q)

V1 = clamp01("({W}/_D1)".format(W=W))
V2 = clamp01("(({W}-_P2)/_D2)".format(W=W))
V3 = clamp01("(({W}-_P3)/_D3)".format(W=W))
V4 = clamp01("(({W}-_P4)/_D4)".format(W=W))
V5 = clamp01("(({W}-_P5)/_D5)".format(W=W))

U  = "(_Utip+{V2}*(_U0-_Utip))".format(V2=V2)                 # involute roll parameter
S  = "(_S0*(1-{V4}))".format(V4=V4)                           # trochoid generator parameter
K  = "(1+_Rho/sqr(({S})^2+_Bc^2))".format(S=S)
KS = "(({K})*({S}))".format(K=K, S=S)
KY = "(_R-({K})*_Bc)".format(K=K)

THETA = ("(_ThA*{V1}"
         "+(_PsiB-(({U})-atan({U}))-_ThA)"
         "+{V3}*(_Th1-_Th2)"
         "+(atan(({KS})/({KY}))-(({S})-_Ac)/_R-_Th1)"
         "+{V5}*(_HalfP-_Th0))").format(V1=V1, U=U, V3=V3, KS=KS, KY=KY, S=S, V5=V5)

RAD   = ("(_Rb*sqr(1+({U})^2)"
         "+{V3}*(_R1-_R2)"
         "+(sqr(({KS})^2+({KY})^2)-_R1))").format(U=U, V3=V3, KS=KS, KY=KY)

PHI   = ("((2*pi/Teeth)*(T+_Eps-0.5+({A})/pi)-sgn({A})*{TH})").format(A=A, TH=THETA)

XFORM = "({R})*cos({P})".format(R=RAD, P=PHI)
YFORM = "({R})*sin({P})".format(R=RAD, P=PHI)

# ---------- evaluation harness ------------------------------------------------
def to_py(s):
    s = s.replace("^", "**")
    s = re.sub(r"\bsqr\(", "np.sqrt(", s)
    s = re.sub(r"\bcotan\(", "_cot(", s)
    s = re.sub(r"\bcosec\(", "_csc(", s)
    s = re.sub(r"\bsec\(", "_sec(", s)
    s = re.sub(r"\batan\(", "np.arctan(", s)
    s = re.sub(r"\barcsin\(", "np.arcsin(", s)
    s = re.sub(r"\barccos\(", "np.arccos(", s)
    s = re.sub(r"\bsin\(", "np.sin(", s)
    s = re.sub(r"\bcos\(", "np.cos(", s)
    s = re.sub(r"\btan\(", "np.tan(", s)
    s = re.sub(r"\bexp\(", "np.exp(", s)
    s = re.sub(r"\blog\(", "np.log(", s)
    s = re.sub(r"\babs\(", "np.abs(", s)
    s = re.sub(r"\bsgn\(", "np.sign(", s)
    s = s.replace("np.np.", "np.")
    s = re.sub(r"\bpi\b", "np.pi", s)
    return s

ENV = {"np": np, "_cot": lambda v: 1.0/np.tan(v), "_csc": lambda v: 1.0/np.sin(v),
       "_sec": lambda v: 1.0/np.cos(v)}

def evaluate(inputs, T):
    env = dict(ENV); env.update(inputs); env["T"] = T
    for name, expr in CONSTS:
        env[name] = eval(to_py(expr), {"__builtins__": {}}, env)
    x = eval(to_py(XFORM), {"__builtins__": {}}, env)
    y = eval(to_py(YFORM), {"__builtins__": {}}, env)
    return x, y, env

def consts_report(inputs):
    _, _, env = evaluate(inputs, np.array([0.5]))
    return {k: env[k] for k, _ in CONSTS}

if __name__ == "__main__":
    inp = dict(Module=1.0, PressureAngle=20.0, Teeth=17, ProfileShift=0.2,
               HelixAngle=0.0, Addendum=1.0, Dedendum=1.25, RootFilletCoef=0.38)
    T = np.linspace(0, inp["Teeth"], 17*400+1)
    x, y, env = evaluate(inp, T)
    r = np.hypot(x, y)
    print("len XFORM =", len(XFORM), " len YFORM =", len(YFORM))
    print("NaNs:", np.isnan(x).sum(), np.isnan(y).sum())
    print("r min/max:", r.min(), r.max(), " Rf/Ra:", env["_Rf"], env["_Ra"])
    d = np.hypot(np.diff(x), np.diff(y))
    print("max step:", d.max(), " mean step:", d.mean())
    print("closure err:", math.hypot(x[0]-x[-1], y[0]-y[-1]))
