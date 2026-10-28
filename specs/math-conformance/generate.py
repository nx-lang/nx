"""Regenerate checked-in references: mpmath 1.3.0, 500 decimal digits.

Inputs are converted to binary64 first, then represented exactly in mpmath. Reference
results are rounded once to binary64. Edge semantics specified by NX are explicit,
not inferred from the host's Math implementation. No generator dependency is needed
by the test suite.
"""
import json
from pathlib import Path
import mpmath as mp
mp.mp.dps = 500
cases = []

def add(name, op, args, expected=None, exact=False, error=None):
    if expected is None and error is None:
        values = [mp.mpf(float(a)) for a in args]
        fn = {'asin': mp.asin, 'acos': mp.acos, 'atan': mp.atan,
              'atan2': mp.atan2, 'hypot': lambda x,y: mp.sqrt(x*x+y*y),
              'lerp': lambda a,b,t: (1-t)*a+t*b,
              'degreesToRadians': lambda x: x*mp.mpf(float(mp.pi/180)),
              'radiansToDegrees': lambda x: x*mp.mpf(float(180/mp.pi))}.get(op, getattr(mp, op, None))
        expected = repr(float(fn(*values)))
    case = dict(name=name, op=op, args=args)
    if error: case['error'] = error
    else: case.update(expected=expected, exact=exact)
    cases.append(case)

for op in ['sin','cos','tan','asin','acos','atan','sqrt']:
    add(op+'-ordinary', op, ['0.5'])
for op in ['sin','tan','asin','atan','sqrt','sign','floor','ceil','round','roundTiesToEven','trunc','degreesToRadians','radiansToDegrees']:
    add(op+'-negative-zero',op,['-0.0'],'-0.0',True)
for op in ['cos','abs','sign']:
    add(op+'-zero',op,['0.0'],'1.0' if op=='cos' else '0.0',True)
add('asin-upper-bound','asin',['1.0'])
add('acos-upper-bound','acos',['1.0'],'0.0',True)
add('atan-positive-zero','atan',['0.0'],'0.0',True)
add('abs-negative-zero','abs',['-0.0'],'0.0',True)
for op in ['min','max']:
    for args in [['0.0','-0.0'],['-0.0','0.0']]:
        add(op+'-zeros-'+args[0],op,args,'-0.0' if op=='min' else '0.0',True)
    add(op+'-ordinary',op,['-2.0','3.0'],'-2.0' if op=='min' else '3.0',True)
for op in ['round','roundTiesToEven']:
    for x, js, even in [('2.5','3.0','2.0'),('3.5','4.0','4.0'),('-2.5','-2.0','-2.0'),('-3.5','-3.0','-4.0'),('-0.5','-0.0','-0.0'),('0.49999999999999994','0.0','0.0'),('4503599627370495.5','4503599627370496.0','4503599627370496.0')]:
        add(op+'-'+x,op,[x],js if op=='round' else even,True)
for op, expected in [('floor','-1.0'),('ceil','-0.0'),('trunc','-0.0')]:
    add(op+'-negative-fraction',op,['-0.1'],expected,True)
for op in ['sin','cos','tan']:
    add(op+'-huge-angle',op,['1e300'])
add('tan-near-pole','tan',['1.5707963267948966'])
for args in [['1.0','1.0'],['1.0','-1.0'],['-1.0','-1.0'],['-1.0','1.0']]:
    add('atan2-quadrant-'+','.join(args),'atan2',args)
for args, result in [(['0.0','0.0'],'0.0'),(['-0.0','0.0'],'-0.0'),(['0.0','-0.0'],repr(float(mp.pi))),(['-0.0','-0.0'],repr(-float(mp.pi)))]:
    add('atan2-zero-'+','.join(args),'atan2',args,result,True)
add('hypot-large','hypot',['1e308','1e308'])
add('hypot-zero','hypot',['-0.0','-0.0'],'0.0',True)
add('hypot-subnormal','hypot',['5e-324','0.0'],'5e-324',True)
add('sin-subnormal','sin',['5e-324'],'5e-324',True)
add('sqrt-subnormal','sqrt',['5e-324'])
for op in ['degreesToRadians','radiansToDegrees']:
    add(op+'-ordinary',op,['90.0'])
add('degrees-no-intermediate-overflow','degreesToRadians',['1e308'])
for args,result in [(['-1e308','1e308','0.5'],'0.0'),(['-0.0','1.0','0.0'],'-0.0'),(['1.0','-0.0','1.0'],'-0.0'),(['-0.0','0.0','0.5'],'-0.0'),(['0.0','10.0','1.5'],'15.0')]:
    add('lerp-'+','.join(args),'lerp',args,result,True)
add('lerp-negative-extrapolation','lerp',['0.0','10.0','-0.5'],'-5.0',True)
add('lerp-interior-inexact','lerp',['0.1','0.3','0.7'])
add('lerp-extrapolation-no-intermediate-overflow','lerp',['1.7e308','1e307','1.2'])
add('lerp-negative-extrapolation-no-intermediate-overflow','lerp',['1e307','1.7e308','-1.15'])
add('sign-negative','sign',['-2.0'],'-1.0',True)
add('sign-positive','sign',['2.0'],'1.0',True)
add('clamp-low','clamp',['-1.0','0.0','1.0'],'0.0',True)
add('clamp-high','clamp',['2.0','0.0','1.0'],'1.0',True)
add('clamp-zero','clamp',['-0.0','0.0','1.0'],'0.0',True)
add('clamp-equal-bounds','clamp',['5.0','2.0','2.0'],'2.0',True)
add('clamp-inclusive-low','clamp',['2.0','2.0','5.0'],'2.0',True)
add('clamp-inclusive-high','clamp',['5.0','2.0','5.0'],'5.0',True)
for op,args in [('sqrt',['-1.0']),('asin',['1.1']),('acos',['-1.1']),('clamp',['0.0','2.0','1.0'])]:
    add(op+'-domain',op,args,error='nx-math-domain')
for op,args in [('hypot',['1.7e308','1.7e308']),('radiansToDegrees',['1e308']),('lerp',['0.0','1e308','2.0'])]:
    add(op+'-overflow',op,args,error='nx-math-overflow')
arities = {op: 1 for op in ['sin','cos','tan','asin','acos','atan','sqrt','abs','sign','floor','ceil','round','roundTiesToEven','trunc','degreesToRadians','radiansToDegrees']}
arities.update({op: 2 for op in ['atan2','hypot','min','max']})
arities.update(clamp=3, lerp=3)
for op, arity in arities.items():
    for position in range(arity):
        for value in ['NaN','Infinity','-Infinity']:
            args = ['0.0'] * arity
            args[position] = value
            add(f'{op}-nonfinite-{position}-{value}',op,args,error='nx-math-non-finite-input')
(Path(__file__).resolve().parents[2] / 'crates/nx-value/src/math-cases.json').write_text(json.dumps(cases,indent=2)+'\n')
