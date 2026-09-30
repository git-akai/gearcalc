"""make_oracle.py -- the differential oracle for the Rust port of the unified contact field (contact-proto-6).

ONE script writes every file of this directory: <module>.json (records: id, fn, inputs, outputs, tol) and index.json
(the Python sources' md5, the cases, the tolerance rules).  The Rust port reproduces each record's outputs from its
inputs within the record's tolerance.  See README.md.
Usage: python3 oracle/make_oracle.py [module ...] [--pool 4]   (modules: form kernel across compliance gap state phase
matched; default all).  Every worker caps its address space at 1.8 GB.

Version 2 (oracle-fix, 2026-09-30; version 1 is kept in ../oracle_v1/): the tooth compliance is a converged quantity
(field.ToothCompliance: Chebyshev in the roll parameter, order by a stated bound) and every state, phase and matched
record carries each member's compliance nodes as an INPUT (mesh.members[i].tooth, read by Member(tooth=...)); the
separated panel is cancellation-free (kernel.panel_sep) and G is tabulated to 2e-14; L* is closed form; carter_slide
is cancellation-free (with |B| << |A| cases in across/slide); panels are graded into punch ends (trace.end_weights)."""
import sys, os, json, math, time, random, hashlib, multiprocessing as mp
HERE = '/home/user/.cache/gearcalc-work/contact-proto/'
OUT = os.environ.get('ORACLE_OUT', HERE + 'oracle/')    # a test run may write elsewhere
sys.path.insert(0, HERE)
SOURCES = ['trace.py', 'valley.py', 'coupled.py', 'field.py', 'kernel.py', 'form.py', 'foundation.py', 'stiff.py', 'carlson.py',
           'phase.py', 'phase4.py', 'mw_tfield.py', 'mw_field.py', 'mw_surface.py', 'oracle.py', 'c6.py']
VERSION = 2
CHANGES = ['tooth compliance: Chebyshev in the involute roll parameter t, order doubled from 8 until <= 1e-12 at the next grid\'s new '
           'nodes (v1: order 20 in depth, 3.2e-3 off on the z 17 spur); its nodes and values are an input of every state, phase and matched record',
           'stiff.py: the involute integrated in t and the fillet\'s derivative in closed form (v1: in r, and a central difference): converged to 1e-15',
           'kernel.panel: separated panels by panel_sep (cancellation-free; v1 amplified the G table\'s 7.9e-11 to 1.9e-8); G table step 0.0025 '
           '(2e-14); G_direct 16-point cells; the width rule TOL 1e-14; panel records toleranced on the scale of their terms',
           'foundation L* = 18(1 - nu^2)/pi in closed form (v1: quadrature, 8.7e-8 off)',
           'carter_slide cancellation-free (v1: 1.7e-2 off per panel on spur lines, where |B|/|A| ~ 1e-13); across/slide adds |B| << |A| cases',
           'panels graded into punch ends (one body\'s face ends the line, the other runs on): trace.end_weights/end_map, continuous in every input',
           'across c tolerance absolute (brent stops at 1e-14 mm), slide tolerance 1e-13',
           'state tolerances F 1e-9 and loss 2e-8: 10x the iteration residual, which exceeded v1\'s 1e-10 and 1e-9 (loss 1.4x on the creep state)']
def cap():
    import resource; resource.setrlimit(resource.RLIMIT_AS, (1_800_000_000, 1_800_000_000))

# ---------------------------------------------------------------- tolerance rules (README 'Tolerances')
# closed form: floating point only -> 1e-13 relative (1e-13 absolute for unit vectors / values that pass 0)
# root found at tol r -> 10 r (relative to the quantity's own scale)
# the field's state: 10 x the solve's own floor, measured as the largest change under two exact symmetries of the
#   model (every length x10 and x0.1, and the nominal line load x2 and x1/4, which moves only the seed):
#   floors pmax 5.4e-9, pflank 2.8e-9, pedge 5.4e-9, Fedge 3.5e-9, D 3.0e-10, L 4.3e-10, F 3.2e-12, T 6e-13 (r6/homog_*.txt)
TOL_CLOSED = dict(rel=1e-13, basis='closed form: floating point')
STATE_TOL = {'D': 3e-9, 'pmax': 5e-8, 'pflank': 5e-8, 'pedge': 5e-8, 'Fedge': 5e-8, 'L': 5e-9, 'Lp': 5e-9, 'F': 1e-9,
             'T1': 1e-10, 'T2': 1e-10, 'loss': 2e-8, 'n': 0, 'q': 1e-6, 'ys': 1e-9, 'at_y': 2e-3}
# v2: F and loss are 10x the ITERATION residual, which exceeds their symmetry floor: stopping the width fixed point at
# dq 1e-10 (not 1e-7) and the span at 1e-13 (not 1e-10) moves F 6.6e-11 (s0/0.3183) and loss 1.4e-9 (s0/0.7495/carter);
# v1's 1e-10 and 1e-9 failed a port iterating tighter (contact-verify7's tol7, rerun on v2: oracle-fix/v7/tol7.txt)
STATE_BASIS = ('10 x the larger of the Python solve floor (homogeneity x10/x0.1 and nominal x2/x1/4 changes, r6/homog_*.txt) and its '
               'iteration residual (the change when the width and span fixed points stop 1000x tighter: F 6.6e-11, loss 1.4e-9); n and the end '
               'tags exact; station q relative to the line max (the b fixed point stops at dq < 1e-7); ys (station sections) '
               'relative to the span; at_y: the location of a maximum is ill-conditioned (sqrt of the value floor), '
               'relative to the line length')

