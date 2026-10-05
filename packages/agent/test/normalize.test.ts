import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import { join } from "node:path";
import { after, test } from "node:test";

import { AGENT_DEFINITION_FORMAT_VERSION, NX_AGENT_TOOL_CONTEXT, type Agent, type NormalizedAgent } from "../src/index.js";
import { normalizeAgent, type AgentSchemaSource, type NormalizeAgentResult } from "../src/normalize.js";
import { compileProgram, disposeCompiled } from "./nx-support.js";
import { packageRoot } from "./support.js";

after(disposeCompiled);

const fixtures = join(packageRoot, "test", "fixtures");
const fixture = compileProgram(readFileSync(join(fixtures, "agent", "main.nx"), "utf8"));
const fixtureValue = JSON.parse(readFileSync(join(fixtures, "agent-value.json"), "utf8")) as Agent;

function definitionOf(result: NormalizeAgentResult): NormalizedAgent {
  assert.equal(result.ok, true, JSON.stringify(result.diagnostics, null, 2));
  return (result as Extract<NormalizeAgentResult, { ok: true }>).definition;
}

test("the checked-in agent value is what the fixture program evaluates to", () => {
  assert.deepEqual(fixture.agent(), fixtureValue);
});

test("a program artifact is a schema source as it stands", () => {
  // The assignment is the check: it compiles only while the artifact's `functionSchema` has the
  // signature, and answers the fields, that the package reads.
  const source: AgentSchemaSource = fixture.artifact;
  const answer = source.functionSchema({ module: "main.nx", name: "findPlans" }, { hostSuppliedTypes: [NX_AGENT_TOOL_CONTEXT] });
  assert.equal(answer.description, "Finds the plans that fit a team.");
  assert.deepEqual(NX_AGENT_TOOL_CONTEXT, { module: "@nx/agent/agent.nx", name: "ToolContext" });
});

test("an evaluated agent normalizes to a definition", () => {
  const result = normalizeAgent(fixtureValue, { schemas: fixture.artifact });
  const definition = definitionOf(result);
  assert.deepEqual(result.diagnostics, []);
  assert.equal(definition.formatVersion, AGENT_DEFINITION_FORMAT_VERSION);
  assert.equal(definition.formatVersion, 1);
  assert.equal(definition.name, "support");
  assert.equal(definition.description, "Answers customer questions.");
  assert.equal(definition.model, "balanced");
  assert.equal(definition.instructions, "You are the support assistant. Answer from the documents.");
  assert.deepEqual(definition.documents, [{ title: "Refund policy", text: "Refunds are available within 30 days of purchase." }]);
  assert.deepEqual(definition.limits, { maxSteps: 6, maxToolCalls: 12 });
  assert.deepEqual(
    definition.tools.map((tool) => [tool.name, tool.kind]),
    [
      ["find_plans", "function"],
      ["web_search", "provider"],
      ["lookup_order", "http"],
      ["place_order", "http"],
    ],
  );
  assert.deepEqual(Object.keys(definition), ["formatVersion", "name", "description", "model", "instructions", "documents", "limits", "tools"]);
});

test("the query lists ToolContext as host-supplied, by module and name", () => {
  const asked: unknown[] = [];
  const recording: AgentSchemaSource = {
    functionSchema: (reference, options) => {
      asked.push([reference, options]);
      return fixture.artifact.functionSchema(reference, options);
    },
  };
  definitionOf(normalizeAgent(fixtureValue, { schemas: recording }));
  const hostSupplied = { hostSuppliedTypes: [{ module: "@nx/agent/agent.nx", name: "ToolContext" }] };
  assert.deepEqual(asked, [
    [{ module: "main.nx", name: "findPlans" }, hostSupplied],
    [{ module: "main.nx", name: "lookupOrder" }, hostSupplied],
    [{ module: "main.nx", name: "placeOrder" }, hostSupplied],
  ]);
});

