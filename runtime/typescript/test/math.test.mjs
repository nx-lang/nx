// Independent binary64 references across TypeScript IR and generated TypeScript/JavaScript.
import assert from "node:assert/strict";
import { readFileSync, writeFileSync, readdirSync, mkdtempSync, rmSync } from "node:fs";
import { tmpdir } from "node:os";
import { resolve, join } from "node:path";
import { fileURLToPath, pathToFileURL } from "node:url";
import { spawnSync } from "node:child_process";
import { evaluateFunction, prepareNxIrModule, linkNxIrProgram, callFunction, NxIrRuntimeError } from "../dist/src/index.js";
import { nxStandardMathNames, nxStandardMathArities } from "../dist/src/math.js";

const root = fileURLToPath(new URL("../../..", import.meta.url));
const corpus = JSON.parse(readFileSync(join(root,"crates/nx-value/src/math-cases.json"),"utf8"));
const dir = mkdtempSync(join(tmpdir(),"nx-math-"));
function run(command,args,cwd=root) {
  const result = spawnSync(command,args,{cwd,encoding:"utf8"});
  assert.equal(result.status,0,`${command} ${args.join(" ")}\n${result.stdout}\n${result.stderr}`);
}
const parameters = name => name === "atan2" ? ["y","x"] : name === "clamp" ? ["value","min","max"]
  : name === "lerp" ? ["a","b","t"] : name === "min" || name === "max" ? ["a","b"] : name === "hypot" ? ["x","y"] : ["x"];
function check(evaluate) {
  for (const fixture of corpus) {
    const args = fixture.args.map(Number);
    if (fixture.error) {
      assert.throws(() => evaluate(fixture.op,args),error => {
        const code = error instanceof NxIrRuntimeError ? error.diagnostics[0].code : error.code;
        return code === fixture.error && error.message.includes(fixture.op);
      },fixture.name);
    } else {
      const actual = evaluate(fixture.op,args);
      const expected = Number(fixture.expected);
      if (fixture.exact) assert.ok(Object.is(actual,expected),`${fixture.name}: ${actual} vs ${expected} (including zero sign)`);
      else assert.ok(Math.abs(actual-expected) <= 1e-12*Math.max(1,Math.abs(expected)),fixture.name);
    }
  }
}
try {
  const source = ['import "@nx/math" as math', ...nxStandardMathNames.map(name => {
    const params = parameters(name);
    assert.equal(params.length,nxStandardMathArities[nxStandardMathNames.indexOf(name)]);
    return `export let ${name}(${params.map(p=>`${p}:float64`).join(",")}):float64 = { math.${name}(${params.join(",")}) }`;
  }), 'type Numeric = <function x:float64/>:float64',
  'let pass(f:Numeric):Numeric = { f }',
  'let callback:Numeric = { pass(math.sqrt) }',
  'export let indirect(x:float64):float64 = <callback x={x}/>',
  'export let constant():float64 = { math.tau }'].join("\n");
  const path = join(dir,"main.nx");
  writeFileSync(path,source);
  const out = join(dir,"ir");
  run("cargo",["run","-q","-p","nx-cli","--","codegen",path,"--target","nx-ir","--output",out]);
  const modules = new Map(readdirSync(out,{recursive:true}).filter(p=>p.endsWith(".nxir")).map(p=>{
    const module=prepareNxIrModule(readFileSync(join(out,p)));return [module.identity,module];
  }));
  const program = linkNxIrProgram([...modules.values()].find(module=>module.identity.endsWith("main.nx")),{resolve:identity=>modules.get(identity)});
  check((op,args)=>evaluateFunction(program,op,args));
  assert.equal(evaluateFunction(program,"indirect",[4]),2);
  assert.throws(()=>evaluateFunction(program,"indirect",[-1]),error=>
    error.diagnostics[0].code === "nx-math-domain" && error.diagnostics[0].source?.identity.endsWith("main.nx"));
  assert.equal(callFunction(program,{$type:"Function",module:"@nx/math/math.nx",name:"sin"},{x:0}),0);
  assert.throws(()=>evaluateFunction(program,"sin",[0],{maxOperations:1}),error=>error.diagnostics[0].code === "nx-ir-resource-limit");
  const usage={};evaluateFunction(program,"sin",[0],{usage,maxOperations:1000});
  assert.throws(()=>evaluateFunction(program,"sin",[0],{maxOperations:usage.operations-1}),error=>error.diagnostics[0].code === "nx-ir-resource-limit");
  // Existing source generators reject function-value constructs. Keep the diagnostic and
  // exercise the supported direct-call subset separately.
  const unsupported=spawnSync("cargo",["run","-q","-p","nx-cli","--","codegen",path,"--target","javascript","--output",join(dir,"unsupported")],{cwd:root,encoding:"utf8"});
  assert.notEqual(unsupported.status,0);
  assert.match(unsupported.stderr,/unsupported|function.*value|function-typed/i);
  writeFileSync(path,source.split("\ntype Numeric")[0]+'\nexport let constant():float64 = { math.tau }');
  for (const target of ["typescript","javascript"]) {
    const dest=join(dir,target);
    run("cargo",["run","-q","-p","nx-cli","--","codegen",path,"--target",target,"--output",dest]);
    writeFileSync(join(dest,"package.json"),'{"type":"module"}');
    if(target === "typescript") run(join(root,"runtime/typescript/node_modules/.bin/tsc"),["--target","es2022","--module","es2022","--moduleResolution","bundler","--strict",...readdirSync(dest).filter(p=>p.endsWith(".ts"))],dest);
    const generated=await import(pathToFileURL(join(dest,"index.js")));
    check((op,args)=>generated[op](...args));
    assert.equal(generated.constant(),2*Math.PI);
    const runtime=await import(pathToFileURL(join(dest,"nx-runtime.js")));
    try { generated.sqrt(-1); assert.fail("sqrt domain must fail"); }
    catch(error) { assert.equal(runtime.nxDiagnosticsFromError(error)[0].code,"nx-math-domain"); }
  }
  writeFileSync(path, readFileSync(join(root,"examples/math-graphics/main.nx")));
  const geometryOut=join(dir,"geometry");
  run("cargo",["run","-q","-p","nx-cli","--","codegen",path,"--target","nx-ir","--output",geometryOut]);
  const geometryModules=new Map(readdirSync(geometryOut,{recursive:true}).filter(p=>p.endsWith(".nxir")).map(p=>{
    const module=prepareNxIrModule(readFileSync(join(geometryOut,p)));return [module.identity,module];
  }));
  const geometryProgram=linkNxIrProgram([...geometryModules.values()].find(m=>m.identity.endsWith("main.nx")),{resolve:id=>geometryModules.get(id)});
  const geometry=evaluateFunction(geometryProgram,"root");
  assert.equal(geometry.polygon.length,3);assert.equal(geometry.star.length,6);assert.equal(geometry.arrow.length,5);
  for (const [index,x,y] of [[0,10,0],[1,-5,8.660254037844386],[2,-5,-8.660254037844386]]) {
    assert.ok(Math.abs(geometry.polygon[index].x-x)<1e-12);
    assert.ok(Math.abs(geometry.polygon[index].y-y)<1e-12);
  }
  assert.equal(geometry.arrow[2].x,1);assert.equal(geometry.arrow[2].y,3);
  console.log(`math: ${corpus.length} reference cases pass in TypeScript IR and generated TS/JS`);
} finally { rmSync(dir,{recursive:true,force:true}); }
