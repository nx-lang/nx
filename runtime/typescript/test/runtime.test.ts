import {
  NxIrProgram,
  NxIrExpression,
  NxIrRecordField,
  NxIrRuntimeError,
  NxIrSemanticType,
  NxIrTypeRef,
  applyComponentStatePatch,
  constructComponentDescriptor,
  evaluateComponent,
  evaluateFunction,
  initializeComponent,
  prepareNxIrProgram,
  tryPrepareNxIrProgram,
} from "../src/index.js";

type TestCase = readonly [string, () => void];

const tests: TestCase[] = [];

function test(name: string, run: () => void): void {
  tests.push([name, run]);
}

function assertEqual(actual: unknown, expected: unknown): void {
  const actualJson = stableJson(actual);
  const expectedJson = stableJson(expected);
  if (actualJson !== expectedJson) {
    throw new Error(`Expected ${expectedJson}, got ${actualJson}`);
  }
}

function stableJson(value: unknown): string {
  if (Array.isArray(value)) {
    return `[${value.map(stableJson).join(",")}]`;
  }
  if (value !== null && typeof value === "object") {
    const entries = Object.entries(value)
      .sort(([left], [right]) => left.localeCompare(right))
      .map(([key, item]) => `${JSON.stringify(key)}:${stableJson(item)}`);
    return `{${entries.join(",")}}`;
  }
  return JSON.stringify(value);
}

function assertThrows(run: () => void, expectedMessage: string): void {
  try {
    run();
  } catch (error) {
    if (!(error instanceof NxIrRuntimeError)) {
      throw error;
    }
    if (!error.diagnostics.some((diagnostic) => diagnostic.message.includes(expectedMessage))) {
      throw new Error(`Expected diagnostic containing '${expectedMessage}', got ${error.message}`);
    }
    return;
  }
  throw new Error("Expected function to throw");
}

const sourceSpan = { source: "test.nx", start: 0, end: 0 };
let expressionCounter = 0;

function ref(name: string, declaration: string, kind: string, module = "m0") {
  return { module, declaration, name, kind };
}

function primitive(name: string): NxIrTypeRef {
  return { kind: "primitive", name };
}

function nominal(reference: ReturnType<typeof ref>, display = reference.name): NxIrTypeRef {
  return { kind: "nominal", reference, display };
}

const stringType: NxIrTypeRef = primitive("string");
const intType: NxIrTypeRef = primitive("int");
const themeType: NxIrTypeRef = nominal(ref("Theme", "m0:d3", "enum"));
const loadStateType: NxIrTypeRef = nominal(ref("LoadState", "m0:d5", "union"));
const intSemantic: NxIrSemanticType = { display: "int", shape: { kind: "primitive", name: "int" } };
const floatSemantic: NxIrSemanticType = { display: "float", shape: { kind: "primitive", name: "float" } };

function expr(
  op: { readonly tag: string; readonly [key: string]: unknown },
  ty?: NxIrSemanticType,
): NxIrExpression {
  expressionCounter += 1;
  const base = {
    id: `e${expressionCounter}`,
    span: sourceSpan,
    op,
  };
  return ty === undefined ? base : { ...base, ty };
}

function lit(value: unknown) {
  if (typeof value === "string") {
    return expr({ tag: "literal", value: { kind: "string", value } });
  }
  if (typeof value === "number") {
    return expr({ tag: "literal", value: { kind: "int", value: String(value), number: value } });
  }
  if (typeof value === "boolean") {
    return expr({ tag: "literal", value: { kind: "boolean", value } });
  }
  return expr({ tag: "literal", value: { kind: "null" } });
}

function floatLit(value: number) {
  return expr({ tag: "literal", value: { kind: "float", value: String(value) } }, floatSemantic);
}

function binary(operator: string, lhs: NxIrExpression, rhs: NxIrExpression, ty?: NxIrSemanticType) {
  return expr({ tag: "binary", operator, lhs, rhs }, ty);
}

function slot(name: string, slotId: string) {
  return expr({ tag: "slot", name, slot: slotId });
}

function reference(name: string, declaration: string, kind: string) {
  return expr({ tag: "reference", reference: ref(name, declaration, kind) });
}