# ---------------------------------------------------------------- mesh specifications (plain numbers: the Rust builds these)
def member_spec(z, beta, x, b, mn, form, E=206000.0, nu=0.3, rim=None, end=None):
    return dict(z=z, beta_deg=beta, x=x, b=b, mn=mn, an_deg=20.0, E=E, nu=nu, ha=1.0, hf=1.25, rho_f=0.38,
                form=dict(Ca=form[0], La=form[1], re=form[2]), rim=rim, end=end)
AP20 = None
def mesh_of(name):
    """(spec, builder kwargs) for each named mesh."""
    import oracle as O
    re01 = 0.1
    def ext(z1, z2, beta, S, b=(10.0, 10.0), x=(0.0, 0.0), mn=1.0, Ca=0.0, La=0.4, re=0.1, mu=0.06, nu=0.3):
        bw2 = (S - beta)
        a = O.a_par(O.Gear(z1, beta, x[0], b[0], mn), O.Gear(z2, bw2 if S == 0 else -beta, x[1], b[1], mn))
        return dict(members=[member_spec(z1, beta, x[0], b[0], mn, (Ca, La * mn, re), nu=nu), member_spec(z2, bw2, x[1], b[1], mn, (Ca, La * mn, re), nu=nu)],
                    Sigma_deg=S, a=a, mu=mu)
    if name == 'h20': return ext(17, 43, 20.0, 0.0)
    if name == 'h20r': return ext(17, 43, 20.0, 0.0, Ca=0.005)
    if name == 's0': return ext(17, 43, 0.0, 0.0)
    if name == 's0bb': return ext(17, 43, 0.0, 0.0, b=(10.0, 10.5))
    if name == 'c1': return ext(17, 43, 20.0, 1.0)
    if name == 'c10': return ext(17, 43, 20.0, 10.0)
    if name == 'ring':       # ISO 6336-2 example: ring helical 17/-43 b15 m2 x.3/-.3 b20, mu 0, r_e 0.1 m_n
        z1, z2, beta, mn, b = 17, -43, 15.0, 2.0, 20.0
        a = O.a_par(O.Gear(z1, beta, 0.3, b, mn), O.Gear(z2, beta, -0.3, b, mn))
        return dict(members=[member_spec(z1, beta, 0.3, b, mn, (0.0, 0.4 * mn, 0.2)), member_spec(z2, beta, -0.3, b, mn, (0.0, 0.4 * mn, 0.2))], Sigma_deg=0.0, a=a, mu=0.0)
    if name == 'worm':       # the shipped worm 1/40 ZI, d1 7, m 1, steel on C360, the involute wheel
        b1 = math.degrees(math.acos(1 / 7)); b2 = 90 - b1
        w = member_spec(1, b1, 0.0, 13.539, 1.0, (0.0, 0.4, 0.1), E=190000.0, nu=0.29)
        g = member_spec(40, b2, 0.0, 4.690, 1.0, (0.0, 0.4, 0.1), E=96500.0, nu=0.32)
        from field import Member
        a = abs(Member(1, b1, 0.0, 13.539, 1.0).r) + abs(Member(40, b2, 0.0, 4.690, 1.0).r)
        return dict(members=[w, g], Sigma_deg=90.0, a=a, mu=0.06)
    raise KeyError(name)
def scaled(spec, k):
    s = json.loads(json.dumps(spec))
    for m in s['members']:
        for key in ('b', 'mn'): m[key] *= k
        m['form']['La'] *= k; m['form']['Ca'] *= k
        if m['form']['re']: m['form']['re'] *= k
        if m['rim'] is not None: m['rim'] *= k
    s['a'] *= k
    return s
def build(spec, N=24, friction='panel', nsec=None, klass='TField'):
    """The field of a mesh spec.  A member carrying 'tooth' (a ToothCompliance record plus found_H) is built on
    that compliance (ToothCompliance.from_nodes), bit for bit what the record was computed with."""
    from field import Member
    from form import Form
    import trace
    ms = []
    for m in spec['members']:
        M = Member(m['z'], m['beta_deg'], m['x'], m['b'], m['mn'], m['an_deg'], E=m['E'], nu=m['nu'], ha=m['ha'], hf=m['hf'], rho_f=m['rho_f'],
                   tooth=m.get('tooth'))
        if m['rim'] is not None: M.rim = m['rim']
        ms.append(M)
    forms = tuple(Form(M, m['form']['Ca'], m['form']['La'], m['form']['re']) for M, m in zip(ms, spec['members']))
    if klass == 'MWTField':
        from mw_tfield import MWTField
        F = MWTField(ms[0], ms[1], spec['Sigma_deg'], spec['a'], kind='ZI', mu=spec['mu'], forms=forms)
    else:
        F = trace.TField(ms[0], ms[1], spec['Sigma_deg'], spec['a'], mu=spec['mu'], N=N, forms=forms, friction=friction)
    for M, m in zip(ms, spec['members']):
        if m['end']: M.end = tuple(m['end'])
    if nsec: F.NSEC = nsec
    return F

