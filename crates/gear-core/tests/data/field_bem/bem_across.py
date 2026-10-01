"""The across strip's reference by a different method: contact-verify5's boundary-element solver
(bem2d.py: piecewise-constant pressure, midpoint collocation, the exact log kernel, Levinson), on the
piecewise-quadratic gap h(t) = int_0^t h'(s) ds of each strip, at M = 1000, 2000, 4000, 8000 panels over
[m - 1.25 c, m + 1.25 c] (m, c the prototype's, only to place the window; the BEM's active set is its own and
it refuses a contact that reaches the window's edge).  Each figure is extrapolated by Richardson from the
last three M with the order they show; the BEM's own uncertainty is the change of that extrapolation from the
first three M to the last three.  Figures: the peak pressure (three-panel parabola), and the load fraction on
t > hi and on t < lo (the strip's edge, its flank [lo, hi]; all of it where lo > hi).
Usage: python3 bem_across.py OUT.json  (run once; it is the record, not a check)."""
import sys, json, math, hashlib, multiprocessing as mp, resource
BEM = '/home/user/.cache/gearcalc-work/contact-verify5/bem2d.py'
sys.path.insert(0, '/home/user/.cache/gearcalc-work/contact-verify5')
sys.path.insert(0, '/home/user/.cache/gearcalc-work/contact-proto')
INF = math.inf
def fl(x): return float(x)
def gap_of(k0, steps):
    """h(t) with h(0) = h'(0) = 0 and h'' = k0 + the steps' dk: exact, piece by piece."""
    ks = sorted(set([0.0] + [x for a, b, _ in steps for x in (a, b) if math.isfinite(x)]))
    kap = lambda t: k0 + sum(dk for a, b, dk in steps if a < t < b)
    def h(t):
        # integrate from 0 to t across knots: h' and h accumulated exactly on each constant-curvature piece
        s = 1.0 if t >= 0 else -1.0
        pts = [0.0] + [k for k in (ks if s > 0 else ks[::-1]) if (0 < k < t if s > 0 else t < k < 0)] + [t]
        H, D = 0.0, 0.0
        for u, v in zip(pts[:-1], pts[1:]):
            k = kap(0.5 * (u + v)); d = v - u
            H += D * d + 0.5 * k * d * d; D += k * d
        return H
    return h
def run(case):
    resource.setrlimit(resource.RLIMIT_AS, (1800 << 20, 1800 << 20))
    from bem2d import solve
    from trace import contact2d
    name, q, Es, k0, steps, flank = case
    c, m, _, _ = contact2d(q, Es, k0, steps)
    h = gap_of(k0, steps)
    lo, hi = flank
    rows = []
    for M in (1000, 2000, 4000, 8000):
        xl, xr = m - 1.25 * c, m + 1.25 * c
        D = (xr - xl) / M
        seed = (max(int((m - c - xl) / D) + 1, 1), min(int((m + c - xl) / D) - 1, M - 2))
        s = solve(h, q, Es, xl, xr, M, seed=seed)
        tot = sum(s['p']) * D
        def beyond(t, right):          # load on t' > t (right) or t' < t, the panel holding t shared linearly
            out = 0.0
            for x, p in zip(s['x'], s['p']):
                a, b = x - 0.5 * D, x + 0.5 * D
                w = (max(0.0, b - max(a, t)) if right else max(0.0, min(b, t) - a))
                out += p * w
            return out
        if lo > hi: fe = tot                # no flank in this section: the whole strip is edge
        else: fe = (beyond(hi, True) if hi < INF else 0.0) + (beyond(lo, False) if lo > -INF else 0.0)
        rows.append(dict(M=M, pmax=s['pmax'], xpmax=s['xpmax'], a=s['a'], b=s['b'], load=tot, fedge=fe / tot))
    def rich(vals):
        v1, v2, v4 = vals; d1, d2 = v2 - v1, v4 - v2
        if d2 == 0.0: return v4, None
        r = d1 / d2
        if not r > 1.0: return v4, None
        order = math.log(r, 2); return v4 + d2 / (r - 1.0), order
    out = dict(name=name, q=q, Es=Es, k0=k0, steps=[[a if math.isfinite(a) else ('-inf' if a < 0 else 'inf'), b if math.isfinite(b) else ('-inf' if b < 0 else 'inf'), dk] for a, b, dk in steps],
               flank=[lo if math.isfinite(lo) else '-inf', hi if math.isfinite(hi) else 'inf'], rows=rows)
    for key in ('pmax', 'fedge'):
        e1, o1 = rich([r[key] for r in rows[:3]]); e2, o2 = rich([r[key] for r in rows[1:]])
        out[key] = dict(value=e2, order=o2, previous=e1, previous_order=o1, uncertainty=abs(e2 - e1))
    return out
if __name__ == '__main__':
    import os
    d = json.load(open('/home/user/.cache/gearcalc-work/wt/field/crates/gear-core/tests/data/field_oracle/across.json'))
    pick = ['across/strip0', 'across/strip1', 'across/strip7', 'across/strip11', 'across/strip15', 'across/strip36', 'across/strip39', 'across/strip42']
    cases = []
    for r in d['records']:
        if r['id'] in pick:
            i = r['inputs']; cases.append((r['id'], i['q'], i['Es'], i['k0'], [(fl(a), fl(b), dk) for a, b, dk in i['steps']], (fl(i['flank'][0]), fl(i['flank'][1]))))
    # a section concave away from its contact (the ring's valley, gap.json's ring pair 0 point 3): k0 < 0 beyond a round
    cases.append(('ring_concave_beyond', 100.0, 113186.81318681319, -0.048367718543815094, [(-0.06114719532307458, 0.11149908445304985, 4.994284347581456)], (-INF, -0.06114719532307458)))
    with mp.Pool(len(cases)) as pool:
        res = pool.map(run, cases)
    src = open(BEM, 'rb').read(); me = open(__file__, 'rb').read()
    json.dump(dict(what=__doc__, bem2d_md5=hashlib.md5(src).hexdigest(), script_md5=hashlib.md5(me).hexdigest(), strips=res), open(sys.argv[1], 'w'), indent=1)
    for r in res: print(r['name'], {k: (r[k]['value'], r[k]['order'], r[k]['uncertainty']) for k in ('pmax', 'fedge')})
