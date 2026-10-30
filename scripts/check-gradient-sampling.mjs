// Independent analytic reference, kept outside renderer implementations.
import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
const cases = JSON.parse(readFileSync(new URL('../crates/nx-graphics-skia/fixtures/gradients/sampling.json', import.meta.url)));
export function sample(paint, x, y) {
  const t = paint.kind === 'linearGradient'
    ? ((x-paint.x1)*(paint.x2-paint.x1)+(y-paint.y1)*(paint.y2-paint.y1))/((paint.x2-paint.x1)**2+(paint.y2-paint.y1)**2)
    : Math.hypot(x-paint.cx,y-paint.cy)/paint.radius;
  const stops=paint.stops;
  const upper=stops.findIndex(s=>s.offset>t);
  const premul=c=>[c[0]*c[3],c[1]*c[3],c[2]*c[3],c[3]];
  if(upper===0) return premul(stops[0].color);
  if(upper===-1) return premul(stops.at(-1).color);
  const a=stops[upper-1],b=stops[upper],w=(t-a.offset)/(b.offset-a.offset);
  return premul(a.color).map((c,i)=>c*(1-w)+premul(b.color)[i]*w);
}
for(const test of cases) for(const s of test.samples) {
  sample(test.paint,s.x+.5,s.y+.5).forEach((v,i)=>assert.ok(Math.abs(v-s.expected[i])<1e-12, test.name));
}
assert.deepEqual(sample(cases[2].paint,50.5,.5),[0,0,1,1]);
console.log('Analytic gradient corpus passes, including exact hard-stop ordering.');
