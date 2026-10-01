"""contact-verify5: an own 2-D frictionless half-plane contact solver, sharing no code with the prototype.
Piecewise-constant pressure on a uniform grid, midpoint collocation, the exact log-kernel panel integral,
the Toeplitz structure solved by Levinson's recursion (O(n^2)), a contiguous active set grown/shrunk until
p >= 0 inside and no penetration outside.  Plane strain, combined modulus Es (1/Es = sum (1-nu^2)/E)."""
import math

def _lnint(u0, u1):                       # int_{u0}^{u1} ln|u| du
    f = lambda u: 0.0 if u == 0.0 else u * math.log(abs(u)) - u
    return f(u1) - f(u0)

def levinson(r, b):
    """Solve T x = b, T symmetric Toeplitz with first row r (r[0] != 0); Golub-Van Loan 4.7.2 on T/r[0]."""
    n = len(b); r0 = r[0]; rr = [v / r0 for v in r]; bb = [v / r0 for v in b]
    if n == 1: return [bb[0]]
    y = [-rr[1]]; x = [bb[0]]; beta = 1.0; alpha = -rr[1]
    for k in range(1, n):
        beta = (1.0 - alpha * alpha) * beta
        mu = (bb[k] - sum(rr[1 + i] * x[k - 1 - i] for i in range(k))) / beta
        x = [x[i] + mu * y[k - 1 - i] for i in range(k)] + [mu]
        if k < n - 1:
            alpha = -(rr[k + 1] + sum(rr[1 + i] * y[k - 1 - i] for i in range(k))) / beta
            y = [y[i] + alpha * y[k - 1 - i] for i in range(k)] + [alpha]
    return x

def solve(h, q, Es, xl, xr, M, seed=None, maxit=200):
    """Contact of a gap profile h(x) (>= 0, rigid) at line load q over the window [xl, xr] with M panels.
    Returns dict(x, p, a, b, pmax, xpmax, delta)."""
    D = (xr - xl) / M; xs = [xl + D * (i + 0.5) for i in range(M)]; hs = [h(x) for x in xs]
    k = -2.0 / (math.pi * Es)
    r = [k * _lnint(-0.5 * D - i * D, 0.5 * D - i * D) for i in range(M)]   # influence of panel j on point i, |i-j| = idx
    i0 = min(range(M), key=lambda i: hs[i])
    if seed: lo, hi = seed
    else:                                   # Hertz estimate from the local curvature at the minimum
        j = min(max(i0, 1), M - 2); kap = max((hs[j - 1] - 2 * hs[j] + hs[j + 1]) / (D * D), 1e-12)
        aH = math.sqrt(4 * q / (math.pi * Es * kap)); w = max(int(0.8 * aH / D), 2)
        lo, hi = max(i0 - w, 0), min(i0 + w, M - 1)
    for it in range(maxit):
        n = hi - lo + 1; rr = r[:n]
        x1 = levinson(rr, [1.0] * n); x0 = levinson(rr, [-hs[i] for i in range(lo, hi + 1)])
        dl = (q / D - sum(x0)) / sum(x1)
        p = [a + dl * b for a, b in zip(x0, x1)]
        nlo, nhi = lo, hi
        while nlo < nhi - 2 and p[nlo - lo] < 0: nlo += 1
        while nhi > nlo + 2 and p[nhi - lo] < 0: nhi -= 1
        if (nlo, nhi) == (lo, hi):
            def gap(i):
                return hs[i] + sum(p[j - lo] * r[abs(i - j)] for j in range(lo, hi + 1)) - dl
            while nlo > 0 and gap(nlo - 1) < 0: nlo -= 1
            while nhi < M - 1 and gap(nhi + 1) < 0: nhi += 1
        if (nlo, nhi) == (lo, hi): break
        lo, hi = nlo, nhi
    else:
        raise RuntimeError('active set did not settle')
    if lo == 0 or hi == M - 1: raise RuntimeError('contact reaches the window edge')
    ip = max(range(n), key=lambda i: p[i])
    # parabolic refinement of the peak on three panels
    pm, xm = p[ip], xs[lo + ip]
    if 0 < ip < n - 1:
        a_, b_, c_ = p[ip - 1], p[ip], p[ip + 1]; den = a_ - 2 * b_ + c_
        if den < 0:
            s = 0.5 * (a_ - c_) / den; pm = b_ - 0.25 * (a_ - c_) * s; xm = xs[lo + ip] + s * D
    return dict(x=xs[lo:hi + 1], p=p, a=xs[lo] - 0.5 * D, b=xs[hi] + 0.5 * D, pmax=pm, xpmax=xm, delta=dl, lohi=(lo, hi))

