"""Check fixed spherical stability witnesses using mpmath1.4.1; no writes."""
import json,sys,struct
from pathlib import Path
import mpmath as m
assert sys.argv[1:]==['--check']
assert m.__version__=='1.4.1'
data=json.loads((Path(__file__).parent/'witnesses.json').read_text())
def compute(c,dps):
    m.mp.dps=dps
    r=m.pi/180
    positions=c['geometry']['coordinates'][0]
    vertices=[]
    for lon,lat in positions[:-1]:
        # mp.mpf(float) uses the exact binary64 rational, never decimal text.
        lo=m.mpf(lon)*r;la=m.mpf(lat)*r
        vertices.append([m.cos(la)*m.cos(lo),m.cos(la)*m.sin(lo),m.sin(la)])
    def cross(a,b):return[a[1]*b[2]-a[2]*b[1],a[2]*b[0]-a[0]*b[2],a[0]*b[1]-a[1]*b[0]]
    sums=[m.mpf(0)]*3
    edges=[]
    for a,b in zip(vertices,vertices[1:]+vertices[:1]):
        v=cross(a,b);edges.append(v);n=m.sqrt(sum(x*x for x in v))
        sums=[s-m.asin(n)/n*x for s,x in zip(sums,v)]
    n=m.sqrt(sum(x*x for x in sums))
    answer=[m.atan2(sums[1],sums[0])/r,m.asin(sums[2]/n)/r]
    lo,la=[m.mpf(v)*r for v in c['queries'][2]]
    q=[m.cos(la)*m.cos(lo),m.cos(la)*m.sin(lo),m.sin(la)]
    sides=[sum(x*y for x,y in zip(q,v)) for v in edges]
    if isinstance(c['id'],int):
        assert all(x<0 for x in sides)
    return answer
maximum=m.mpf(0)
for c in data['cases']:
    expected_bits=c['sourceBits']
    assert expected_bits==[[struct.pack('>d',v).hex() for v in p] for p in c['geometry']['coordinates'][0]]
    a=compute(c,80);b=compute(c,120)
    assert max(abs(x-y) for x,y in zip(a,b))<m.mpf('1e-55')
    expected=[m.mpf(v) for v in c['highPrecision']['oracleCentroid']]
    error=max(abs(x-y) for x,y in zip(b,expected));maximum=max(maximum,error)
    assert error<m.mpf('1e-55')
print(json.dumps({'passed':True,'checkOnly':True,'cases':len(data['cases']),'precisions':[80,120],'maximumStoredErrorDegrees':str(maximum),'source':'exact input binary64 rational; independent spherical vector integral and convex-edge center signs','mpmath':m.__version__}))