const userRecordFields: NxIrRecordField[] = [
  {
    name: "name",
    slot: "User:field:0",
    ty: stringType,
    isContent: false,
    isRequired: true,
    span: sourceSpan,
  },
  {
    name: "score",
    slot: "User:field:1",
    ty: intType,
    isContent: false,
    isRequired: false,
    default: lit(42),
    span: sourceSpan,
  },
];

const program: NxIrProgram = {
  format: "nx-ir-json",
  schemaVersion: 1,
  runtimeAbi: "nx-ir-runtime-v1",
  programFingerprint: "42",
  requiredFeatures: ["eager-v1"],
  functionEntrypoints: [
    { name: "root", reference: ref("root", "m0:d2", "function") },
    { name: "values", reference: ref("values", "m0:d6", "function") },
    { name: "failed", reference: ref("failed", "m0:d7", "function") },
    { name: "describe", reference: ref("describe", "m0:d8", "function") },
    { name: "echoTheme", reference: ref("echoTheme", "m0:d9", "function") },
    { name: "themeValue", reference: ref("themeValue", "m0:d12", "function") },
    { name: "mappedValues", reference: ref("mappedValues", "m0:d13", "function") },
    { name: "flow", reference: ref("flow", "m0:d14", "function") },
    { name: "letValue", reference: ref("letValue", "m0:d15", "function") },
    { name: "describeOffline", reference: ref("describeOffline", "m0:d16", "function") },
  ],
  componentEntrypoints: [
    { name: "TextInput", reference: ref("TextInput", "m0:d10", "component") },
    { name: "SearchBox", reference: ref("SearchBox", "m0:d11", "component") },
  ],
  sources: [{ identity: "test.nx", source: "let root() = { 42 }" }],
  modules: [
    {
      id: "m0",
      runtimeId: 0,
      provenance: { kind: "sourceProvider", identity: "test.nx" },
      imports: [],
      declarations: [
        {
          id: "m0:d0",
          reference: ref("answer", "m0:d0", "function"),
          span: sourceSpan,
          kind: {
            tag: "function",
            params: [],
            body: lit(41),
          },
        },
        {
          id: "m0:d1",
          reference: ref("add", "m0:d1", "function"),
          span: sourceSpan,
          kind: {
            tag: "function",
            params: [
              { name: "a", slot: "add:param:0", ty: intType, isContent: false, span: sourceSpan },
              { name: "b", slot: "add:param:1", ty: intType, isContent: false, span: sourceSpan },
            ],
            body: expr({
              tag: "binary",
              operator: "add",
              lhs: slot("a", "add:param:0"),
              rhs: slot("b", "add:param:1"),
            }),
          },
        },
        {
          id: "m0:d2",
          reference: ref("root", "m0:d2", "function"),
          span: sourceSpan,
          kind: {
            tag: "function",
            params: [],
            body: expr({
              tag: "call",
              callee: reference("add", "m0:d1", "function"),
              args: [
                expr({ tag: "call", callee: reference("answer", "m0:d0", "function"), args: [] }),
                lit(1),
              ],
            }),
          },
        },
        {
          id: "m0:d3",
          reference: ref("Theme", "m0:d3", "enum"),
          span: sourceSpan,
          kind: { tag: "enum", members: ["light", "dark"] },
        },
        {
          id: "m0:d4",
          reference: ref("User", "m0:d4", "record"),
          span: sourceSpan,
          kind: {
            tag: "record",
            fields: userRecordFields,
          },
        },
        {
          id: "m0:d5",
          reference: ref("LoadState", "m0:d5", "union"),
          span: sourceSpan,
          kind: {
            tag: "union",
            cases: [
              { name: "idle", fields: [], span: sourceSpan },
              {
                name: "failed",
                fields: [
                  {
                    name: "message",
                    slot: "LoadState.failed:field:0",
                    ty: stringType,
                    isContent: false,
                    isRequired: true,
                    span: sourceSpan,
                  },
                ],
                span: sourceSpan,
              },
            ],
          },
        },
        {
          id: "m0:d6",
          reference: ref("values", "m0:d6", "function"),
          span: sourceSpan,
          kind: {
            tag: "function",
            params: [],
            body: expr({
              tag: "record",
              name: "User",
              fields: userRecordFields,
              properties: [{ name: "name", value: lit("Ada"), span: sourceSpan }],
            }),
          },
        },
        {
          id: "m0:d7",
          reference: ref("failed", "m0:d7", "function"),
          span: sourceSpan,
          kind: {
            tag: "function",
            params: [],
            body: expr({
              tag: "unionCase",
              union: ref("LoadState", "m0:d5", "union"),
              caseName: "failed",
              fields: [
                {
                  name: "message",
                  slot: "failed:field:0",
                  ty: stringType,
                  isContent: false,
                  isRequired: true,
                  span: sourceSpan,
                },
              ],
              properties: [{ name: "message", value: lit("offline"), span: sourceSpan }],
              content: [],
            }),
          },
        },
        {
          id: "m0:d8",
          reference: ref("describe", "m0:d8", "function"),
          span: sourceSpan,
          kind: {
            tag: "function",
            params: [
              {
                name: "state",
                slot: "describe:param:0",
                ty: loadStateType,
                isContent: false,
                span: sourceSpan,
              },
            ],
            body: expr({
              tag: "ifIs",
              scrutinee: slot("state", "describe:param:0"),
              arms: [
                {
                  patterns: [
                    expr({
                      tag: "unionCase",
                      union: ref("LoadState", "m0:d5", "union"),
                      caseName: "failed",
                      fields: [],
                      properties: [],
                      content: [],
                    }),
                  ],
                  body: lit("failed"),
                },
              ],
              elseBranch: lit("ok"),
            }),
          },
        },
        {
          id: "m0:d9",
          reference: ref("echoTheme", "m0:d9", "function"),
          span: sourceSpan,
          kind: {
            tag: "function",
            params: [
              { name: "mode", slot: "theme:param:0", ty: themeType, isContent: false, span: sourceSpan },
            ],
            body: slot("mode", "theme:param:0"),
          },
        },
        {
          id: "m0:d10",
          reference: ref("TextInput", "m0:d10", "component"),
          span: sourceSpan,
          kind: {
            tag: "component",
            isAbstract: false,
            isExternal: true,
            props: [
              {
                name: "value",
                slot: "TextInput:prop:0",
                ownerModule: "m0",
                ty: stringType,
                isContent: false,
                isRequired: true,
                span: sourceSpan,
              },
            ],
            state: [],
          },
        },
        {
          id: "m0:d11",
          reference: ref("SearchBox", "m0:d11", "component"),
          span: sourceSpan,
          kind: {
            tag: "component",
            isAbstract: false,
            isExternal: false,
            props: [
              {
                name: "placeholder",
                slot: "SearchBox:prop:0",
                ownerModule: "m0",
                ty: stringType,
                isContent: false,
                isRequired: false,
                default: lit("Find docs"),
                span: sourceSpan,
              },
            ],
            state: [
              {
                name: "query",
                slot: "SearchBox:state:0",
                ownerModule: "m0",
                ty: stringType,
                isContent: false,
                isRequired: false,
                default: slot("placeholder", "SearchBox:prop:0"),
                span: sourceSpan,
              },
            ],
            body: expr({
              tag: "componentDescriptor",
              component: ref("TextInput", "m0:d10", "component"),
              targetKind: "external",
              properties: [{ name: "value", value: slot("query", "SearchBox:state:0"), span: sourceSpan }],
              content: [],
            }),
          },
        },
        {
          id: "m0:d12",
          reference: ref("themeValue", "m0:d12", "function"),
          span: sourceSpan,
          kind: {
            tag: "function",
            params: [],
            body: expr({
              tag: "enumMember",
              enum: ref("Theme", "m0:d3", "enum"),
              member: "dark",
            }),
          },
        },
        {
          id: "m0:d13",
          reference: ref("mappedValues", "m0:d13", "function"),
          span: sourceSpan,
          kind: {
            tag: "function",
            params: [],
            body: expr({
              tag: "for",
              itemName: "item",
              itemSlot: "mappedValues:item",
              indexName: "index",
              indexSlot: "mappedValues:index",
              iterable: expr({ tag: "array", elements: [lit(10), lit(20)] }),
              body: expr({
                tag: "binary",
                operator: "add",
                lhs: slot("item", "mappedValues:item"),
                rhs: slot("index", "mappedValues:index"),
              }),
            }),
          },
        },
        {
          id: "m0:d14",
          reference: ref("flow", "m0:d14", "function"),
          span: sourceSpan,
          kind: {
            tag: "function",
            params: [],
            body: expr({
              tag: "block",
              statements: [
                {
                  tag: "let",
                  name: "user",
                  slot: "flow:user",
                  init: expr({
                    tag: "record",
                    name: "User",
                    fields: userRecordFields,
                    properties: [{ name: "name", value: lit("Grace"), span: sourceSpan }],
                  }),
                },
                {
                  tag: "let",
                  name: "items",
                  slot: "flow:items",
                  init: expr({ tag: "array", elements: [lit(40), lit(42)] }),
                },
              ],
              expression: expr({
                tag: "if",
                condition: expr({
                  tag: "binary",
                  operator: "and",
                  lhs: expr({ tag: "unary", operator: "not", expr: lit(false) }),
                  rhs: expr({
                    tag: "binary",
                    operator: "eq",
                    lhs: expr({
                      tag: "member",
                      base: slot("user", "flow:user"),
                      member: "score",
                    }),
                    rhs: lit(42),
                  }),
                }),
                thenBranch: expr({
                  tag: "index",
                  base: slot("items", "flow:items"),
                  index: lit(1),
                }),
                elseBranch: lit(0),
              }),
            }),
          },
        },
        {
          id: "m0:d15",
          reference: ref("letValue", "m0:d15", "function"),
          span: sourceSpan,
          kind: {
            tag: "function",
            params: [],
            body: expr({
              tag: "let",
              name: "amount",
              slot: "letValue:amount",
              value: lit(12),
              body: expr({
                tag: "unary",
                operator: "neg",
                expr: slot("amount", "letValue:amount"),
              }),
            }),
          },
        },
        {
          id: "m0:d16",
          reference: ref("describeOffline", "m0:d16", "function"),
          span: sourceSpan,
          kind: {
            tag: "function",
            params: [
              {
                name: "state",
                slot: "describeOffline:param:0",
                ty: loadStateType,
                isContent: false,
                span: sourceSpan,
              },
            ],
            body: expr({
              tag: "ifIs",
              scrutinee: slot("state", "describeOffline:param:0"),
              arms: [
                {
                  patterns: [
                    expr({
                      tag: "unionCase",
                      union: ref("LoadState", "m0:d5", "union"),
                      caseName: "failed",
                      fields: [
                        {
                          name: "message",
                          slot: "LoadState.failed:field:0",
                          ty: stringType,
                          isContent: false,
                          isRequired: true,
                          span: sourceSpan,
                        },
                      ],
                      properties: [{ name: "message", value: lit("offline"), span: sourceSpan }],
                      content: [],
                    }),
                  ],
                  body: lit("failed-type"),
                },
              ],
              elseBranch: lit("other"),
            }),
          },
        },
      ],
    },
  ],
};

