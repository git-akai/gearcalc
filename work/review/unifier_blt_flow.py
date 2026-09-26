# Prototype: flow.rs's 2^M enumeration vs a block-triangular (BLT) sequential solve.
import itertools, random, sys
from fractions import Fraction as Fr
ZERO=1e-12
def gauss(A,b):
    n=len(A); m=len(A[0]) if A else 0
    M=[row[:]+[bb] for row,bb in zip(A,b)]
    piv=[];r=0
    for c in range(m):
        p=max(range(r,n),key=lambda i:abs(M[i][c]),default=None)
        if p is None or abs(M[p][c])<1e-12: continue
        M[r],M[p]=M[p],M[r]; pv=M[r][c]; M[r]=[x/pv for x in M[r]]
        for i in range(n):
            if i!=r and M[i][c]!=0:
                f=M[i][c]; M[i]=[x-f*y for x,y in zip(M[i],M[r])]
        piv.append(c); r+=1
    if len(piv)<m: return None
    sc=max([1]+[abs(M[i][m]) for i in range(n)])
    if any(abs(M[i][m])>1e-9*sc for i in range(r,n)): return None
    x=[0]*m
    for i,c in enumerate(piv): x[c]=M[i][m]
    return x
def per_unit(mesh,d,f,s):
    a,b,fr,za,zb=mesh[:5]
    on_a,on_b=(za,f*zb) if d==0 else (f*za,zb)
    t=0
    if a==s:t+=on_a
    if b==s:t+=on_b
    if fr==s:t-=on_a+on_b
    return t
def check(meshes,speed,c,dirs,idx,pin):
    for k in idx:
        a,b,fr,za,zb=meshes[k][:5]
        p=c[k]*za*(speed[a]-speed[fr]) if dirs[k]==0 else c[k]*zb*(speed[b]-speed[fr])
        if not(abs(p)<=ZERO*pin or p>0): return False
    return True
def enumerate_flow(nb,meshes,speed,known):
    pin=sum(abs(t*speed[s]) for s,t in known.items())
    best=None
    for dirs in itertools.product([0,1],repeat=len(meshes)):
        f=[m[5] if d==0 else m[6] for m,d in zip(meshes,dirs)]
        rows=[[per_unit(m,d,fk,s) for m,d,fk in zip(meshes,dirs,f)] for s in sorted(known)]
        c=gauss(rows,[known[s] for s in sorted(known)])
        if c is None or not check(meshes,speed,c,dirs,range(len(meshes)),pin): continue
        sh=[sum(per_unit(m,d,fk,s)*ck for m,d,fk,ck in zip(meshes,dirs,f,c)) for s in range(nb)]
        pw=[sh[s]*speed[s] for s in range(nb)]
        if sum(pw)< -ZERO*pin: continue
        pi=sum(p for p in pw if p>ZERO*pin); po=sum(p for p in pw if p<-ZERO*pin)
        if pi<=ZERO*pin: continue
        e=abs(po)/pi
        if best is None or e>best[0]: best=(e,dirs,c)
    return best
# --- BLT: bipartite matching rows(known bodies) x cols(meshes), Tarjan SCC
def blt(nb,meshes,known):
    rows=sorted(known); M=len(meshes)
    adj={r:[k for k,m in enumerate(meshes) if r in m[:3]] for r in rows}
    match_col={}
    def aug(r,seen):
        for k in adj[r]:
            if k in seen: continue
            seen.add(k)
            if k not in match_col or aug(match_col[k],seen):
                match_col[k]=r; return True
        return False
    for r in rows: aug(r,set())
    if len(match_col)<M: return None
    row_of={k:r for k,r in match_col.items()}
    # graph on columns: k -> j if row_of[k] uses j (j must be solved before k)
    g={k:[j for j in adj[row_of[k]] if j!=k] for k in range(M)}
    idx={};low={};st=[];on=set();out=[];cnt=[0]
    sys.setrecursionlimit(10000)
    def sc(v):
        idx[v]=low[v]=cnt[0];cnt[0]+=1;st.append(v);on.add(v)
        for w in g[v]:
            if w not in idx: sc(w);low[v]=min(low[v],low[w])
            elif w in on: low[v]=min(low[v],idx[w])
        if low[v]==idx[v]:
            comp=[]
            while True:
                w=st.pop();on.discard(w);comp.append(w)
                if w==v:break
            out.append(comp)
    for v in range(M):
        if v not in idx: sc(v)
    return out,row_of  # Tarjan emits in dependency order (deps first)
def blt_flow(nb,meshes,speed,known):
    pin=sum(abs(t*speed[s]) for s,t in known.items())
    r=blt(nb,meshes,known)
    if r is None: return None
    blocks,row_of=r
    M=len(meshes); work=[0]; best=[None]; branches=[0]
    def fin(c,dirs,f):
        sh=[sum(per_unit(m,d,fk,s)*ck for m,d,fk,ck in zip(meshes,dirs,f,c)) for s in range(nb)]
        pw=[sh[s]*speed[s] for s in range(nb)]
        if sum(pw)< -ZERO*pin: return
        pi=sum(p for p in pw if p>ZERO*pin); po=sum(p for p in pw if p<-ZERO*pin)
        if pi<=ZERO*pin: return
        e=abs(po)/pi
        if best[0] is None or e>best[0][0]: best[0]=(e,tuple(dirs))
    def go(bi,c,dirs,f):
        if bi==len(blocks): branches[0]+=1; fin(c,dirs,f); return
        comp=blocks[bi]; brows=[row_of[k] for k in comp]
        for dd in itertools.product([0,1],repeat=len(comp)):
            work[0]+=1
            d2=dirs[:]; f2=f[:]
            for k,d in zip(comp,dd): d2[k]=d; f2[k]=meshes[k][5] if d==0 else meshes[k][6]
            A=[[per_unit(meshes[k],d2[k],f2[k],s) for k in comp] for s in brows]
            rhs=[known[s]-sum(per_unit(meshes[j],d2[j],f2[j],s)*c[j] for j in range(M) if c[j] is not None) for s in brows]
            x=gauss(A,rhs)
            if x is None: continue
            cc=c[:]
            for k,v in zip(comp,x): cc[k]=v
            if check(meshes,speed,cc,d2,comp,pin): go(bi+1,cc,d2,f2)
    go(0,[None]*M,[None]*M,[None]*M)
    if best[0] is None: return ('none',[len(b) for b in blocks])
    return (best[0][0],best[0][1],[len(b) for b in blocks],work[0],branches[0])