test("a value that is not an Agent is refused, naming what it is", () => {
  const cases: readonly (readonly [unknown, RegExp])[] = [
    [{ $type: "Plan", name: "Team" }, /a 'Plan', not an 'Agent'/],
    [{ name: "support", instructions: "x" }, /a record with no '\$type'/],
    [null, /null, not an 'Agent'/],
    ["support", /the string "support"/],
    [[fixtureValue], /a list, not an 'Agent'/],
    [undefined, /undefined/],
  ];
  for (const [value, message] of cases) {
    const result = normalizeAgent(value as Agent, { schemas: fixture.artifact });
    assert.equal(result.ok, false);
    assert.equal("definition" in result, false);
    assert.equal(result.diagnostics.length, 1);
    assert.deepEqual(
      { ...result.diagnostics[0], message: "" },
      { severity: "error", code: "nx-agent-not-an-agent", message: "", path: "" },
    );
    assert.match(result.diagnostics[0]!.message, message);
  }
});

test("duplicate document titles are rejected, naming the title", () => {
  const value = {
    ...fixtureValue,
    documents: [
      { $type: "Document", title: "Refund policy", text: "One." },
      { $type: "Document", title: "Shipping", text: "Two." },
      { $type: "Document", title: "Refund policy", text: "Three." },
    ],
  } as Agent;
  const result = normalizeAgent(value, { schemas: fixture.artifact });
  assert.equal(result.ok, false);
  assert.equal("definition" in result, false);
  assert.deepEqual(result.diagnostics, [
    {
      severity: "error",
      code: "nx-agent-duplicate-document-title",
      message: 'The document title "Refund policy" is used by documents[0] and documents[2]; each title names one document.',
      path: "documents[2].title",
    },
  ]);
});

test("an optional field with no value is absent, and an agent with no tools has an empty list", () => {
  const bare = compileProgram('let root(): Agent = { <Agent name="bare">Be brief.</Agent> }');
  assert.deepEqual(bare.agent(), { $type: "Agent", name: "bare", instructions: "Be brief." });
  const definition = definitionOf(normalizeAgent(bare.agent(), { schemas: bare.artifact }));
  assert.deepEqual(definition, { formatVersion: 1, name: "bare", instructions: "Be brief.", documents: [], limits: {}, tools: [] });
  assert.deepEqual(Object.keys(definition), ["formatVersion", "name", "instructions", "documents", "limits", "tools"]);
  assert.equal(JSON.stringify(definition).includes("null"), false);
  // The other two spellings of an empty optional a host may hand in.
  const spelled = { ...bare.agent(), description: null, model: [], documents: [], tools: null, limits: [] } as unknown as Agent;
  assert.deepEqual(definitionOf(normalizeAgent(spelled, { schemas: bare.artifact })), definition);
});

test("a field of the wrong kind is reported and yields no definition", () => {
  const cases: readonly (readonly [object, string])[] = [
    [{ name: 7 }, "name"],
    [{ instructions: undefined }, "instructions"],
    [{ model: 4 }, "model"],
    [{ documents: "none" }, "documents"],
    [{ documents: ["Refund policy"] }, "documents[0]"],
    [{ documents: [{ $type: "Document", title: "T" }] }, "documents[0].text"],
    [{ tools: { $type: "FunctionTool" } }, "tools"],
    [{ limits: 3 }, "limits"],
    [{ limits: { $type: "AgentLimits", maxSteps: "six" } }, "limits.maxSteps"],
  ];
  for (const [change, path] of cases) {
    const result = normalizeAgent({ ...fixtureValue, ...change } as Agent, { schemas: fixture.artifact });
    assert.equal(result.ok, false, path);
    assert.deepEqual(result.diagnostics.map((diagnostic) => [diagnostic.code, diagnostic.path]), [["nx-agent-not-an-agent", path]], path);
  }
});