test("prepares supported IR and rejects unsupported feature flags", () => {
  const prepared = prepareNxIrProgram(program);
  assertEqual(prepared.functionEntrypoints.has("root"), true);

  const rejected = tryPrepareNxIrProgram({
    ...program,
    requiredFeatures: ["future-reactivity"],
  });
  assertEqual(rejected.ok, false);
});

test("rejects unknown operation tags during preparation", () => {
  const badProgram: NxIrProgram = {
    ...program,
    modules: [
      {
        ...program.modules[0]!,
        declarations: [
          {
            id: "m0:bad",
            reference: ref("bad", "m0:bad", "function"),
            span: sourceSpan,
            kind: {
              tag: "function",
              params: [],
              body: expr({ tag: "teleport" }),
            },
          },
        ],
      },
    ],
    functionEntrypoints: [{ name: "bad", reference: ref("bad", "m0:bad", "function") }],
    componentEntrypoints: [],
  };

  const rejected = tryPrepareNxIrProgram(badProgram);
  assertEqual(rejected.ok, false);
});

test("uses module-qualified nominal type references and entrypoint-only lookup", () => {
  const rootUserRef = ref("User", "m0:d0", "record", "m0");
  const libraryUserRef = ref("User", "m1:d0", "record", "m1");
  const rootUserFields: NxIrRecordField[] = [
    {
      name: "name",
      slot: "m0:User:field:0",
      ty: stringType,
      isContent: false,
      isRequired: true,
      span: sourceSpan,
    },
  ];
  const libraryUserFields: NxIrRecordField[] = [
    {
      name: "score",
      slot: "m1:User:field:0",
      ty: intType,
      isContent: false,
      isRequired: true,
      span: sourceSpan,
    },
  ];
  const collisionProgram: NxIrProgram = {
    format: "nx-ir-json",
    schemaVersion: 1,
    runtimeAbi: "nx-ir-runtime-v1",
    programFingerprint: "43",
    requiredFeatures: ["eager-v1"],
    functionEntrypoints: [{ name: "acceptUser", reference: ref("acceptUser", "m0:d1", "function", "m0") }],
    componentEntrypoints: [],
    sources: [{ identity: "collision.nx", source: "" }],
    modules: [
      {
        id: "m0",
        runtimeId: 0,
        provenance: { kind: "sourceProvider", identity: "app/main.nx" },
        imports: [],
        declarations: [
          {
            id: "m0:d0",
            reference: rootUserRef,
            span: sourceSpan,
            kind: { tag: "record", fields: rootUserFields },
          },
          {
            id: "m0:d1",
            reference: ref("acceptUser", "m0:d1", "function", "m0"),
            span: sourceSpan,
            kind: {
              tag: "function",
              params: [
                {
                  name: "user",
                  slot: "acceptUser:param:0",
                  ty: nominal(rootUserRef),
                  isContent: false,
                  span: sourceSpan,
                },
              ],
              body: slot("user", "acceptUser:param:0"),
            },
          },
          {
            id: "m0:d2",
            reference: ref("helper", "m0:d2", "function", "m0"),
            span: sourceSpan,
            kind: { tag: "function", params: [], body: lit(1) },
          },
        ],
      },
      {
        id: "m1",
        runtimeId: 1,
        provenance: { kind: "sourceProvider", identity: "lib/model.nx" },
        imports: [],
        declarations: [
          {
            id: "m1:d0",
            reference: libraryUserRef,
            span: sourceSpan,
            kind: { tag: "record", fields: libraryUserFields },
          },
        ],
      },
    ],
  };
  const prepared = prepareNxIrProgram(collisionProgram);

  assertEqual(evaluateFunction(prepared, "acceptUser", [{ name: "Ada" }]), { $type: "User", name: "Ada" });
  assertThrows(() => evaluateFunction(prepared, "helper"), "Function entrypoint 'helper' was not found");
});

