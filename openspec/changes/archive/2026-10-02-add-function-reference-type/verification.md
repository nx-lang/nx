# End-to-end check (task 10.3)

A sample shaped like the agent library's use of the type, kept here so `add-agent-library` and
`add-agent-host-package` can reuse the commands. Run from a directory holding the three files, so
the module identity is `main.nx`.

`main.nx`:

```nx
type HttpArguments = { url:string query?:string }

type FunctionTool = { name:string function: <function ... />: object* }
type HttpTool = { name:string arguments: <function ... />: HttpArguments }
type Agent = { tools:object+ }

/// Finds the plans sold in a region.
let findPlans(region:string, limit?:int): string* = {region}

/// Builds the request for one plan.
let planRequest(plan:string, verbose:boolean = false): HttpArguments = <HttpArguments url={"/plans/" + plan} />

let root() = <Agent tools={
  <FunctionTool name="find_plans" function={findPlans} />
  <HttpTool name="get_plan" arguments={planRequest} />
} />
```

`wrong.nx`, which the compiler rejects:

```nx
type HttpArguments = { url:string }
type HttpTool = { name:string arguments: <function ... />: HttpArguments }
let findPlans(region:string): string* = {region}
let root() = <HttpTool name="bad" arguments={findPlans} />
```

`run.mjs` (the import is `@nx-lang/ir-runtime` outside this repository):

```js
import { readFileSync } from "node:fs";
import { callFunction, evaluateFunction, prepareNxIrProgram } from "/home/bret/src/nx/runtime/typescript/dist/src/index.js";

const program = prepareNxIrProgram(readFileSync("out/main.nxir"));
const agent = evaluateFunction(program, "root");
console.log(JSON.stringify(agent));
const [functionTool, httpTool] = agent.tools;
console.log(JSON.stringify(callFunction(program, functionTool.function, { region: "emea", limit: 3 })));
console.log(JSON.stringify(callFunction(program, httpTool.arguments, { plan: "pro" })));
try {
  callFunction(program, httpTool.arguments, { plan: 7 });
} catch (error) {
  console.log("refused:", error.message);
}
```

Commands and output:

```text
$ nxlang run wrong.nx
error wrong.nx:4:35: Record field 'arguments' on 'HttpTool' expects <function ... />: HttpArguments, found <function region:string />: string*; the result string* is not HttpArguments

$ nxlang codegen main.nx --target nx-ir --output out
$ nxlang ir explain out/main.nxir | sed -n 2p
requires function-values-v1 function-reference-type-v1

$ node run.mjs
{"$type":"Agent","tools":[{"$type":"FunctionTool","name":"find_plans","function":{"$type":"Function","module":"main.nx","name":"findPlans"}},{"$type":"HttpTool","name":"get_plan","arguments":{"$type":"Function","module":"main.nx","name":"planRequest"}}]}
["emea"]
{"$type":"HttpArguments","url":"/plans/pro"}
refused: Expected plan to be a string.
```
