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
const themeType: NxIrTypeRef = nominal(ref("Theme", "m0:d3", "union"));
const loadStateType: NxIrTypeRef = nominal(ref("LoadState", "m0:d5", "union"));
const nullableLoadStateType: NxIrTypeRef = { kind: "nullable", inner: loadStateType };
const intSemantic: NxIrSemanticType = { display: "int", shape: { kind: "primitive", name: "int" } };
const floatSemantic: NxIrSemanticType = { display: "float64", shape: { kind: "primitive", name: "float64" } };

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
  schemaVersion: 2,
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
          reference: ref("Theme", "m0:d3", "union"),
          span: sourceSpan,
          kind: {
            tag: "union",
            cases: [
              { name: "light", fields: [], isConstant: true, span: sourceSpan },
              { name: "dark", fields: [], isConstant: true, span: sourceSpan },
            ],
          },
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
              { name: "idle", fields: [], isConstant: true, span: sourceSpan },
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
                isConstant: false,
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
              tag: "unionCase",
              union: ref("Theme", "m0:d3", "union"),
              caseName: "dark",
              fields: [],
              properties: [],
              contentField: null,
              content: [],
              isConstant: true,
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
    schemaVersion: 2,
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

test("evaluates function calls, records, union cases, loops, and match expressions", () => {
  const prepared = prepareNxIrProgram(program);

  assertEqual(evaluateFunction(prepared, "root"), 42);
  assertEqual(evaluateFunction(prepared, "values"), { $type: "User", name: "Ada", score: 42 });
  assertEqual(evaluateFunction(prepared, "failed"), {
    $type: "LoadState.failed",
    message: "offline",
  });
  assertEqual(evaluateFunction(prepared, "describe", [{ $type: "LoadState.failed", message: "x" }]), "failed");
  assertEqual(evaluateFunction(prepared, "describe", ["idle"]), "ok");
  assertEqual(
    evaluateFunction(prepared, "describeOffline", [{ $type: "LoadState.failed", message: "online" }]),
    "failed-type",
  );
  assertEqual(evaluateFunction(prepared, "echoTheme", ["dark"]), "dark");
  assertEqual(evaluateFunction(prepared, "themeValue"), "dark");
  assertEqual(evaluateFunction(prepared, "mappedValues"), [10, 21]);
  assertEqual(evaluateFunction(prepared, "flow"), 42);
  assertEqual(evaluateFunction(prepared, "letValue"), -12);
  assertThrows(
    () => evaluateFunction(prepared, "echoTheme", ["blue"]),
    "Invalid constant union case",
  );
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

test("normalizes nullable union null and rejects undeclared union cases", () => {
  const module = program.modules[0]!;
  const nullableProgram: NxIrProgram = {
    ...program,
    functionEntrypoints: [
      ...program.functionEntrypoints,
      { name: "optionalState", reference: ref("optionalState", "m0:d17", "function") },
    ],
    modules: [
      {
        ...module,
        declarations: [
          ...module.declarations,
          {
            id: "m0:d17",
            reference: ref("optionalState", "m0:d17", "function"),
            span: sourceSpan,
            kind: {
              tag: "function",
              params: [
                {
                  name: "state",
                  slot: "optionalState:param:0",
                  ty: nullableLoadStateType,
                  isContent: false,
                  span: sourceSpan,
                },
              ],
              body: slot("state", "optionalState:param:0"),
            },
          },
        ],
      },
    ],
  };
  const prepared = prepareNxIrProgram(nullableProgram);

  assertEqual(evaluateFunction(prepared, "optionalState", [null]), null);
  assertThrows(
    () => evaluateFunction(prepared, "optionalState", [{ $type: "LoadState.undefined" }]),
    "Invalid union case",
  );
});

test("applies content bindings before required field validation", () => {
  const module = program.modules[0]!;
  const bodyField: NxIrRecordField = {
    name: "body",
    slot: "ContentBox:field:0",
    ty: stringType,
    isContent: true,
    isRequired: true,
    span: sourceSpan,
  };
  const contentProgram: NxIrProgram = {
    ...program,
    functionEntrypoints: [
      ...program.functionEntrypoints,
      { name: "recordContent", reference: ref("recordContent", "m0:d17", "function") },
      { name: "componentContent", reference: ref("componentContent", "m0:d19", "function") },
    ],
    componentEntrypoints: [
      ...program.componentEntrypoints,
      { name: "Panel", reference: ref("Panel", "m0:d18", "component") },
    ],
    modules: [
      {
        ...module,
        declarations: [
          ...module.declarations,
          {
            id: "m0:d17",
            reference: ref("recordContent", "m0:d17", "function"),
            span: sourceSpan,
            kind: {
              tag: "function",
              params: [],
              body: expr({
                tag: "record",
                name: "ContentBox",
                fields: [bodyField],
                properties: [],
                contentField: "body",
                content: [lit("hello")],
              }),
            },
          },
          {
            id: "m0:d18",
            reference: ref("Panel", "m0:d18", "component"),
            span: sourceSpan,
            kind: {
              tag: "component",
              isAbstract: false,
              isExternal: true,
              props: [{ ...bodyField, slot: "Panel:prop:0", ownerModule: "m0" }],
              state: [],
            },
          },
          {
            id: "m0:d19",
            reference: ref("componentContent", "m0:d19", "function"),
            span: sourceSpan,
            kind: {
              tag: "function",
              params: [],
              body: expr({
                tag: "componentDescriptor",
                component: ref("Panel", "m0:d18", "component"),
                targetKind: "external",
                properties: [],
                contentField: "body",
                content: [lit("hello")],
              }),
            },
          },
        ],
      },
    ],
  };
  const prepared = prepareNxIrProgram(contentProgram);

  assertEqual(evaluateFunction(prepared, "recordContent"), { $type: "ContentBox", body: "hello" });
  assertEqual(evaluateFunction(prepared, "componentContent"), { $type: "Panel", body: "hello" });
  assertThrows(() => constructComponentDescriptor(prepared, "Panel"), "Missing required");
  assertThrows(() => constructComponentDescriptor(prepared, "Panel", { extra: "nope" }), "Unknown Panel props field");
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

test("coerces a single value at a list-typed field into a list of one", () => {
  const module = program.modules[0]!;
  const listField: NxIrRecordField = {
    name: "sizes",
    slot: "Sizes:prop:0",
    ty: { kind: "nullable", inner: { kind: "array", element: intType } },
    isContent: false,
    isRequired: false,
    span: sourceSpan,
  };
  const coercionProgram: NxIrProgram = {
    ...program,
    functionEntrypoints: [
      ...program.functionEntrypoints,
      { name: "oneSize", reference: ref("oneSize", "m0:d32", "function") },
    ],
    componentEntrypoints: [
      ...program.componentEntrypoints,
      { name: "Sizes", reference: ref("Sizes", "m0:d31", "component") },
    ],
    modules: [
      {
        ...module,
        declarations: [
          ...module.declarations,
          {
            id: "m0:d31",
            reference: ref("Sizes", "m0:d31", "component"),
            span: sourceSpan,
            kind: {
              tag: "component",
              isAbstract: false,
              isExternal: true,
              props: [{ ...listField, ownerModule: "m0" }],
              state: [],
            },
          },
          {
            id: "m0:d32",
            reference: ref("oneSize", "m0:d32", "function"),
            span: sourceSpan,
            kind: {
              tag: "function",
              params: [],
              body: expr({
                tag: "componentDescriptor",
                component: ref("Sizes", "m0:d31", "component"),
                targetKind: "external",
                properties: [{ name: "sizes", value: lit(3), span: sourceSpan }],
                contentField: null,
                content: [],
              }),
            },
          },
        ],
      },
    ],
  };
  const prepared = prepareNxIrProgram(coercionProgram);

  assertEqual(evaluateFunction(prepared, "oneSize"), { $type: "Sizes", sizes: [3] });
  assertEqual(constructComponentDescriptor(prepared, "Sizes", { sizes: 3 }), { $type: "Sizes", sizes: [3] });
  assertEqual(constructComponentDescriptor(prepared, "Sizes", { sizes: [3, 4] }), {
    $type: "Sizes",
    sizes: [3, 4],
  });
  assertEqual(constructComponentDescriptor(prepared, "Sizes", {}), { $type: "Sizes", sizes: null });
});

test("normalizes a constructed record into a record-typed field", () => {
  const module = program.modules[0]!;
  const userType: NxIrTypeRef = nominal(ref("User", "m0:d4", "record"));
  const ownerField: NxIrRecordField = {
    name: "owner",
    slot: "Card:prop:0",
    ty: userType,
    isContent: false,
    isRequired: true,
    span: sourceSpan,
  };
  const constructUser = expr({
    tag: "record",
    name: "User",
    fields: userRecordFields,
    properties: [{ name: "name", value: lit("Ada"), span: sourceSpan }],
  });

  const recordProgram: NxIrProgram = {
    ...program,
    functionEntrypoints: [
      ...program.functionEntrypoints,
      { name: "cardWithOwner", reference: ref("cardWithOwner", "m0:d29", "function") },
      { name: "nestedOwner", reference: ref("nestedOwner", "m0:d30", "function") },
    ],
    componentEntrypoints: [
      ...program.componentEntrypoints,
      { name: "Card", reference: ref("Card", "m0:d28", "component") },
    ],
    modules: [
      {
        ...module,
        declarations: [
          ...module.declarations,
          {
            id: "m0:d28",
            reference: ref("Card", "m0:d28", "component"),
            span: sourceSpan,
            kind: {
              tag: "component",
              isAbstract: false,
              isExternal: true,
              props: [{ ...ownerField, ownerModule: "m0" }],
              state: [],
            },
          },
          {
            id: "m0:d29",
            reference: ref("cardWithOwner", "m0:d29", "function"),
            span: sourceSpan,
            kind: {
              tag: "function",
              params: [],
              body: expr({
                tag: "componentDescriptor",
                component: ref("Card", "m0:d28", "component"),
                targetKind: "external",
                properties: [{ name: "owner", value: constructUser, span: sourceSpan }],
                contentField: null,
                content: [],
              }),
            },
          },
          {
            id: "m0:d30",
            reference: ref("nestedOwner", "m0:d30", "function"),
            span: sourceSpan,
            kind: {
              tag: "function",
              params: [],
              body: expr({
                tag: "record",
                name: "Wrapper",
                fields: [{ ...ownerField, name: "owner", slot: "Wrapper:field:0" }],
                properties: [{ name: "owner", value: constructUser, span: sourceSpan }],
              }),
            },
          },
        ],
      },
    ],
  };
  const prepared = prepareNxIrProgram(recordProgram);
  const owner = { $type: "User", name: "Ada", score: 42 };

  assertEqual(evaluateFunction(prepared, "cardWithOwner"), { $type: "Card", owner });
  assertEqual(evaluateFunction(prepared, "nestedOwner"), { $type: "Wrapper", owner });
  assertEqual(constructComponentDescriptor(prepared, "Card", { owner }), { $type: "Card", owner });

  // A host writing a plain object has no discriminator to give, which is not an error.
  assertEqual(constructComponentDescriptor(prepared, "Card", { owner: { name: "Ada", score: 42 } }), {
    $type: "Card",
    owner,
  });

  // But one it does give has to be its own. Restamping a foreign discriminator with the declared
  // name would hand the program back a value reported as the type it is not.
  assertThrows(
    () =>
      constructComponentDescriptor(prepared, "Card", {
        owner: { $type: "Ghost", name: "Ada", score: 42 },
      }),
    "Expected Card props.owner to be a User, got 'Ghost'",
  );
});

test("accepts a derived record at a base-typed field and rejects an unrelated one", () => {
  const module = program.modules[0]!;
  const baseField: NxIrRecordField = {
    name: "name",
    slot: "Base:field:0",
    ty: stringType,
    isContent: false,
    isRequired: true,
    span: sourceSpan,
  };
  // A derived record's fields arrive flattened, base's first, the way the builder emits them.
  const derivedFields: NxIrRecordField[] = [
    { ...baseField, slot: "Derived:field:0" },
    {
      name: "role",
      slot: "Derived:field:1",
      ty: stringType,
      isContent: false,
      isRequired: true,
      span: sourceSpan,
    },
  ];
  const recordDeclaration = (
    id: string,
    name: string,
    fields: NxIrRecordField[],
    bases: ReturnType<typeof ref>[],
  ) => ({
    id,
    reference: ref(name, id, "record"),
    span: sourceSpan,
    kind: { tag: "record" as const, fields, bases },
  });

  const subtypeProgram: NxIrProgram = {
    ...program,
    componentEntrypoints: [
      ...program.componentEntrypoints,
      { name: "Holder", reference: ref("Holder", "m0:d43", "component") },
    ],
    modules: [
      {
        ...module,
        declarations: [
          ...module.declarations,
          recordDeclaration("m0:d40", "Base", [baseField], []),
          recordDeclaration("m0:d41", "Derived", derivedFields, [ref("Base", "m0:d40", "record")]),
          recordDeclaration("m0:d42", "Unrelated", [baseField], []),
          {
            id: "m0:d43",
            reference: ref("Holder", "m0:d43", "component"),
            span: sourceSpan,
            kind: {
              tag: "component",
              isAbstract: false,
              isExternal: true,
              props: [
                {
                  name: "held",
                  slot: "Holder:prop:0",
                  ty: nominal(ref("Base", "m0:d40", "record")),
                  isContent: false,
                  isRequired: true,
                  ownerModule: "m0",
                  span: sourceSpan,
                },
              ],
              state: [],
            },
          },
        ],
      },
    ],
  };
  const prepared = prepareNxIrProgram(subtypeProgram);

  // A derived value is not the wrong type. It keeps its own discriminator and its own fields,
  // which the base's field list does not have room for.
  const derived = { $type: "Derived", name: "Ada", role: "admin" };
  assertEqual(constructComponentDescriptor(prepared, "Holder", { held: derived }), {
    $type: "Holder",
    held: derived,
  });

  // A record that does not extend the base is still the wrong type, even where its fields happen
  // to fit.
  assertThrows(
    () =>
      constructComponentDescriptor(prepared, "Holder", {
        held: { $type: "Unrelated", name: "Ada" },
      }),
    "Expected Holder props.held to be a Base, got 'Unrelated'",
  );
});

test("reports a subtype whose name two declarations share rather than guessing", () => {
  const module = program.modules[0]!;
  const nameField: NxIrRecordField = {
    name: "name",
    slot: "Base:field:0",
    ty: stringType,
    isContent: false,
    isRequired: true,
    span: sourceSpan,
  };
  const base = ref("Base", "m0:d50", "record");
  // Two modules may each declare a `Derived` extending the same base. A value carries only its
  // name, so nothing in it says which declaration to normalize against.
  const twinProgram: NxIrProgram = {
    ...program,
    componentEntrypoints: [
      ...program.componentEntrypoints,
      { name: "Twin", reference: ref("Twin", "m0:d53", "component") },
    ],
    modules: [
      {
        ...module,
        declarations: [
          ...module.declarations,
          {
            id: "m0:d50",
            reference: base,
            span: sourceSpan,
            kind: { tag: "record", fields: [nameField], bases: [] },
          },
          {
            id: "m0:d51",
            reference: ref("Derived", "m0:d51", "record"),
            span: sourceSpan,
            kind: { tag: "record", fields: [nameField], bases: [base] },
          },
          {
            id: "m0:d52",
            reference: ref("Derived", "m0:d52", "record"),
            span: sourceSpan,
            kind: { tag: "record", fields: [nameField], bases: [base] },
          },
          {
            id: "m0:d53",
            reference: ref("Twin", "m0:d53", "component"),
            span: sourceSpan,
            kind: {
              tag: "component",
              isAbstract: false,
              isExternal: true,
              props: [
                {
                  name: "held",
                  slot: "Twin:prop:0",
                  ty: nominal(base),
                  isContent: false,
                  isRequired: true,
                  ownerModule: "m0",
                  span: sourceSpan,
                },
              ],
              state: [],
            },
          },
        ],
      },
    ],
  };
  const prepared = prepareNxIrProgram(twinProgram);

  assertThrows(
    () =>
      constructComponentDescriptor(prepared, "Twin", {
        held: { $type: "Derived", name: "Ada" },
      }),
    "Ambiguous subtype at Twin props.held: 2 declarations named 'Derived' extend Base",
  );
});

test("rejects an abstract record at a base-typed field and accepts one extending it", () => {
  const module = program.modules[0]!;
  const nameField: NxIrRecordField = {
    name: "name",
    slot: "Base:field:0",
    ty: stringType,
    isContent: false,
    isRequired: true,
    span: sourceSpan,
  };
  const base = ref("Base", "m0:d60", "record");
  const middle = ref("Middle", "m0:d61", "record");
  const abstractProgram: NxIrProgram = {
    ...program,
    componentEntrypoints: [
      ...program.componentEntrypoints,
      { name: "Slot", reference: ref("Slot", "m0:d63", "component") },
    ],
    modules: [
      {
        ...module,
        declarations: [
          ...module.declarations,
          {
            id: "m0:d60",
            reference: base,
            span: sourceSpan,
            kind: { tag: "record", fields: [nameField], bases: [], isAbstract: true },
          },
          {
            id: "m0:d61",
            reference: middle,
            span: sourceSpan,
            kind: { tag: "record", fields: [nameField], bases: [base], isAbstract: true },
          },
          {
            id: "m0:d62",
            reference: ref("Derived", "m0:d62", "record"),
            span: sourceSpan,
            kind: {
              tag: "record",
              fields: [nameField],
              bases: [middle, base],
              isAbstract: false,
            },
          },
          {
            id: "m0:d63",
            reference: ref("Slot", "m0:d63", "component"),
            span: sourceSpan,
            kind: {
              tag: "component",
              isAbstract: false,
              isExternal: true,
              props: [
                {
                  name: "held",
                  slot: "Slot:prop:0",
                  ty: nominal(base),
                  isContent: false,
                  isRequired: true,
                  ownerModule: "m0",
                  span: sourceSpan,
                },
              ],
              state: [],
            },
          },
        ],
      },
    ],
  };
  const prepared = prepareNxIrProgram(abstractProgram);

  // A plain host object would otherwise be stamped with the abstract type's own name, producing a
  // value NX itself refuses to construct.
  assertThrows(
    () => constructComponentDescriptor(prepared, "Slot", { held: { name: "Ada" } }),
    "Expected Slot props.held to be a concrete type extending Base, got an object with no '$type' discriminator naming one.",
  );
  assertThrows(
    () => constructComponentDescriptor(prepared, "Slot", { held: { $type: "Base", name: "Ada" } }),
    "Expected Slot props.held to be a concrete type extending Base, got abstract 'Base'.",
  );

  // An intermediate abstract record passes the base check and is still a type with no values.
  assertThrows(
    () => constructComponentDescriptor(prepared, "Slot", { held: { $type: "Middle", name: "Ada" } }),
    "Expected Slot props.held to be a concrete type extending Base, got abstract 'Middle'.",
  );

  const derived = { $type: "Derived", name: "Ada" };
  assertEqual(constructComponentDescriptor(prepared, "Slot", { held: derived }), {
    $type: "Slot",
    held: derived,
  });
});

test("binds list-typed content as a list whatever the child count", () => {
  const module = program.modules[0]!;
  const listType: NxIrTypeRef = { kind: "array", element: stringType };
  const listBody: NxIrRecordField = {
    name: "body",
    slot: "Stack:prop:0",
    ty: listType,
    isContent: true,
    isRequired: true,
    span: sourceSpan,
  };
  const optionalListBody: NxIrRecordField = {
    name: "body",
    slot: "OptionalStack:prop:0",
    ty: { kind: "nullable", inner: listType },
    isContent: true,
    isRequired: false,
    span: sourceSpan,
  };
  const scalarBody: NxIrRecordField = {
    name: "body",
    slot: "Caption:prop:0",
    ty: stringType,
    isContent: true,
    isRequired: true,
    span: sourceSpan,
  };

  function externalComponent(id: string, name: string, field: NxIrRecordField) {
    return {
      id,
      reference: ref(name, id, "component"),
      span: sourceSpan,
      kind: {
        tag: "component" as const,
        isAbstract: false,
        isExternal: true,
        props: [{ ...field, ownerModule: "m0" }],
        state: [],
      },
    };
  }

  function descriptorFunction(id: string, name: string, component: string, componentId: string, children: string[]) {
    return {
      id,
      reference: ref(name, id, "function"),
      span: sourceSpan,
      kind: {
        tag: "function" as const,
        params: [],
        body: expr({
          tag: "componentDescriptor",
          component: ref(component, componentId, "component"),
          targetKind: "external",
          properties: [],
          contentField: "body",
          content: children.map((child) => lit(child)),
        }),
      },
    };
  }

  const listProgram: NxIrProgram = {
    ...program,
    functionEntrypoints: [
      ...program.functionEntrypoints,
      { name: "oneChild", reference: ref("oneChild", "m0:d23", "function") },
      { name: "manyChildren", reference: ref("manyChildren", "m0:d24", "function") },
      { name: "optionalOneChild", reference: ref("optionalOneChild", "m0:d25", "function") },
      { name: "scalarOneChild", reference: ref("scalarOneChild", "m0:d26", "function") },
      { name: "recordOneChild", reference: ref("recordOneChild", "m0:d27", "function") },
    ],
    componentEntrypoints: [
      ...program.componentEntrypoints,
      { name: "Stack", reference: ref("Stack", "m0:d20", "component") },
    ],
    modules: [
      {
        ...module,
        declarations: [
          ...module.declarations,
          externalComponent("m0:d20", "Stack", listBody),
          externalComponent("m0:d21", "OptionalStack", optionalListBody),
          externalComponent("m0:d22", "Caption", scalarBody),
          descriptorFunction("m0:d23", "oneChild", "Stack", "m0:d20", ["only"]),
          descriptorFunction("m0:d24", "manyChildren", "Stack", "m0:d20", ["first", "second"]),
          descriptorFunction("m0:d25", "optionalOneChild", "OptionalStack", "m0:d21", ["only"]),
          descriptorFunction("m0:d26", "scalarOneChild", "Caption", "m0:d22", ["only"]),
          {
            id: "m0:d27",
            reference: ref("recordOneChild", "m0:d27", "function"),
            span: sourceSpan,
            kind: {
              tag: "function",
              params: [],
              body: expr({
                tag: "record",
                name: "StackRecord",
                fields: [{ ...listBody, slot: "StackRecord:field:0" }],
                properties: [],
                contentField: "body",
                content: [lit("only")],
              }),
            },
          },
        ],
      },
    ],
  };
  const prepared = prepareNxIrProgram(listProgram);

  assertEqual(evaluateFunction(prepared, "oneChild"), { $type: "Stack", body: ["only"] });
  assertEqual(evaluateFunction(prepared, "manyChildren"), { $type: "Stack", body: ["first", "second"] });
  assertEqual(evaluateFunction(prepared, "optionalOneChild"), { $type: "OptionalStack", body: ["only"] });
  assertEqual(evaluateFunction(prepared, "scalarOneChild"), { $type: "Caption", body: "only" });
  assertEqual(evaluateFunction(prepared, "recordOneChild"), { $type: "StackRecord", body: ["only"] });
  assertEqual(constructComponentDescriptor(prepared, "Stack", {}, ["only"]), {
    $type: "Stack",
    body: ["only"],
  });
  assertEqual(constructComponentDescriptor(prepared, "Stack", {}, ["first", "second"]), {
    $type: "Stack",
    body: ["first", "second"],
  });
});

for (const [name, run] of tests) {
  run();
  console.log(`ok - ${name}`);
}