test("evaluates function calls, records, union cases, enums, loops, and match expressions", () => {
  const prepared = prepareNxIrProgram(program);

  assertEqual(evaluateFunction(prepared, "root"), 42);
  assertEqual(evaluateFunction(prepared, "values"), { $type: "User", name: "Ada", score: 42 });
  assertEqual(evaluateFunction(prepared, "failed"), {
    $type: "LoadState.failed",
    message: "offline",
  });
  assertEqual(evaluateFunction(prepared, "describe", [{ $type: "LoadState.failed", message: "x" }]), "failed");
  assertEqual(evaluateFunction(prepared, "describe", [{ $type: "LoadState.idle" }]), "ok");
  assertEqual(
    evaluateFunction(prepared, "describeOffline", [{ $type: "LoadState.failed", message: "online" }]),
    "failed-type",
  );
  assertEqual(evaluateFunction(prepared, "echoTheme", ["dark"]), "dark");
  assertEqual(evaluateFunction(prepared, "themeValue"), "dark");
  assertEqual(evaluateFunction(prepared, "mappedValues"), [10, 21]);
  assertEqual(evaluateFunction(prepared, "flow"), 42);
  assertEqual(evaluateFunction(prepared, "letValue"), -12);
  assertThrows(() => evaluateFunction(prepared, "echoTheme", ["blue"]), "Invalid enum member");
});