def with_teeth(spec, klass='TField'):
    """(spec with each member's tooth compliance as an input, the field built on it).  The compliance is generated
    once (Member + derive_foundation), recorded, and the field the record is computed on is built FROM the record,
    so its inputs reproduce its outputs through the same path a port takes (ToothCompliance.from_nodes)."""
    F0 = build(spec, klass=klass)
    s = json.loads(json.dumps(spec))
    for m, M in zip(s['members'], F0.m):
        m['tooth'] = dict(M.tooth.record(), found_H=M.found_H)
    return s

# ---------------------------------------------------------------- records
def rec(id_, fn, inputs, outputs, tol):
    return dict(id=id_, fn=fn, inputs=inputs, outputs=outputs, tol=tol)
def f(x):
    if isinstance(x, float) and not math.isfinite(x): return str(x)
    return x

def mod_form():
    from field import Member
    from form import Form
    out = []
    for mname, (z, beta, x, mn) in (('p17b20', (17, 20.0, 0.0, 1.0)), ('w43b20', (43, -20.0, 0.0, 1.0)), ('s17', (17, 0.0, 0.0, 1.0)),
                                    ('ring43b15', (-43, 15.0, -0.3, 2.0)), ('worm', (1, math.degrees(math.acos(1 / 7)), 0.0, 1.0))):
        M = Member(z, beta, x, 10.0, mn)
        for Ca, La, re in ((0.0, 0.0, 0.1 * mn), (0.005 * mn, 0.4 * mn, 0.1 * mn), (0.005 * mn, 0.4 * mn, None), (0.0, 0.0, 0.45 * mn)):
            Fm = Form(M, Ca, La, re)
            lo = (Fm.sig_t - 2 * Fm.w_end) if re else -0.3 * mn
            hi = max(La / Fm.sth if Ca else 0.0, 0.3 * mn) * 1.2
            sig = [lo + (hi - lo) * i / 24 for i in range(25)]
            vals = [list(Fm(s)) for s in sig]
            out.append(rec(f'form/{mname}/Ca{Ca}/La{La}/re{re}', 'form.Form.__call__ (F, dF/dsigma, kappa) and on_edge',
                           dict(member=dict(z=z, beta_deg=beta, x=x, mn=mn, an_deg=20.0, ha=1.0, hf=1.25), Ca=Ca, La=La, re=re, sigma=sig),
                           dict(sth=Fm.sth, cth=Fm.cth, sig_t=getattr(Fm, 'sig_t', None), w_end=getattr(Fm, 'w_end', None), F_end=getattr(Fm, 'F_end', None),
                                ra=M.ra, rb=M.rb, tw=M.tw, values=vals, on_edge=[Fm.on_edge(s) for s in sig]), TOL_CLOSED))
    return out