def speeds(nb,meshes,fixed):
    # kinematic rows za(wa-wf)+zb(wb-wf)=0 ; fixed: {body:value}
    A=[];b=[]
    for m in meshes:
        a,bb,fr,za,zb=m[:5]; row=[0.0]*nb
        row[a]+=za; row[bb]+=zb; row[fr]-=za+zb; A.append(row); b.append(0.0)
    for s,v in fixed.items():
        row=[0.0]*nb; row[s]=1.0; A.append(row); b.append(v)
    return gauss(A,b)
def eta(): return random.uniform(0.3,0.995)
def run(name,nb,meshes,fixed,known):
    fixed=dict(fixed); fixed[0]=0.0
    sp=speeds(nb,meshes,fixed)
    if sp is None: print(name,'no motion');return
    e=enumerate_flow(nb,meshes,sp,known); bl=blt_flow(nb,meshes,sp,known)
    ok = (e is None and (bl is None or bl[0]=='none')) or (e and bl and bl[0]!='none' and abs(e[0]-bl[0])<1e-9 and tuple(e[1])==bl[1])
    return ok,e,bl
random.seed(1)
fails=0;tot=0;hist={};multi={}
for trial in range(3000):
    kind=trial%4
    if kind==0: # chain of n external pairs, shafts 1..n+1
        n=random.randint(1,7); nb=n+2
        ms=[(k+1,k+2,0,random.randint(12,60),random.randint(12,60),eta(),eta()) for k in range(n)]
        T=random.choice([1,-1]); inp=random.choice([1,n+1]); out=n+2-inp
        known={s:0.0 for s in range(1,nb) if s not in (out,)}; known[inp]=T
        res=run('chain',nb,ms,{inp:1.0},known)
    elif kind==1: # simple planetary: 1 sun,2 planet,3 carrier,4 ring ; hold one of sun/carrier/ring
        zs,zp=random.randint(12,40),random.randint(12,40); zr=zs+2*zp
        ms=[(1,2,3,zs,zp,eta(),eta()),(2,4,3,zp,-zr,eta(),eta())]
        held=random.choice([1,3,4]); ins=[s for s in (1,3,4) if s!=held]; random.shuffle(ins); inp,out=ins
        known={inp:random.choice([1,-1]),2:0.0}
        res=run('planetary',5,ms,{held:0.0,inp:1.0},known)
    elif kind==2: # wolfrom: 1 sun 2 planet 3 carrier(free) 4 ring1 5 ring2
        zs,zp1=random.randint(12,30),random.randint(12,30); zr1=zs+2*zp1
        zp2=zp1+random.choice([-3,-2,-1,1,2,3]); zr2=zr1+random.choice([-3,-2,-1,1,2,3])
        ms=[(1,2,3,zs,zp1,eta(),eta()),(2,4,3,zp1,-zr1,eta(),eta()),(2,5,3,zp2,-zr2,eta(),eta())]
        held=random.choice([4,5]); ins=[s for s in (1,4,5) if s!=held]; random.shuffle(ins); inp,out=ins
        known={inp:random.choice([1,-1]),2:0.0,3:0.0}
        res=run('wolfrom',6,ms,{held:0.0,inp:1.0},known)
    else: # two planetary sets in series: set A (1 sun,2 pl,3 carrier=set B sun,4 ringA held) set B (3 sun,5 pl,6 carrier out,7 ringB held)
        zs,zp=random.randint(12,30),random.randint(12,30); zr=zs+2*zp
        zs2,zp2=random.randint(12,30),random.randint(12,30); zr2=zs2+2*zp2
        ms=[(1,2,3,zs,zp,eta(),eta()),(2,4,3,zp,-zr,eta(),eta()),(3,5,6,zs2,zp2,eta(),eta()),(5,7,6,zp2,-zr2,eta(),eta())]
        known={1:1.0,2:0.0,3:0.0,5:0.0}
        res=run('compound',8,ms,{4:0.0,7:0.0,1:1.0},known)
    if res is None: continue
    ok,e,bl=res; tot+=1
    key=(['chain','planetary','wolfrom','compound'][kind], tuple(sorted(bl[2])) if bl and bl[0]!='none' else 'none')
    hist[key]=hist.get(key,0)+1
    if bl and bl[0]!='none' and bl[4]>1: multi[key]=multi.get(key,0)+1
    if not ok:
        fails+=1
        if fails<6: print('MISMATCH',kind,e and (round(e[0],6),e[1]),bl)
print('total',tot,'mismatches',fails); print('cases with >1 consistent full branch:',multi)
for k,v in sorted(hist.items(),key=str): print(k,v)