def rich(f, M):
    """f(M) -> value; Richardson on M, 2M (order estimated from M, 2M, 4M)."""
    v1, v2, v4 = f(M), f(2 * M), f(4 * M)
    d1, d2 = v2 - v1, v4 - v2
    order = math.log(abs(d1 / d2), 2) if d2 != 0 and d1 / d2 > 0 else float('nan')
    ex = v4 + d2 / (2 ** order - 1) if order == order and order > 0.3 else v4
    return v1, v2, v4, order, ex

# ---------------------------------------------------------------------------- exact C1 profiles
def profile_round_flank(R1, re, R2, tT):
    """Exact gap h(t) between body 1 (circle R1, convex) and body 2 = a round of radius re joined C1 to a
    flank circle of radius R2 at tangency abscissa tT (t < tT: round side if tT >= 0 ... see below).
    Common tangent y = 0 at t = 0, the rigid touching point.  tT >= 0: the touch is on the ROUND, the flank
    lies at t > tT.  tT < 0: the touch is on the FLANK, the round lies at t < tT."""
    y1 = lambda t: R1 - math.sqrt(R1 * R1 - t * t)
    if tT >= 0:
        Cr = (0.0, -re); s = tT / re; c = math.sqrt(1 - s * s)
        PT = (Cr[0] + re * s, Cr[1] + re * c); n = ((Cr[0] - PT[0]) / re, (Cr[1] - PT[1]) / re)
        Cf = (PT[0] + R2 * n[0], PT[1] + R2 * n[1])
        def y2(t):
            if t <= tT: return Cr[1] + math.sqrt(re * re - (t - Cr[0]) ** 2)
            return Cf[1] + math.sqrt(R2 * R2 - (t - Cf[0]) ** 2)
    else:
        Cf = (0.0, -R2); PT = (tT, Cf[1] + math.sqrt(R2 * R2 - tT * tT)); n = ((Cf[0] - PT[0]) / R2, (Cf[1] - PT[1]) / R2)
        Cr = (PT[0] + re * n[0], PT[1] + re * n[1])
        def y2(t):
            if t >= tT: return Cf[1] + math.sqrt(R2 * R2 - (t - Cf[0]) ** 2)
            return Cr[1] + math.sqrt(max(re * re - (t - Cr[0]) ** 2, 0.0))
    return lambda t: y1(t) - y2(t)

def profile_c1(R1, re, R2, tT, turn):
    """As profile_round_flank, with the round ending after a normal turn `turn` (rad) from the flank
    tangency and the tip land beyond it (a straight line: its curvature 1/r_a is negligible)."""
    y1 = lambda t: R1 - math.sqrt(R1 * R1 - t * t)
    if tT >= 0:
        Cr = (0.0, -re); phT = math.asin(tT / re)
        PT = (re * math.sin(phT), Cr[1] + re * math.cos(phT)); n = (-math.sin(phT), -math.cos(phT))
        Cf = (PT[0] + R2 * n[0], PT[1] + R2 * n[1])
    else:
        Cf = (0.0, -R2); phT = math.asin(tT / R2)
        PT = (tT, Cf[1] + math.sqrt(R2 * R2 - tT * tT)); n = (-math.sin(phT), -math.cos(phT))
        Cr = (PT[0] + re * n[0], PT[1] + re * n[1])
    phE = phT - turn                         # the round's end (tip-land side), normal angle
    tE = Cr[0] + re * math.sin(phE); yE = Cr[1] + re * math.cos(phE); slope = -math.tan(phE)
    def y2(t):
        if t >= tT: return Cf[1] + math.sqrt(R2 * R2 - (t - Cf[0]) ** 2)
        if t >= tE: return Cr[1] + math.sqrt(re * re - (t - Cr[0]) ** 2)
        return yE + slope * (t - tE)
    return lambda t: y1(t) - y2(t)