def mod_kernel():
    import kernel, carlson
    out = []
    rs = [10 ** (k / 2) for k in range(-12, 13)]
    out.append(rec('kernel/G', 'kernel.G (Hermite table in ln r) and kernel.G_direct (quadrature), kernel.dG (closed form via R_D)',
                   dict(r=rs), dict(G=[kernel.G(r) for r in rs], G_direct=[kernel.G_direct(r) for r in rs], dG=[kernel.dG(r) for r in rs]),
                   dict(G=dict(abs=1e-12, basis='G_direct (16-point cells) equals tanh-sinh quadrature of the definition to 2e-14; the table (step 0.0025 in ln r) equals it to 4e-14'),
                        G_direct=dict(abs=1e-12, basis='as G'), dG=TOL_CLOSED)))
    rng = random.Random(11); cases = []
    for _ in range(24):
        b = 10 ** rng.uniform(-3, -0.5); h = 10 ** rng.uniform(-1, 0.3); e1 = rng.uniform(-3, 1); e2 = e1 + 10 ** rng.uniform(-2, 0.5)
        cases.append((e1, e2, b, h, 0.3, 206000.0))
    cases.append((-0.1, 0.1, 0.05, float('inf'), 0.3, 206000.0))
    # v2: the separated form's hard cases -- far panels (surface/total ~ -7e3), h < b, the kernel's zero near r ~ h,
    # both sides of RSEP, a panel ending at the point, an image-like far panel
    b0 = 0.05
    cases += [(-153.2 * b0, -152.9 * b0, b0, 3.94 * b0, 0.3, 206000.0), (21.7 * b0, 35.9 * b0, b0, 0.702 * b0, 0.3, 206000.0),
              (-693 * b0, -691 * b0, b0, 28.4 * b0, 0.29, 190000.0), (-4.06 * b0, -4.01 * b0, b0, 0.919 * b0, 0.3, 206000.0),
              (4.69 * b0, 5.81 * b0, b0, 4.03 * b0, 0.3, 206000.0),
              (0.5 * b0, 0.9 * b0, b0, 5.0 * b0, 0.3, 206000.0), (0.4999 * b0, 0.9 * b0, b0, 5.0 * b0, 0.3, 206000.0),
              (0.0, 2.0 * b0, b0, 3.0 * b0, 0.32, 96500.0), (-1.2 * b0, 1e-3 * b0, b0, 8.0 * b0, 0.3, 206000.0),
              (-2000 * b0, -1980 * b0, b0, 30.0 * b0, 0.3, 206000.0)]
    def parts(e1, e2, b, h, nu, E):
        """The larger of the panel's two terms (the tolerance's scale): each term is known to ~1e-15 of itself."""
        if not math.isfinite(h): return abs((1 - nu * nu) / (math.pi * E) * (kernel.G_direct(e2 / b) - kernel.G_direct(e1 / b))), 'G'
        if e1 * e2 > 0 and min(abs(e1), abs(e2)) >= kernel.RSEP * b:
            r = min(abs(e1), abs(e2)) / b; Eb = r + math.sqrt(1 + r * r)
            n = max(2, math.ceil(math.log(1 / kernel.TOL_SEP) / (2 * math.log(Eb))) + 1)
            D = (e2 - e1) * (e2 + e1); h2 = h * h; ta = tb = 0.0
            for t, w in kernel._gc(n):
                x2 = (b * t) ** 2
                s1, s2 = math.sqrt(x2 + e1 * e1), math.sqrt(x2 + e2 * e2); R1, R2 = math.sqrt(x2 + h2 + e1 * e1), math.sqrt(x2 + h2 + e2 * e2)
                ds, dR = e2 * s1 + e1 * s2, e2 * R1 + e1 * R2; a, c = D / ds, D / dR
                amc = D * h2 * (e2 / (R1 + s1) + e1 / (R2 + s2)) / (ds * dR)
                ta += w * abs(2 * (1 - nu) * math.asinh(amc * (a + c) / (a * math.sqrt(1 + c * c) + c * math.sqrt(1 + a * a))))
                tb += w * abs(h2 * D / (dR * R1 * R2))
            return (1 + nu) / (2 * math.pi * E) * max(ta, tb), 'sep'
        sur = abs((1 - nu * nu) / (math.pi * E) * (kernel.G_direct(e2 / b) - kernel.G_direct(e1 / b)))
        dep = abs((1 + nu) / (2 * math.pi * E) * kernel.depth_panel(e1, e2, b, h, nu))
        return max(sur, dep), 'G'
    sc = [parts(*c) for c in cases]
    out.append(rec('kernel/panel', 'kernel.panel(eta1, eta2, b, h, nu, E): depth-referenced lengthwise influence of a Hertz-strip panel (separated panels: kernel.panel_sep, cancellation-free)',
                   dict(cases=[list(map(f, c)) for c in cases]),
                   dict(panel=[kernel.panel(*c) for c in cases], scale=[v[0] for v in sc], route=[v[1] for v in sc],
                        n_width=[kernel.n_width(c[2], c[3]) if math.isfinite(c[3]) else None for c in cases]),
                   dict(panel=dict(abs_of_scale=1e-12, basis='|port - record| <= 1e-12 x scale (the larger of the panel\'s two terms, each known to ~1e-15 of itself): '
                                   'relative 3e-12 far out (the -nu cancellation), absolute at the kernel\'s zero near r ~ h. A G-form port fails the separated cases '
                                   '(surface/total up to 3e7): separated panels need a cancellation-free form'),
                        n_width=dict(abs=0, basis='integer'))))
    lcases = [(0.01, 0.8, 0.3, 206000.0), (0.1, 0.5, 0.29, 190000.0), (0.004, 1.2, 0.32, 96500.0)]
    out.append(rec('kernel/line_limit', 'kernel.line_limit(b, h, nu, E)', dict(cases=lcases), dict(v=[kernel.line_limit(*c) for c in lcases]), TOL_CLOSED))
    qs = [0.0, 1e-8, 1e-4, 0.01, 0.05, 0.1, 0.2, 0.3, 0.5, 0.7, 0.9, 1.0]
    out.append(rec('kernel/shape_C', 'kernel.shape_C(q) and carlson.aspect(q)', dict(q=qs),
                   dict(C=[kernel.shape_C(q) for q in qs], aspect=[carlson.aspect(q) for q in qs if q > 0]),
                   dict(C=dict(rel=1e-12, basis='R_D to 1e-15, aspect bisection to 1e-15 in ln s'), aspect=dict(rel=1e-12, basis='bisection tol 1e-15'))))
    rds = [(0.0, 0.5, 1.0), (0.0, 1.0, 0.01), (0.2, 0.3, 0.7), (1e-6, 1.0, 1.0)]
    out.append(rec('kernel/carlson', 'carlson.rf, carlson.rd (as elliptic.rs)', dict(args=rds),
                   dict(rf=[carlson.rf(*a) for a in rds], rd=[carlson.rd(*a) for a in rds]), dict(rel=1e-13, basis='duplication to 1e-4 mu + 5th-order series: 1e-15')))
    return out