test("matches native numeric division and modulo semantics", () => {
  const module = program.modules[0]!;
  const arithmeticProgram: NxIrProgram = {
    ...program,
    functionEntrypoints: [
      ...program.functionEntrypoints,
      { name: "numericValues", reference: ref("numericValues", "m0:d17", "function") },
      { name: "divideByZero", reference: ref("divideByZero", "m0:d18", "function") },
      { name: "moduloByZero", reference: ref("moduloByZero", "m0:d19", "function") },
    ],
    modules: [
      {
        ...module,
        declarations: [
          ...module.declarations,
          {
            id: "m0:d17",
            reference: ref("numericValues", "m0:d17", "function"),
            span: sourceSpan,
            kind: {
              tag: "function",
              params: [],
              body: expr({
                tag: "array",
                elements: [
                  binary("div", lit(7), lit(2), intSemantic),
                  binary("div", lit(-7), lit(2), intSemantic),
                  binary("mod", lit(7), lit(2), intSemantic),
                  binary("mod", lit(-7), lit(2), intSemantic),
                  binary("div", floatLit(7), floatLit(2), floatSemantic),
                ],
              }),
            },
          },
          {
            id: "m0:d18",
            reference: ref("divideByZero", "m0:d18", "function"),
            span: sourceSpan,
            kind: {
              tag: "function",
              params: [],
              body: binary("div", lit(1), lit(0), intSemantic),
            },
          },
          {
            id: "m0:d19",
            reference: ref("moduloByZero", "m0:d19", "function"),
            span: sourceSpan,
            kind: {
              tag: "function",
              params: [],
              body: binary("mod", lit(1), lit(0), intSemantic),
            },
          },
        ],
      },
    ],
  };
  const prepared = prepareNxIrProgram(arithmeticProgram);

  assertEqual(evaluateFunction(prepared, "numericValues"), [3, -3, 1, -1, 3.5]);
  assertThrows(() => evaluateFunction(prepared, "divideByZero"), "Division by zero");
  assertThrows(() => evaluateFunction(prepared, "moduloByZero"), "Division by zero");
});

