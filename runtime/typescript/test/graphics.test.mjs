// Graphics authoring uses ordinary emitted NX IR and the transitive standard math image.
import assert from "node:assert/strict";
import {readFileSync,readdirSync,mkdtempSync,rmSync} from "node:fs";
import {tmpdir} from "node:os";
import {join} from "node:path";
import {fileURLToPath} from "node:url";
import {spawnSync} from "node:child_process";
import {declarationAtOrigin,evaluateFunction,prepareNxIrModule,linkNxIrProgram} from "../dist/src/index.js";
const root=fileURLToPath(new URL("../../..",import.meta.url));
const out=mkdtempSync(join(tmpdir(),"nx-graphics-ir-"));
try {
 const run=spawnSync("cargo",["run","-q","-p","nx-cli","--","codegen",join(root,"examples/graphics/runtime.nx"),"--target","nx-ir","--output",out],{cwd:root,encoding:"utf8"});
 assert.equal(run.status,0,run.stderr);
 const modules=new Map(readdirSync(out,{recursive:true}).filter(p=>p.endsWith(".nxir")).map(p=>{const module=prepareNxIrModule(readFileSync(join(out,p)));return [module.identity,module]}));
 assert.ok(modules.has("@nx/math/math.nx"));assert.ok(modules.has("@nx/graphics/graphics.nx"));
 const entry=[...modules.values()].find(m=>m.identity.endsWith("runtime.nx"));
 const program=linkNxIrProgram(entry,{resolve:id=>modules.get(id)});
 const value=name=>evaluateFunction(program,name,[]);
 const near=(a,b)=>assert.ok(Math.abs(a-b)<=1e-5*Math.max(1,Math.abs(b)),`${a} vs ${b}`);
 const polygon=value("polygon");assert.equal(polygon.$type,"Polygon");assert.equal(polygon.points.length,3);near(polygon.points[1].x,-5);near(polygon.points[1].y,Math.sqrt(75));
 for(const [name,count] of [["star",5],["revisedStar",7]]) {const s=value(name);assert.equal(s.points.length,count*2);s.points.forEach((p,i)=>near(Math.hypot(p.x,p.y),i%2?5:10));near(s.points[0].x,0);near(s.points[0].y,-10);}
 const large=value("largeRotation").points[0];near(large.x,10*Math.cos((1e100%360)*Math.PI/180));near(large.y,10*Math.sin((1e100%360)*Math.PI/180));near(value("periodicRotation").points[0].y,10);assert.equal((value("periodicArc").data.match(/ A /g)||[]).length,2);
 assert.equal(value("nonfiniteArc").data,"nx-graphics-invalid: Arc radius must be finite");assert.equal(value("nonfiniteRing").data,"nx-graphics-invalid: RingSector outerRadius must be finite");
 const arrow=value("arrow");assert.equal(arrow.points.length,7);assert.deepEqual(arrow.points[3],{$type:"Point",x:30,y:40});
 assert.equal(value("invalidArrow").$type,"Path");assert.match(value("invalidArrow").data,/nx-graphics-invalid: Arrow/);
 assert.equal(value("ring").fillRule,"evenodd");assert.equal((value("ring").data.match(/ A /g)||[]).length,4);
 assert.equal((value("arc").data.match(/ A /g)||[]).length,2);assert.equal(value("zeroArc").data,"");
 assert.equal(value("aliasedValue").width,3);
 assert.throws(()=>value("numericalFailure"),e=>e.diagnostics[0].code==="nx-math-domain");
 assert.throws(()=>linkNxIrProgram(entry,{resolve:id=>id==="@nx/math/math.nx"?undefined:modules.get(id)}));
 const baseOrigins={},callOrigins={includeCallSites:true},baseUsage={},callUsage={};
 const base=evaluateFunction(program,"arrow",[],{origins:baseOrigins,usage:baseUsage,maxOperations:5000000});
 const traced=evaluateFunction(program,"arrow",[],{origins:callOrigins,usage:callUsage,maxOperations:5000000});
 assert.deepEqual(traced,base);assert.deepEqual(callUsage,baseUsage);
 assert.deepEqual(callOrigins.entries.map(({callSites,...entry})=>entry),baseOrigins.entries);
 const construction=callOrigins.entries.find(entry=>entry.path==="");
 const declaration=declarationAtOrigin(program,construction);
 assert.equal(declaration.name,"Polygon");assert.equal(declaration.module.identity,"@nx/graphics/graphics.nx");
 assert.equal(declarationAtOrigin(program,{...construction,module:"missing.nx"}),undefined);
 assert.equal(declarationAtOrigin(program,{...construction,start:-1,end:-1}),undefined);
 const sites=construction.callSites;
 assert.ok(sites.length>=2);assert.equal(sites[0].module,"@nx/graphics/graphics.nx");assert.equal(sites.at(-1).module,entry.identity);
 const invalid={entries:[{path:"old"}],includeCallSites:"yes"};
 assert.throws(()=>evaluateFunction(program,"arrow",[],{origins:invalid}),error=>error.diagnostics[0].code==="nx-ir-options");
 assert.deepEqual(invalid.entries,[]);
 console.log("Optional call-site origins preserve construction, values and operation costs, innermost first.");
 console.log("Graphics helpers and aliased values pass on TypeScript IR (geometry tolerance 1e-5).");
} finally {rmSync(out,{recursive:true,force:true});}