def mod_across():
    import trace
    from trace import contact2d, strip_report, panel_slide, carter_slide, _strip, INF
    Es = 113186.81318681319
    out = []
    secs = [(300.0, 0.4, [], (-INF, INF)), (60.0, 0.8, [], (-0.01, INF))]
    for seed, both in ((6, False), (7, True)):
        rng = random.Random(seed)
        for _ in range(20):
            k0 = rng.uniform(0.1, 1.5); q = math.exp(rng.uniform(math.log(20), math.log(800)))
            re = rng.choice((0.05, 0.1, 0.2, 0.45)); c2 = rng.uniform(0.3, 1.0); w = re * rng.uniform(0.5, 1.2)
            cH = math.sqrt(4 * q / (math.pi * Es * k0)); s = rng.uniform(-1.5 * cH, 1.5 * cH)
            steps = [(s, s + w, c2 / re)]; flank = (-INF, s)
            if both or rng.random() < 0.4:
                re2 = rng.choice((0.05, 0.1, 0.2)); w2 = re2 * rng.uniform(0.5, 1.2); s2 = rng.uniform(-1.5 * cH, s)
                steps.append((s2 - w2, s2, rng.uniform(0.3, 1.0) / re2)); flank = (s2, s)
            if both or rng.random() < 0.3:
                L = rng.uniform(-cH, cH); steps.append((-INF, L, rng.uniform(0.05, 1.0)))
            secs.append((q, k0, steps, flank))
    secs.append((200.0, 0.5, [(-0.1032625722527446, -0.1032625722527446 + 0.06, 10.0), (-0.09, -0.04, 8.0)], (-0.04, -0.1032625722527446)))
    for i, (q, k0, steps, flank) in enumerate(secs):
        c, m, p, tp = contact2d(q, Es, k0, steps)
        r = strip_report(q, Es, k0, steps, flank)
        out.append(rec(f'across/strip{i}', 'trace.contact2d and trace.strip_report (frictionless 2-D contact of a piecewise-quadratic section; region maxima per piece)',
                       dict(q=q, Es=Es, k0=k0, steps=[[f(a), f(b), dk] for a, b, dk in steps], flank=[f(flank[0]), f(flank[1])]),
                       dict(c=c, m=m, pmax2d=p, t_pmax2d=tp, **{k: r[k] for k in ('pmax', 't_pmax', 'pflank', 'pedge', 'fedge')}),
                       dict(c=dict(abs=1e-13, basis='brent stops at 1e-14 (1 + |a| + |b|) on c, in mm: 10x absolute (v1 said rel 1e-12, tighter than its own stop at c 3.7 um)'),
                            m=dict(abs=1e-12, basis='brent'),
                            pmax=dict(rel=1e-10, basis='golden section per piece to 1e-14 in theta'), pflank=dict(rel=1e-10, basis='as pmax'),
                            pedge=dict(rel=1e-10, basis='as pmax'), fedge=dict(abs=1e-10, basis='Gauss-Legendre 12 per piece between steps (same rule)'),
                            t_pmax=dict(rel=1e-4, basis='location of a smooth maximum: sqrt of the value tolerance'))))
    rng = random.Random(3); sl = []
    for i in range(30):
        A = tuple(rng.uniform(-1, 1) for _ in range(3)); B = tuple(rng.uniform(-2, 2) for _ in range(3))
        if rng.random() < 0.5: B = tuple(A[k] * rng.uniform(-3, 3) for k in range(3))
        ss = rng.uniform(0.05, 2.0)
        if i >= 20:     # v2: a spur line's panel, |B| ~ 1e-16 ... 1e-12 |A| (parallel or not), inside and outside the creep window
            na = math.sqrt(sum(a * a for a in A)); e = 10 ** rng.uniform(-16, -12)
            B = tuple(A[k] * e * rng.choice((1, -1)) + (1e-17 * rng.uniform(-1, 1) if i % 2 else 0.0) for k in range(3))
            ss = na * 10 ** rng.uniform(-0.5, 0.5)
        E1, S1 = panel_slide(A, B); E2, S2 = carter_slide(A, B, ss)
        sl.append(dict(A=A, B=B, sstar=ss, coulomb_dir=E1, coulomb_speed=S1, carter_dir=E2, carter_speed=S2))
    out.append(rec('across/slide', 'trace.panel_slide (Coulomb mean over a panel) and trace.carter_slide (Carter creep), closed forms',
                   dict(cases=[dict(A=s['A'], B=s['B'], sstar=s['sstar']) for s in sl]),
                   dict(cases=[{k: s[k] for k in ('coulomb_dir', 'coulomb_speed', 'carter_dir', 'carter_speed')} for s in sl]),
                   dict(abs=1e-13, basis='closed forms without cancellation; = Gauss-Legendre 64 x 8 per piece (split at the window and the zero of sliding) '
                        'to 1.9e-14 on 3000 panels incl. |B| 1e-16 ... 1e-12 |A| (oracle-fix probe8; v1 carter_slide erred 8.9e-2 there)')))
    return out