test("rejects out-of-bounds array indexes", () => {
  const module = program.modules[0]!;
  const outOfBoundsProgram: NxIrProgram = {
    ...program,
    functionEntrypoints: [
      ...program.functionEntrypoints,
      { name: "outOfBounds", reference: ref("outOfBounds", "m0:d17", "function") },
    ],
    modules: [
      {
        ...module,
        declarations: [
          ...module.declarations,
          {
            id: "m0:d17",
            reference: ref("outOfBounds", "m0:d17", "function"),
            span: sourceSpan,
            kind: {
              tag: "function",
              params: [],
              body: expr({
                tag: "index",
                base: expr({ tag: "array", elements: [lit(1), lit(2)] }),
                index: lit(2),
              }),
            },
          },
        ],
      },
    ],
  };
  const prepared = prepareNxIrProgram(outOfBoundsProgram);

  assertThrows(
    () => evaluateFunction(prepared, "outOfBounds"),
    "Array index 2 is out of bounds for length 2",
  );
});

test("constructs descriptors and evaluates components with host-owned state", () => {
  const prepared = prepareNxIrProgram(program);

  assertEqual(constructComponentDescriptor(prepared, "SearchBox"), {
    $type: "SearchBox",
    placeholder: "Find docs",
  });
  assertEqual(initializeComponent(prepared, "SearchBox"), {
    state: { query: "Find docs" },
    rendered: { $type: "TextInput", value: "Find docs" },
  });
  assertEqual(evaluateComponent(prepared, "SearchBox", {}, { query: "docs" }), {
    rendered: { $type: "TextInput", value: "docs" },
  });
  assertEqual(applyComponentStatePatch(prepared, "SearchBox", { query: "docs" }, { query: "guides" }), {
    query: "guides",
  });
  assertThrows(
    () => applyComponentStatePatch(prepared, "SearchBox", { query: "docs" }, { query: 123 }),
    "Expected SearchBox state.query to be a string",
  );
  assertThrows(() => constructComponentDescriptor(prepared, "TextInput"), "Missing required");
});

for (const [name, run] of tests) {
  run();
  console.log(`ok - ${name}`);
}
