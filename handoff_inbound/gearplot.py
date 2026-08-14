import numpy as np, matplotlib
matplotlib.use("Agg")
import matplotlib.pyplot as plt
from gearbuild import evaluate

cases = [
 dict(Module=1, PressureAngle=20, Teeth=17, ProfileShift=0.2, HelixAngle=0,  Addendum=1, Dedendum=1.25),
 dict(Module=1, PressureAngle=20, Teeth=8,  ProfileShift=0.0, HelixAngle=0,  Addendum=1, Dedendum=1.25),
 dict(Module=1, PressureAngle=20, Teeth=3,  ProfileShift=0.5, HelixAngle=0,  Addendum=1, Dedendum=1.25),
 dict(Module=1, PressureAngle=14.5,Teeth=24,ProfileShift=0.0, HelixAngle=0,  Addendum=1, Dedendum=1.25),
 dict(Module=1, PressureAngle=25, Teeth=13, ProfileShift=-0.3,HelixAngle=0,  Addendum=1, Dedendum=1.25),
 dict(Module=1, PressureAngle=20, Teeth=17, ProfileShift=0.2, HelixAngle=30, Addendum=1, Dedendum=1.25),
 dict(Module=2, PressureAngle=20, Teeth=40, ProfileShift=0.0, HelixAngle=0,  Addendum=1, Dedendum=1.25),
 dict(Module=1, PressureAngle=20, Teeth=12, ProfileShift=0.6, HelixAngle=0,  Addendum=1.2,Dedendum=1.4),
 dict(Module=1, PressureAngle=20, Teeth=100,ProfileShift=0.0, HelixAngle=0,  Addendum=1, Dedendum=1.25),
]

fig, axes = plt.subplots(3, 3, figsize=(15, 15))
for ax, c in zip(axes.ravel(), cases):
    c = dict(c); c["RootFilletCoef"] = 0.38
    T = np.linspace(0, c["Teeth"], c["Teeth"]*300+1)
    x, y, env = evaluate(c, T)
    ax.plot(x, y, lw=0.9)
    th = np.linspace(0, 2*np.pi, 400)
    for rr, st in ((env["_Rb"], ":"), (env["_R"], "--")):
        ax.plot(rr*np.cos(th), rr*np.sin(th), st, lw=0.5, color="0.6")
    ax.set_aspect("equal"); ax.axis("off")
    ax.set_title("z=%d a=%.1f x=%.1f b=%d m=%g" % (c["Teeth"], c["PressureAngle"],
                 c["ProfileShift"], c["HelixAngle"], c["Module"]), fontsize=9)
plt.tight_layout(); plt.savefig("matrix.png", dpi=95)

# zoom on one tooth of the reference case
c = dict(Module=1, PressureAngle=20, Teeth=17, ProfileShift=0.2, HelixAngle=0,
         Addendum=1, Dedendum=1.25, RootFilletCoef=0.38)
T = np.linspace(0.0, 2.0, 4000)
x, y, env = evaluate(c, T)
fig, ax = plt.subplots(figsize=(7, 7))
ax.plot(x, y, lw=1.2, marker=".", ms=1.4)
th = np.linspace(-0.6, 0.6, 300)
for rr, lab in ((env["_Rb"], "base"), (env["_R"], "pitch"), (env["_Rf"], "root"), (env["_Ra"], "tip")):
    ax.plot(rr*np.cos(th), rr*np.sin(th), "--", lw=0.6, color="0.6")
    ax.annotate(lab, (rr*np.cos(0.55), rr*np.sin(0.55)), fontsize=8, color="0.4")
ax.set_aspect("equal"); ax.set_title("two teeth, z=17 x=0.2 (dots = sample density)")
plt.tight_layout(); plt.savefig("tooth.png", dpi=110)
print("ok")