def mod_compliance():
    from field import Member
    from coupled import derive_foundation
    import foundation, stiff
    out = []
    for name, (z, beta, x, mn, b) in (('p17b20', (17, 20.0, 0.0, 1.0, 10.0)), ('w43b20', (43, -20.0, 0.0, 1.0, 10.0)), ('s17', (17, 0.0, 0.0, 1.0, 10.0)),
                                      ('s43', (43, 0.0, 0.0, 1.0, 10.0)), ('ring43b15', (-43, 15.0, -0.3, 2.0, 20.0)), ('p17b15x3', (17, 15.0, 0.3, 2.0, 20.0))):
        M = Member(z, beta, x, b, mn)
        fr = [i / 8 for i in range(8)] + [0.95, 0.99, 0.999, 1.0]       # v2: down to the form circle, where v1's interpolant failed
        sai = [M.ct(M.lo + (M.hi - M.lo) * v) for v in fr]
        derive_foundation(M)
        rho = [M.lo + (M.hi - M.lo) * v for v in fr]
        T = M.tooth
        out.append(rec(f'compliance/{name}', 'field.Member + coupled.derive_foundation: stiff.py\'s generated tooth at unit module (the involute integrated in its roll '
                       'parameter, the fillet in the rack roll angle), the half-plane foundation at the rim depth s_R = 1.2 h_t ext / 3.5 m_n ring, '
                       'read through field.ToothCompliance (Chebyshev in the roll parameter, order by the stated bound)'
                       + ('; A RING IS THE RACK TOOTH (stiff.Gear z 4000, ISO 6336-1\'s z_n2 = inf), not its own shaper-cut tooth' if z < 0 else ''),
                       dict(z=z, beta_deg=beta, x=x, mn=mn, b=b, an_deg=20.0, E=206000.0, nu=0.3, ha=1.0, hf=1.25, rho_f=0.38, rho=rho),
                       dict(lo=M.lo, hi=M.hi, ra=M.ra, rf=M.rf, rb=M.rb, found_H=M.found_H, form_depth=T.form_depth, order=T.n, interp_err=T.err,
                            ct=[M.ct(r) for r in rho], hd=[M.hd(r) for r in rho], ct_sainsot=sai),
                       dict(ct=dict(rel=1e-11, basis='the tooth is converged: its quadrature to 1e-15 (Gauss-Legendre 48 vs 96), its interpolant to <= 1e-12 '
                                    '(ToothCompliance.TOL, checked at the next grid and on 801 points); a port of the same tooth to its own converged accuracy matches'),
                            hd=dict(rel=1e-11, basis='as ct'), ct_sainsot=dict(rel=1e-11, basis='as ct (Sainsot\'s fitted foundation)'),
                            lo=dict(rel=1e-12, basis='the form radius: stiff._transition bisection to the last bit'),
                            order=dict(abs=0, basis='informative: a port may interpolate at any order meeting the bound'))))
    cf = []
    for S, H, nu in ((1.8, 2.7, 0.3), (2.2, 2.7, 0.3), (1.8, 7.8, 0.3), (2.0, 7.0, 0.29), (3.0, 3.5, 0.32)):
        L, Mm, P, Q = foundation.coefficients(S, H, nu); cf.append(dict(S=S, H=H, nu=nu, L=L, M=Mm, P=P, Q=Q))
    out.append(rec('compliance/foundation', 'foundation.coefficients(S, H, nu): L*, M* closed form; P*, Q* depth-referenced (Boussinesq/Cerruti along the section)',
                   dict(cases=[{k: c[k] for k in ('S', 'H', 'nu')} for c in cf]), dict(cases=[{k: c[k] for k in ('L', 'M', 'P', 'Q')} for c in cf]),
                   dict(L=TOL_CLOSED, M=TOL_CLOSED, P=dict(rel=1e-12, basis='24-point Gauss x 6-16 panels; doubling the panels changes nothing'), Q=dict(rel=1e-12, basis='as P'),
                        basis='L* = 18(1 - nu^2)/pi and M* = 2(1 - 2nu)(1 + nu) closed form (v1 took L* by quadrature, 8.7e-8 off)')))
    return out

def gap_task(name):
    cap()
    import trace
    from phase import set_nominal
    spec = mesh_of(name); F = build(spec)
    T = dict(h20=2000.0, s0=20e3, c10=2000.0, ring=60e3, worm=2000.0)[name]
    set_nominal(F, T)
    ph = dict(h20=0.3183, s0=0.62, c10=0.4183, ring=0.3183, worm=0.7216)[name]
    t = ph * F.m[0].p
    Ps = F.pairs_at(t)
    pairs = []
    for P in Ps:
        yl, yh, tl, th = P['seg']
        ys = [yl + (yh - yl) * i / 6 for i in range(7)]
        vals = []
        for y in ys:
            X, g, eA = F.valley(t, P, y)
            sec = F.section(t, P, X, eA)
            a = F.part(0, t, P['j'], X, True); b = F.part(1, t, P['k'], X, True)
            vals.append(dict(y=y, X=X, g=g, eA=eA, d1=F.d1(t, P['j'], X), d2=F.d2(t, P['k'], X), sig=(a[2], b[2]), rho_f=(a[5], b[5]),
                             k0=sec['k0'], kz=sec['kz'], kl=sec['kl'], C=sec['C'], steps=[[f(u), f(v), w] for u, v, w in sec['steps']]))
        pairs.append(dict(j=P['j'], k=P['k'], anchor=dict(A=P['A'], kind=P['kind']), seg=[yl, yh, tl, th], y0=P['y0'], g0=P['g0'],
                          faces=[[f(v) for v in r] for r in P['faces']], nodes=dict(TS=P['TS'], TG=P['TG']), valley=vals))
    geo = [dict(ra=m.ra, rf=m.rf, rb=m.rb, lo=m.lo, hi=m.hi, r=m.r, bb=m.bb, k=m.k, e=m.e, sth=m.form.sth, cth=m.form.cth, sig_t=getattr(m.form, 'sig_t', None)) for m in F.m]
    return rec(f'gap/{name}', 'field.Field geometry (d1, d2, grad), valley.VField.part, trace.TField.trace (whole valley in the field, NBEYOND 2), smin, valley, section, face_crossings',
               dict(mesh=spec, T=T, phase=ph, t=t, nominal=F.nominal),
               dict(dsec=F.dsec, dz=F.dz, k0=F.k0, side1=F.side1, comp=F.comp, members=geo, pairs=pairs),
               dict(closed=dict(rel=1e-13, basis='d1, d2, part: closed form'), valley=dict(abs=1e-9, basis='x m_n: the section minimum is a fixed point stopped at 1e-10 m_n'),
                    seg=dict(abs=1e-9, basis='x m_n: clip ends are roots (tol 1e-14) of the margin on valley points (1e-10 m_n)'),
                    tags=dict(abs=0, basis='exact'), nodes=dict(abs=1e-9, basis='x m_n, as valley'), curvature=dict(rel=1e-9, basis='closed form at a valley point known to 1e-10 m_n')))

STATE_CASES = [  # (id, mesh, T N mm, on, phase fraction, friction, N, scale)
    ('h20/0.1183', 'h20', 2000.0, 1, 0.1183, 'panel', 24, 1.0), ('h20/0.3183', 'h20', 2000.0, 1, 0.3183, 'panel', 24, 1.0),
    ('h20/0.5183', 'h20', 2000.0, 1, 0.5183, 'panel', 24, 1.0), ('h20/0.3183/N48', 'h20', 2000.0, 1, 0.3183, 'panel', 48, 1.0),
    ('h20T20/0.3183', 'h20', 20e3, 1, 0.3183, 'panel', 24, 1.0), ('h20T20/0.62', 'h20', 20e3, 1, 0.62, 'panel', 24, 1.0),
    ('h20T20/0.3183/x10', 'h20', 20e3 * 1000, 1, 0.3183, 'panel', 24, 10.0), ('h20T20/0.3183/x0.1', 'h20', 20e3 / 1000, 1, 0.3183, 'panel', 24, 0.1),
    ('h20r/0.3183', 'h20r', 20e3, 1, 0.3183, 'panel', 24, 1.0),
    ('s0/0.3183', 's0', 20e3, 1, 0.3183, 'panel', 24, 1.0), ('s0/0.62', 's0', 20e3, 1, 0.62, 'panel', 24, 1.0),
    ('s0/0.74975', 's0', 20e3, 1, 0.74975, 'panel', 24, 1.0), ('s0/0.75025', 's0', 20e3, 1, 0.75025, 'panel', 24, 1.0),
    ('s0/0.7495/carter', 's0', 20e3, 1, 0.7495, 'carter', 24, 1.0), ('s0/0.75/carter', 's0', 20e3, 1, 0.75, 'carter', 24, 1.0),
    ('s0bb/0.3183', 's0bb', 20e3, 1, 0.3183, 'panel', 24, 1.0),
    ('c1/0.5183', 'c1', 2000.0, 1, 0.5183, 'panel', 24, 1.0), ('c10/0.4183', 'c10', 2000.0, 1, 0.4183, 'panel', 24, 1.0),
    ('c10/0.1183', 'c10', 2000.0, 1, 0.1183, 'panel', 24, 1.0),
    ('ring/0.3183', 'ring', 60e3, 1, 0.3183, 'panel', 24, 1.0),
    ('worm/0.7216', 'worm', 2000.0, 1, 0.7216, 'panel', 24, 1.0), ('worm/0.7216/wheel', 'worm', 54953.0, 2, 0.7216, 'panel', 24, 1.0),
]
def state_task(case):
    cap()
    id_, mesh, T, on, ph, fr, N, k = case
    from phase import set_nominal
    spec = mesh_of(mesh)
    if k != 1.0: spec = scaled(spec, k)
    spec = with_teeth(spec)
    F = build(spec, N=N, friction=fr); set_nominal(F, T, on)
    t = ph * F.m[0].p
    t0 = time.time(); st = F.state(t, T, on)
    lines = []
    for P in F.last:
        s = P['st']
        lines.append(dict(j=P['j'], ends=list(s['ends']), lam=F.end_weights(t, P, s['ends'][0], s['ends'][1]), ye=s['ye'], ys=s['ys'], lp=s['lp'],
                          q=s['q'], g=s['g'], ct=s['ct']))
    at = list(st.get('at') or [])
    return rec(f'state/{id_}', 'trace.TField.state (solve: seed, span fixed point, coupled lines; the report along each line)',
               dict(mesh=spec, T=T, on=on, phase=ph, t=t, friction=fr, N=N, NSEC=F.NSEC, grade='kz', nominal=F.nominal, J=F.J),
               dict(**{q: st[q] for q in ('D', 'n', 'L', 'Lp', 'T1', 'T2', 'loss', 'F', 'Fedge', 'pmax', 'pflank', 'pedge')},
                    sig=[list(x) for x in st['sig']], at=at, lines=lines, span_iterations=len(getattr(F, 'span_hist', [])), secs=round(time.time() - t0, 1),
                    stats={q: F.stats.get(q) for q in ('cut', 'lost', 'nonconv', 'span_nonconv', 'touch', 'endload', 'uneg')}),
               dict(**{q: dict(rel=STATE_TOL[q]) for q in STATE_TOL}, basis=STATE_BASIS))

def phase_task(name):
    cap()
    from phase import set_nominal, breakpoints
    from phase4 import analyse4
    spec = with_teeth(mesh_of(name)); T = dict(s0=20e3, h20=2000.0)[name]
    F = build(spec); set_nominal(F, T)
    t0 = time.time(); r = analyse4(F, T)
    out = {k: r[k] for k in ('n', 'L', 'T1', 'T2', 'loss', 'F', 'Fedge', 'pmax', 't_pmax', 'pflank', 't_pflank', 'pedge', 't_pedge')}
    out['bps'] = r['bps']; out['at_pmax'] = list(r['at_pmax'] or []); out['states'] = r['states']; out['secs'] = round(time.time() - t0)
    return rec(f'phase/{name}', 'phase.breakpoints (signature scan N 24, bisection to 1e-10 p) + phase4.analyse4 (Gauss 6 per piece; maxima: piece ends as one-sided limits, golden 30 on the best node)',
               dict(mesh=spec, T=T, on=1, N=24), out,
               dict(bps=dict(abs=2e-10, basis='x pitch: bisection to 1e-10 p'), means=dict(rel=1e-7, basis='Gauss 6 per piece of states known to the state tolerance'),
                    maxima=dict(rel=1e-7, basis='golden 30 rounds: the phase to 1e-7 h, the value to the state tolerance'), t_max=dict(abs=1e-6, basis='x pitch')))

def matched_task(_):
    cap()
    from phase import set_nominal
    spec = with_teeth(mesh_of('worm'), klass='MWTField'); F = build(spec, klass='MWTField'); set_nominal(F, 2000.0, 1)
    t = 0.7216 * F.m[0].p
    st = F.state(t, 2000.0, 1)
    return rec('matched/worm/0.7216', 'mw_tfield.MWTField.state (the hobbed ZI wheel: foot5 projection, hob tip as a margin)',
               dict(mesh=spec, wheel='matched ZI (hob = the worm generator, addendum 1.25 m, cut at the running centre distance)', T=2000.0, on=1, phase=0.7216, t=t, nominal=F.nominal),
               dict(**{q: st[q] for q in ('D', 'n', 'L', 'F', 'Fedge', 'pmax', 'pflank', 'pedge', 'T2')}, sig=[list(x) for x in st['sig']],
                    stats={q: F.stats.get(q) for q in ('cut', 'offwheel', 'nonconv')}),
               dict(**{q: dict(rel=STATE_TOL.get(q, 1e-8)) for q in ('D', 'L', 'F', 'Fedge', 'pmax', 'pflank', 'pedge', 'T2')}, basis=STATE_BASIS + '; projection converged at 1e-12 m_n'))

def run_task(t):
    kind, arg = t
    try:
        return kind, {'gap': gap_task, 'state': state_task, 'phase': phase_task, 'matched': matched_task}[kind](arg)
    except Exception as e:
        import traceback
        return kind, dict(id=f'{kind}/{arg}', error=repr(e)[:300], trace=traceback.format_exc()[-800:])

if __name__ == '__main__':
    args = [a for a in sys.argv[1:] if not a.startswith('--')]
    pool = int(sys.argv[sys.argv.index('--pool') + 1]) if '--pool' in sys.argv else 4
    if '--pool' in sys.argv: args = [a for a in args if a != str(pool)]
    mods = args or ['form', 'kernel', 'across', 'compliance', 'gap', 'state', 'phase', 'matched']
    cap(); t0 = time.time()
    results = {}
    for m in ('form', 'kernel', 'across', 'compliance'):
        if m in mods: results[m] = globals()['mod_' + m]()
    tasks = []
    if 'phase' in mods: tasks += [('phase', n) for n in ('h20', 's0')]
    if 'matched' in mods: tasks.append(('matched', None))
    if 'gap' in mods: tasks += [('gap', n) for n in ('h20', 's0', 'c10', 'ring', 'worm')]
    if 'state' in mods: tasks += [('state', c) for c in STATE_CASES]
    if tasks:
        with mp.Pool(pool) as P:
            for kind, r in P.imap_unordered(run_task, tasks):
                results.setdefault(kind, []).append(r); print(kind, r.get('id'), 'ERROR ' + r['error'] if 'error' in r else 'ok', flush=True)
    for m, recs in results.items():
        recs.sort(key=lambda r: r['id'])
        json.dump(dict(module=m, records=recs), open(OUT + f'{m}.json', 'w'), indent=1, default=lambda o: str(o))
    idx = dict(version=VERSION, changes=CHANGES, previous='../oracle_v1/ (version 1, 2026-09-29)',
               generated=time.strftime('%Y-%m-%d %H:%M'), seconds=round(time.time() - t0),
               sources={s: hashlib.md5(open(HERE + s, 'rb').read()).hexdigest() for s in SOURCES},
               modules={m: [r['id'] for r in recs] for m, recs in results.items()},
               errors=[r for recs in results.values() for r in recs if 'error' in r],
               tolerance_rules=dict(closed=TOL_CLOSED, state=STATE_TOL, state_basis=STATE_BASIS))
    old = {}
    if os.path.exists(OUT + 'index.json') and args:
        old = json.load(open(OUT + 'index.json')); old['modules'].update(idx['modules']); old['sources'] = idx['sources']
        old['errors'] = [e for e in old.get('errors', []) if e['id'].split('/')[0] not in results] + idx['errors']
        idx = dict(old, generated=idx['generated'], seconds=old.get('seconds', 0) + idx['seconds'], version=VERSION, changes=CHANGES, previous=idx['previous'],
                   tolerance_rules=idx['tolerance_rules'])
    json.dump(idx, open(OUT + 'index.json', 'w'), indent=1)
    print('done', round(time.time() - t0), 's', 'errors', len(idx['errors']))
