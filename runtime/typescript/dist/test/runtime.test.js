/**
 * The runtime's own behavior: header checks, preparation, linking and boundary normalization, over
 * small artifacts built in the test. Evaluation of real programs is the corpus's job.
 */
import { readFileSync } from "node:fs";
import { fileURLToPath } from "node:url";
import { NX_IR_NONE, NX_IR_REQUIRED_FEATURE_OCCURRENCE_V1, NX_IR_REQUIRED_FEATURE_RANGES_V1, NX_IR_RUNTIME_ABI, NX_IR_SCHEMA_VERSION, NX_PRELUDE_MODULE_IDENTITY, NX_PRELUDE_VERSION, NxIrRuntimeError, applyComponentStatePatch, applyUpdate, callFunction, changedFields, constructComponentDescriptor, declarationKinds, diffRecords, dispatchComponentActions, evaluateComponent, evaluateFunction, float32Text, initializeComponent, linkNxIrProgram, mergeUpdates, nodeKinds, normalizeComponentState, occurrenceFlags, prepareNxIrModule, prepareNxIrProgram, tryLinkNxIrProgram, tryPrepareNxIrModule, tryPrepareNxIrProgram, typeKinds, } from "../src/index.js";
const tests = [];
function test(name, run) {
    tests.push([name, run]);
}
function assertEqual(actual, expected) {
    const actualJson = stableJson(actual);
    const expectedJson = stableJson(expected);
    if (actualJson !== expectedJson) {
        throw new Error(`Expected ${expectedJson}, got ${actualJson}`);
    }
}
/** A value known to be an object, read as one. */
function fields(value) {
    if (value === null || typeof value !== "object" || Array.isArray(value)) {
        throw new Error(`Expected an object, got ${stableJson(value)}`);
    }
    return value;
}
function stableJson(value) {
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
function assertThrows(run, expectedMessage) {
    try {
        run();
    }
    catch (error) {
        if (!(error instanceof NxIrRuntimeError)) {
            throw error;
        }
        if (!error.diagnostics.some((diagnostic) => diagnostic.message.includes(expectedMessage))) {
            throw new Error(`Expected diagnostic containing '${expectedMessage}', got ${error.message}`);
        }
        return error;
    }
    throw new Error("Expected function to throw");
}
const sectionKinds = { strings: 0, module: 1, types: 2, constants: 3, nodes: 4, declarations: 5, debug: 6 };
/** Bit 1 of a parameter's flags cell: the parameter is optional (`p?:T`). */
const functionParamOptional = 2;
class ArtifactBuilder {
    strings = [];
    types = [];
    constants = [];
    nodes = [];
    declarations = [];
    functionEntrypoints = [];
    componentEntrypoints = [];
    #links;
    constructor(identity, links = [], version = "") {
        this.#links = [{ identity, version, fingerprint: "1" }, ...links];
    }
    str(value) {
        const existing = this.strings.indexOf(value);
        if (existing >= 0) {
            return existing;
        }
        this.strings.push(value);
        return this.strings.length - 1;
    }
    slot(identity) {
        const slot = this.#links.findIndex((link) => link.identity === identity);
        if (slot < 0) {
            throw new Error(`no link to ${identity}`);
        }
        return slot;
    }
    primitive(name) {
        return this.type([typeKinds.primitive, this.str(name)]);
    }
    nominal(name, identity) {
        return this.type([typeKinds.nominal, identity === undefined ? 0 : this.slot(identity), this.str(name)]);
    }
    /** `[5, item, occurrence]`: `item` under an occurrence, `1` for `?`, `2` for `+` and `3` for `*`. */
    seq(item, occurrence) {
        return this.type([typeKinds.seq, item, occurrence]);
    }
    /** `T?` */
    optional(item) {
        return this.seq(item, occurrenceFlags.empty);
    }
    /** `T+` */
    oneOrMore(item) {
        return this.seq(item, occurrenceFlags.many);
    }
    /** `T*` */
    zeroOrMore(item) {
        return this.seq(item, occurrenceFlags.empty | occurrenceFlags.many);
    }
    /**
     * `[4, result, [[name, type, flags]...]]`: a function type with named parameters, each with its
     * flags: bit 0 content, bit 1 optional.
     */
    functionType(result, params = []) {
        return this.type([
            typeKinds.function,
            result,
            params.length,
            ...params.flatMap(([name, ty, flags]) => [this.str(name), ty, flags ?? 0]),
        ]);
    }
    type(entry) {
        const key = entry.join(",");
        const existing = this.types.findIndex((candidate) => candidate.join(",") === key);
        if (existing >= 0) {
            return existing;
        }
        this.types.push(entry);
        return this.types.length - 1;
    }
    node(entry) {
        this.nodes.push(entry);
        return this.nodes.length - 1;
    }
    string(value) {
        return this.node([nodeKinds.string, this.str(value)]);
    }
    int(value) {
        // Two cells, low word first.
        this.constants.push([0, value >>> 0, Math.floor(value / 4294967296) >>> 0]);
        return this.node([nodeKinds.number, this.constants.length - 1]);
    }
    /** A `float64` constant, as the two cells of its bits, low word first. */
    float(value) {
        const bits = new DataView(new ArrayBuffer(8));
        bits.setFloat64(0, value, true);
        this.constants.push([2, bits.getUint32(0, true), bits.getUint32(4, true)]);
        return this.node([nodeKinds.number, this.constants.length - 1]);
    }
    /** `[20, node, str]`: the canonical text of `operand`, whose static type is `type`. */
    text(operand, type) {
        return this.node([nodeKinds.text, operand, this.str(type)]);
    }
    /** `count, element × count`, for a list operand. */
    list(elements) {
        return [elements.length, ...elements.flat()];
    }
    /** A `[node...]` operand. */
    content(nodes) {
        return [nodes.length, ...nodes];
    }
    field(name, ty, options = {}) {
        const flags = (options.content === true ? 1 : 0) + (options.required === true ? 2 : 0);
        return [this.str(name), ty, options.default ?? NX_IR_NONE, flags];
    }
    property(name, value) {
        return [this.str(name), value];
    }
    declaration(entry) {
        this.declarations.push(entry);
        return this.declarations.length - 1;
    }
    /**
     * A function declaration: `result` is the declared result type, and `optionalResult` says the
     * result type, declared or inferred, is a standalone `T?`. A parameter is `[name, type, flags]`,
     * or `[name, type, default, flags]` when it has a default node.
     */
    fn(name, body, params = [], options = {}) {
        const index = this.declaration([
            declarationKinds.function,
            this.str(name),
            ...this.list(params.map((param) => (param.length === 3 ? [param[0], param[1], NX_IR_NONE, param[2]] : param))),
            body,
            options.result ?? NX_IR_NONE,
            options.optionalResult === true ? 1 : 0,
        ]);
        this.functionEntrypoints.push(index);
        return index;
    }
    record(name, fields, options = {}) {
        return this.declaration([
            declarationKinds.record,
            this.str(name),
            ...this.list(fields),
            ...this.list(options.bases ?? []),
            options.abstract === true ? 1 : 0,
            ...(options.updateTarget ?? [NX_IR_NONE, NX_IR_NONE]),
        ]);
    }
    component(name, props, state, body, options = {}) {
        const flags = (options.abstract === true ? 1 : 0) + (options.external === true ? 2 : 0);
        const index = this.declaration([
            declarationKinds.component,
            this.str(name),
            ...this.list(props),
            ...this.list(state),
            body < 0 ? NX_IR_NONE : body,
            flags,
            ...this.list((options.emits ?? []).map(([emitName, action]) => [this.str(emitName), ...action])),
        ]);
        this.componentEntrypoints.push(index);
        return index;
    }
    /** `[19, ref, str, ref, slot, ref?, node]`: a handler for `component`'s `emit` accepting `action`. */
    handler(component, emit, action, slot, owner, body) {
        return this.node([nodeKinds.actionHandler, ...component, this.str(emit), ...action, slot, ...(owner ?? [NX_IR_NONE, NX_IR_NONE]), body]);
    }
    union(name, cases, options = {}) {
        return this.declaration([
            declarationKinds.union,
            this.str(name),
            ...this.list(cases.map(([caseName, fields, constant]) => [this.str(caseName), ...this.list(fields), constant ? 1 : 0])),
            ...this.list(options.bases ?? []),
            ...(options.propertyTarget ?? [NX_IR_NONE, NX_IR_NONE]),
        ]);
    }
    ref(name, identity) {
        return [identity === undefined ? 0 : this.slot(identity), this.str(name)];
    }
    /** Writes the image: header, directory, and the sections in kind order. */
    build(options = {}) {
        const runtimeAbi = this.str(options.runtimeAbi ?? NX_IR_RUNTIME_ABI);
        const features = (options.requiredFeatures ?? []).map((feature) => this.str(feature));
        const modules = this.#links.flatMap((link) => {
            const fingerprint = BigInt(link.fingerprint ?? "1");
            return [this.str(link.identity), this.str(link.version ?? ""), Number(fingerprint & 0xffffffffn), Number(fingerprint >> 32n)];
        });
        const module = [
            runtimeAbi,
            features.length,
            ...features,
            this.#links.length,
            ...modules,
            this.functionEntrypoints.length,
            ...this.functionEntrypoints,
            this.componentEntrypoints.length,
            ...this.componentEntrypoints,
        ];
        const encoder = new TextEncoder();
        const blob = encoder.encode(this.strings.join(""));
        const stringOffsets = [0];
        for (const value of this.strings) {
            stringOffsets.push(stringOffsets[stringOffsets.length - 1] + encoder.encode(value).byteLength);
        }
        const table = (entries) => {
            const offsets = [0];
            for (const entry of entries) {
                offsets.push(offsets[offsets.length - 1] + entry.length);
            }
            return [entries.length, ...offsets, ...entries.flat()];
        };
        const sections = [
            [sectionKinds.strings, bytesOf([this.strings.length, ...stringOffsets], blob)],
            [sectionKinds.module, bytesOf(module)],
            [sectionKinds.types, bytesOf(table(this.types))],
            [sectionKinds.constants, bytesOf(table(this.constants))],
            [sectionKinds.nodes, bytesOf(table(this.nodes))],
            [sectionKinds.declarations, bytesOf(table(this.declarations))],
        ];
        if (options.debug !== undefined) {
            const spans = (list) => [
                list.length,
                ...list.flatMap(([start, end]) => [start < 0 ? NX_IR_NONE : start, end < 0 ? NX_IR_NONE : end]),
            ];
            const source = encoder.encode(options.debug.source);
            sections.push([
                sectionKinds.debug,
                bytesOf([...spans(options.debug.declarations), ...spans(options.debug.nodes), source.byteLength], source),
            ]);
        }
        const directoryEnd = 16 + sections.length * 12;
        const total = directoryEnd + sections.reduce((sum, [, bytes]) => sum + bytes.byteLength, 0);
        const image = new Uint8Array(total);
        const view = new DataView(image.buffer);
        image.set(encoder.encode("NXIR"), 0);
        view.setUint32(4, NX_IR_SCHEMA_VERSION, true);
        view.setUint32(8, total, true);
        view.setUint32(12, sections.length, true);
        let offset = directoryEnd;
        sections.forEach(([kind, bytes], index) => {
            view.setUint32(16 + index * 12, kind, true);
            view.setUint32(16 + index * 12 + 4, offset, true);
            view.setUint32(16 + index * 12 + 8, bytes.byteLength, true);
            image.set(bytes, offset);
            offset += bytes.byteLength;
        });
        return image;
    }
}
/** Cells as little-endian bytes, followed by a blob padded to four bytes. */
function bytesOf(cells, blob = new Uint8Array(0)) {
    const padded = Math.ceil(blob.byteLength / 4) * 4;
    const bytes = new Uint8Array(cells.length * 4 + padded);
    const view = new DataView(bytes.buffer);
    cells.forEach((cell, index) => view.setUint32(index * 4, cell, true));
    bytes.set(blob, cells.length * 4);
    return bytes;
}
/** A catalog with `SkiaLabel` (Text, FontSize = 14) and `SkiaButton` (Text). */
function catalog(version, extra = () => { }) {
    const b = new ArtifactBuilder("drawnui.nx", [], version);
    const text = b.field("Text", b.primitive("string"), { required: true });
    const fontSize = b.field("FontSize", b.primitive("int"), { default: b.int(14) });
    extra(b);
    b.component("SkiaLabel", [text, fontSize], [], -1, { external: true });
    b.component("SkiaButton", [text], [], -1, { external: true });
    return b.build();
}
/** A snippet whose root is `<SkiaLabel Text="hi" />`, compiled against catalog `version`. */
function snippet(version, componentName = "SkiaLabel") {
    const b = new ArtifactBuilder("input.nx", [{ identity: "drawnui.nx", version, fingerprint: "7" }]);
    const hi = b.string("hi");
    const body = b.node([nodeKinds.component, b.slot("drawnui.nx"), b.str(componentName), ...b.list([b.property("Text", hi)]), ...b.content([])]);
    b.fn("root", body);
    return b.build();
}
// ------------------------------------------------------------------------------------------------
// Header and preparation
// ------------------------------------------------------------------------------------------------
test("prepares a self-contained artifact and evaluates it without linking", () => {
    const b = new ArtifactBuilder("main.nx");
    b.fn("root", b.node([nodeKinds.binary, 0, b.int(1), b.int(2)]));
    const artifact = b.build();
    const program = prepareNxIrProgram(artifact);
    assertEqual(evaluateFunction(program, "root"), 3);
    // A prepared module whose table names only itself is a program on its own.
    assertEqual(evaluateFunction(prepareNxIrModule(artifact), "root"), 3);
    // An ArrayBuffer is read in place; a view at an odd offset is copied and read the same.
    assertEqual(prepareNxIrModule(artifact.buffer).functionEntrypoints.has("root"), true);
    const shifted = new Uint8Array(artifact.byteLength + 2);
    shifted.set(artifact, 2);
    assertEqual(evaluateFunction(prepareNxIrProgram(shifted.subarray(2)), "root"), 3);
});
test("refuses schema 4 naming both versions, and unknown ABIs and features", () => {
    const b = new ArtifactBuilder("main.nx");
    b.fn("root", b.int(1));
    const artifact = b.build();
    assertEqual(new DataView(artifact.buffer, artifact.byteOffset).getUint32(4, true), 5);
    const old = artifact.slice();
    new DataView(old.buffer).setUint32(4, 4, true);
    const schema4 = tryPrepareNxIrModule(old);
    assertEqual(schema4.ok, false);
    if (!schema4.ok) {
        const message = schema4.diagnostics[0].message;
        assertEqual(message.includes("schema version 4"), true);
        assertEqual(message.includes("schema version 5"), true);
    }
    assertEqual(tryPrepareNxIrModule(b.build({ runtimeAbi: "nx-ir-runtime-v1" })).ok, false);
    const notAnImage = tryPrepareNxIrModule(new TextEncoder().encode('{"format":"nx-ir-json","schemaVersion":3}'));
    assertEqual(notAnImage.ok, false);
    if (!notAnImage.ok) {
        assertEqual(notAnImage.diagnostics[0].message, "The input is not an NX IR image.");
    }
    const feature = tryPrepareNxIrProgram(b.build({ requiredFeatures: ["future-reactivity"] }));
    assertEqual(feature.ok, false);
    if (!feature.ok) {
        assertEqual(feature.diagnostics[0].message.includes("future-reactivity"), true);
    }
});
// ------------------------------------------------------------------------------------------------
// Primitive text and string concatenation
// ------------------------------------------------------------------------------------------------
/** The `concat` operator's number, as `docs/nx-ir-format.md` assigns it. */
const CONCAT = 7;
const LT = 10;
/** A one-parameter function `f(value) = text<type>(value)`. */
function textOfParameter(type) {
    const b = new ArtifactBuilder("main.nx");
    const operand = b.node([nodeKinds.slot, 0, b.str("value")]);
    b.fn("f", b.text(operand, type), [[b.str("value"), b.primitive(type), 0]]);
    return prepareNxIrProgram(b.build());
}
test("a text node over an int prints its digits and concat joins the strings", () => {
    const b = new ArtifactBuilder("main.nx");
    const count = b.node([nodeKinds.slot, 0, b.str("count")]);
    const body = b.node([nodeKinds.binary, CONCAT, b.string("Total: "), b.text(count, "int")]);
    b.fn("f", body, [[b.str("count"), b.primitive("int"), 0]]);
    const program = prepareNxIrProgram(b.build());
    assertEqual(evaluateFunction(program, "f", [3]), "Total: 3");
    assertEqual(evaluateFunction(program, "f", [-42]), "Total: -42");
});
test("a text node over a float32 prints as a float32, not as the float64 it is carried in", () => {
    const tenth = Math.fround(0.1);
    assertEqual(String(tenth), "0.10000000149011612");
    assertEqual(evaluateFunction(textOfParameter("float32"), "f", [tenth]), "0.1");
    // The same carried number under its own type prints every digit it has.
    assertEqual(evaluateFunction(textOfParameter("float64"), "f", [tenth]), "0.10000000149011612");
    assertEqual(float32Text(Math.fround(1.5)), "1.5");
    assertEqual(float32Text(Math.fround(16777216)), "16777216");
    assertEqual(float32Text(Math.fround(1e-7)), "1e-7");
    assertEqual(float32Text(Math.fround(1e21)), "1e+21");
    assertEqual(float32Text(Math.fround(3.4028234663852886e38)), "3.4028235e+38");
    assertEqual(float32Text(-0), "0");
    assertEqual(float32Text(Number.NaN), "NaN");
    assertEqual(float32Text(Number.NEGATIVE_INFINITY), "-Infinity");
    // The helper rounds what it is given, so an unrounded float64 still prints as its float32.
    assertEqual(float32Text(Math.fround(2.3) * 3), "6.8999996");
    assertEqual(evaluateFunction(textOfParameter("float32"), "f", [Math.fround(2.3) * 3]), "6.8999996");
});
/** The float32 operators' numbers, as `docs/nx-ir-format.md` assigns them. */
const FADD32 = 16;
const FMUL32 = 18;
const FDIV32 = 19;
test("a float32 operator rounds its result to a float32, as the interpreter computes it", () => {
    const b = new ArtifactBuilder("main.nx");
    const v = b.node([nodeKinds.slot, 0, b.str("v")]);
    b.fn("scaled", b.node([nodeKinds.binary, FMUL32, v, b.float(3)]), [[b.str("v"), b.primitive("float32"), 0]]);
    const sum = b.node([nodeKinds.binary, FADD32, b.node([nodeKinds.slot, 0, b.str("v")]), b.float(Math.fround(0.1))]);
    b.fn("sum", sum, [[b.str("v"), b.primitive("float32"), 0]]);
    const zero = b.node([nodeKinds.binary, FDIV32, b.float(1), b.float(0)]);
    b.fn("zero", zero);
    const program = prepareNxIrProgram(b.build());
    // Unrounded, the product would be the float64 6.8999998569488525.
    assertEqual(evaluateFunction(program, "scaled", [2.3]), 6.899999618530273);
    assertEqual(evaluateFunction(program, "sum", [0.2]), Math.fround(Math.fround(0.2) + Math.fround(0.1)));
    assertThrows(() => evaluateFunction(program, "zero"), "Division by zero");
});
test("a host number takes the width of an int32 or float32 parameter", () => {
    const program = textOfParameter("float32");
    // A real rounds to the nearest float32, and an integer a float32 holds exactly is kept.
    assertEqual(evaluateFunction(program, "f", [0.1]), "0.1");
    assertEqual(evaluateFunction(program, "f", [16777216]), "16777216");
    assertThrows(() => evaluateFunction(program, "f", [16777217]), "not exact as a float32");
    const int32 = textOfParameter("int32");
    assertEqual(evaluateFunction(int32, "f", [-2147483648]), "-2147483648");
    assertThrows(() => evaluateFunction(int32, "f", [3000000000]), "out of range for int32");
    assertThrows(() => evaluateFunction(int32, "f", [1.5]), "to be an int32, got 1.5.");
});
test("a float64 prints in the ECMAScript form: no fraction when integral, exponents at the ends", () => {
    const program = textOfParameter("float64");
    assertEqual(evaluateFunction(program, "f", [1]), "1");
    assertEqual(evaluateFunction(program, "f", [0.1]), "0.1");
    assertEqual(evaluateFunction(program, "f", [1e21]), "1e+21");
    assertEqual(evaluateFunction(program, "f", [1e-7]), "1e-7");
    assertEqual(evaluateFunction(program, "f", [-0]), "0");
});
test("a boolean prints as true or false, and a wide integer as its digits", () => {
    const program = textOfParameter("boolean");
    assertEqual(evaluateFunction(program, "f", [true]), "true");
    assertEqual(evaluateFunction(program, "f", [false]), "false");
    // An integer outside the safe range is a `bigint` constant, carried as its digits.
    const wide = new ArtifactBuilder("main.nx");
    wide.constants.push([1, wide.str("9007199254740993")]);
    wide.fn("root", wide.text(wide.node([nodeKinds.number, wide.constants.length - 1]), "int64"));
    assertEqual(evaluateFunction(prepareNxIrProgram(wide.build()), "root"), "9007199254740993");
});
test("a text node refuses an operand that is not the primitive it names", () => {
    const b = new ArtifactBuilder("main.nx");
    b.fn("root", b.text(b.string("three"), "int"));
    const error = assertThrows(() => evaluateFunction(prepareNxIrProgram(b.build()), "root"), "text conversion");
    assertEqual(error.diagnostics[0].code, "nx-ir-operator");
    const flag = new ArtifactBuilder("main.nx");
    flag.fn("root", flag.text(flag.int(1), "boolean"));
    assertThrows(() => evaluateFunction(prepareNxIrProgram(flag.build()), "root"), "text conversion");
});
test("the validator accepts kind 20 only as [20, node, str] naming a type with a text form", () => {
    const named = new ArtifactBuilder("main.nx");
    named.fn("root", named.text(named.int(1), "string"));
    const unknownType = tryPrepareNxIrModule(named.build());
    assertEqual(unknownType.ok, false);
    if (!unknownType.ok) {
        assertEqual(unknownType.diagnostics[0].message.includes("'string'"), true);
    }
    const short = new ArtifactBuilder("main.nx");
    short.fn("root", short.node([nodeKinds.text, short.int(1)]));
    assertEqual(tryPrepareNxIrModule(short.build()).ok, false);
    const long = new ArtifactBuilder("main.nx");
    long.fn("root", long.node([nodeKinds.text, long.int(1), long.str("int"), 0]));
    assertEqual(tryPrepareNxIrModule(long.build()).ok, false);
    const forward = new ArtifactBuilder("main.nx");
    forward.fn("root", forward.node([nodeKinds.text, 7, forward.str("int")]));
    assertEqual(tryPrepareNxIrModule(forward.build()).ok, false);
});
test("concat over a non-string operand is refused", () => {
    const b = new ArtifactBuilder("main.nx");
    b.fn("root", b.node([nodeKinds.binary, CONCAT, b.string("Total: "), b.int(3)]));
    const error = assertThrows(() => evaluateFunction(prepareNxIrProgram(b.build()), "root"), "'concat' requires string operands");
    assertEqual(error.diagnostics[0].code, "nx-ir-operator");
});
test("mixed numeric comparison and arithmetic need no conversion and compare numerically", () => {
    const b = new ArtifactBuilder("main.nx");
    const n = b.node([nodeKinds.slot, 0, b.str("n")]);
    const x = b.node([nodeKinds.slot, 1, b.str("x")]);
    const params = [
        [b.str("n"), b.primitive("int"), 0],
        [b.str("x"), b.primitive("float64"), 0],
    ];
    b.fn("less", b.node([nodeKinds.binary, LT, n, x]), params);
    b.fn("sum", b.node([nodeKinds.binary, 0, n, x]), params);
    b.fn("same", b.node([nodeKinds.binary, 8, n, x]), params);
    const program = prepareNxIrProgram(b.build());
    assertEqual(evaluateFunction(program, "less", [2, 2.5]), true);
    assertEqual(evaluateFunction(program, "sum", [1, 1.5]), 2.5);
    assertEqual(evaluateFunction(program, "same", [2, 2.0]), true);
});
test("refuses a truncated image and a wrong length with a diagnostic", () => {
    const b = new ArtifactBuilder("main.nx");
    b.fn("root", b.int(1));
    const artifact = b.build();
    for (let end = 0; end < artifact.byteLength; end += 4) {
        const result = tryPrepareNxIrModule(artifact.subarray(0, end));
        assertEqual(result.ok, false);
    }
    const longer = new Uint8Array(artifact.byteLength + 4);
    longer.set(artifact);
    assertEqual(tryPrepareNxIrModule(longer).ok, false);
});
test("rejects unknown kinds and dangling indices during preparation", () => {
    const b = new ArtifactBuilder("main.nx");
    b.fn("root", b.node([99]));
    const unknownKind = tryPrepareNxIrModule(b.build());
    assertEqual(unknownKind.ok, false);
    const c = new ArtifactBuilder("main.nx");
    c.fn("root", c.node([nodeKinds.string, 42]));
    assertEqual(tryPrepareNxIrModule(c.build()).ok, false);
    const d = new ArtifactBuilder("main.nx");
    d.fn("root", d.node([nodeKinds.reference, 0, d.str("missing")]));
    const missing = tryPrepareNxIrModule(d.build());
    assertEqual(missing.ok, false);
    if (!missing.ok) {
        assertEqual(missing.diagnostics[0].message.includes("'missing'"), true);
    }
});
test("refuses to evaluate an unlinked module that names another module", () => {
    const module = prepareNxIrModule(snippet("9"));
    assertThrows(() => evaluateFunction(module, "root"), "must be linked");
});
// ------------------------------------------------------------------------------------------------
// Linking
// ------------------------------------------------------------------------------------------------
test("links a snippet against a prepared catalog and evaluates it", () => {
    const prepared = prepareNxIrModule(catalog("9"));
    const program = linkNxIrProgram(prepareNxIrModule(snippet("9")), {
        resolve: (identity) => (identity === "drawnui.nx" ? prepared : undefined),
    });
    assertEqual(evaluateFunction(program, "root"), { $type: "SkiaLabel", Text: "hi", FontSize: 14 });
});
test("refuses a version mismatch by default and names both versions", () => {
    const prepared = prepareNxIrModule(catalog("10"));
    const result = tryLinkNxIrProgram(prepareNxIrModule(snippet("9")), { resolve: () => prepared });
    assertEqual(result.ok, false);
    if (!result.ok) {
        const message = result.diagnostics[0].message;
        assertEqual(message.includes("drawnui.nx"), true);
        assertEqual(message.includes("'9'"), true);
        assertEqual(message.includes("'10'"), true);
    }
});
test("links across versions when the host opts in and every declaration resolves", () => {
    const prepared = prepareNxIrModule(catalog("10"));
    const program = linkNxIrProgram(prepareNxIrModule(snippet("9")), {
        resolve: () => prepared,
        allowVersionMismatch: true,
    });
    assertEqual(evaluateFunction(program, "root"), { $type: "SkiaLabel", Text: "hi", FontSize: 14 });
});
test("refuses a missing declaration naming the module and the declaration", () => {
    const prepared = prepareNxIrModule(catalog("9"));
    const result = tryLinkNxIrProgram(prepareNxIrModule(snippet("9", "SkiaSlider")), { resolve: () => prepared });
    assertEqual(result.ok, false);
    if (!result.ok) {
        const message = result.diagnostics[0].message;
        assertEqual(message.includes("drawnui.nx"), true);
        assertEqual(message.includes("'SkiaSlider'"), true);
    }
});
test("refuses a module the resolver cannot supply, naming its identity", () => {
    const result = tryLinkNxIrProgram(prepareNxIrModule(snippet("9")), { resolve: () => undefined });
    assertEqual(result.ok, false);
    if (!result.ok) {
        assertEqual(result.diagnostics[0].message.includes("'drawnui.nx'"), true);
    }
});
test("a regenerated catalog with a new control ahead of SkiaLabel still resolves by name", () => {
    const regenerated = prepareNxIrModule(catalog("9", (b) => {
        b.component("SkiaSlider", [b.field("Value", b.primitive("int"))], [], -1, { external: true });
    }));
    const program = linkNxIrProgram(prepareNxIrModule(snippet("9")), { resolve: () => regenerated });
    assertEqual(evaluateFunction(program, "root"), { $type: "SkiaLabel", Text: "hi", FontSize: 14 });
});
test("one prepared module serves many programs without being copied", () => {
    let prepared = 0;
    const cache = new Map();
    const resolve = (identity) => {
        if (!cache.has(identity)) {
            prepared += 1;
            cache.set(identity, prepareNxIrModule(catalog("9")));
        }
        return cache.get(identity);
    };
    const programs = Array.from({ length: 100 }, () => linkNxIrProgram(prepareNxIrModule(snippet("9")), { resolve }));
    assertEqual(prepared, 1);
    for (const program of programs) {
        // The very object the resolver handed out, not a copy.
        if (program.entry.slots[1].module !== cache.get("drawnui.nx")) {
            throw new Error("the linked program does not hold the resolver's prepared module");
        }
        assertEqual(evaluateFunction(program, "root"), { $type: "SkiaLabel", Text: "hi", FontSize: 14 });
    }
});
/**
 * Linking must not walk the linked modules' declarations. A catalog is the largest module in play
 * and a host links against it on every mount, so preparation is what pays for indexing its shapes;
 * a link reads that index rather than rebuilding one.
 */
test("linking reads the prepared module's shape index rather than rebuilding it", () => {
    const prepared = prepareNxIrModule(catalog("9", (b) => {
        b.record("Card", [b.field("name", b.primitive("string"), { required: true })]);
    }));
    // Indexed once, at preparation, before any program exists.
    const indexed = prepared.nominalShapeSkeletons.get("Card");
    assertEqual(indexed?.length, 1);
    let walked = 0;
    const counting = {
        ...prepared,
        get declarations() {
            walked += 1;
            return prepared.declarations;
        },
    };
    const programs = Array.from({ length: 10 }, () => linkNxIrProgram(prepareNxIrModule(snippet("9")), { resolve: () => counting }));
    assertEqual(walked, 0);
    // The very arrays preparation built, handed out rather than rebuilt per program.
    for (const program of programs) {
        const shapes = program.nominalShapesFor("Card");
        assertEqual(shapes.length, 1);
        if (shapes[0].fields !== indexed[0].fields) {
            throw new Error("the linked shape does not reuse the prepared module's field list");
        }
    }
});
// ------------------------------------------------------------------------------------------------
// Diagnostics
// ------------------------------------------------------------------------------------------------
test("a runtime diagnostic cites the span with debug data and the declaration without", () => {
    const b = new ArtifactBuilder("main.nx");
    const zero = b.int(0);
    const one = b.int(1);
    b.fn("root", b.node([nodeKinds.binary, 4, one, zero]));
    const stripped = b.build();
    const source = "let root() = { 1 / 0 }";
    const withDebug = b.build({
        debug: { declarations: [[0, source.length]], nodes: [[19, 20], [15, 16], [15, 20]], source },
    });
    const strippedError = assertThrows(() => evaluateFunction(prepareNxIrProgram(stripped), "root"), "Division by zero");
    assertEqual(strippedError.diagnostics[0].declaration, "main.nx::root");
    assertEqual(strippedError.diagnostics[0].source, undefined);
    const debugError = assertThrows(() => evaluateFunction(prepareNxIrProgram(withDebug), "root"), "Division by zero");
    assertEqual(debugError.diagnostics[0].declaration, "main.nx::root");
    assertEqual(debugError.diagnostics[0].source, { identity: "main.nx", start: 15, end: 20 });
});
// ------------------------------------------------------------------------------------------------
// Boundary normalization with host input
// ------------------------------------------------------------------------------------------------
/**
 * `abstract Base { name }`, `User extends Base { role }`, `Theme = light | dark`, `LoadState =
 * idle | failed { message }`, and a `Card` component with a `user:User`, `theme?:Theme`, a
 * `load?:LoadState` and `content children?:object+`, plus state `{ count:int = 0 }`.
 */
function shapesArtifact() {
    const b = new ArtifactBuilder("main.nx");
    const string = b.primitive("string");
    const int = b.primitive("int");
    b.record("Base", [b.field("name", string, { required: true })], { abstract: true });
    b.record("User", [b.field("name", string, { required: true }), b.field("role", string, { required: true })], {
        bases: [b.ref("Base")],
    });
    b.record("Other", [b.field("name", string, { required: true })]);
    b.union("Theme", [
        ["light", [], true],
        ["dark", [], true],
    ]);
    b.union("LoadState", [
        ["idle", [], true],
        ["failed", [b.field("message", string, { required: true })], false],
    ]);
    const props = [
        b.field("user", b.nominal("Base"), { required: true }),
        b.field("theme", b.optional(b.nominal("Theme"))),
        b.field("load", b.optional(b.nominal("LoadState"))),
        b.field("children", b.zeroOrMore(b.primitive("object")), { content: true }),
    ];
    const state = [b.field("count", int, { default: b.int(0) })];
    // The body renders `<div count={count} />`.
    const body = b.node([nodeKinds.element, 0, b.str("div"), ...b.list([b.property("count", b.node([nodeKinds.slot, 4, b.str("count")]))]), ...b.content([])]);
    b.component("Card", props, state, body);
    return b.build();
}
test("accepts a derived record at a base-typed prop and rejects an unrelated or abstract one", () => {
    const program = prepareNxIrProgram(shapesArtifact());
    const user = { $type: "User", name: "Ada", role: "admin" };
    // An empty optional prop is an omitted key, not a null.
    assertEqual(constructComponentDescriptor(program, "Card", { user }), { $type: "Card", user });
    assertThrows(() => constructComponentDescriptor(program, "Card", { user: { $type: "Other", name: "x" } }), "Expected Card props.user to be a Base");
    assertThrows(() => constructComponentDescriptor(program, "Card", { user: { name: "x" } }), "concrete type extending Base");
    assertThrows(() => constructComponentDescriptor(program, "Card", { user: { $type: "Base", name: "x" } }), "got abstract 'Base'");
    assertThrows(() => constructComponentDescriptor(program, "Card", { user, extra: 1 }), "Unknown Card props field 'extra'");
    assertThrows(() => constructComponentDescriptor(program, "Card", {}), "Missing required Card props field 'user'");
});
/**
 * A `$type` discriminator is a bare name, and since schema 3 a program spans several modules, so
 * two of them may each declare a record of one name extending the same base. The runtime reports
 * that rather than picking one: the two have different fields, and guessing would normalize the
 * value against the wrong schema.
 */
test("reports a subtype whose name two modules share rather than guessing", () => {
    const base = new ArtifactBuilder("base.nx", [], "1");
    base.record("Base", [base.field("name", base.primitive("string"), { required: true })], { abstract: true });
    const preparedBase = prepareNxIrModule(base.build());
    /** A module declaring `Card extends base.nx::Base`, with a field of its own. */
    const cardModule = (identity, ownField) => {
        const b = new ArtifactBuilder(identity, [{ identity: "base.nx", version: "1", fingerprint: "1" }], "1");
        const string = b.primitive("string");
        b.record("Card", [b.field("name", string, { required: true }), b.field(ownField, string, { required: true })], {
            bases: [b.ref("Base", "base.nx")],
        });
        return prepareNxIrModule(b.build());
    };
    const left = cardModule("left.nx", "left");
    const right = cardModule("right.nx", "right");
    const entry = new ArtifactBuilder("main.nx", [
        { identity: "base.nx", version: "1", fingerprint: "1" },
        { identity: "left.nx", version: "1", fingerprint: "1" },
        { identity: "right.nx", version: "1", fingerprint: "1" },
    ]);
    entry.component("Host", [entry.field("item", entry.nominal("Base", "base.nx"), { required: true })], [], -1, { external: true });
    const modules = { "base.nx": preparedBase, "left.nx": left, "right.nx": right };
    const program = linkNxIrProgram(prepareNxIrModule(entry.build()), { resolve: (identity) => modules[identity] });
    assertThrows(() => constructComponentDescriptor(program, "Host", { item: { $type: "Card", name: "a", left: "x" } }), "2 declarations named 'Card' extend Base");
});
test("normalizes constant and payload union cases from host input", () => {
    const program = prepareNxIrProgram(shapesArtifact());
    const user = { $type: "User", name: "Ada", role: "admin" };
    assertEqual(fields(constructComponentDescriptor(program, "Card", { user, theme: "dark" })).theme, "dark");
    assertThrows(() => constructComponentDescriptor(program, "Card", { user, theme: "blue" }), "'blue' is not a case of Theme");
    assertEqual(fields(constructComponentDescriptor(program, "Card", { user, load: { $type: "LoadState.failed", message: "x" } })).load, {
        $type: "LoadState.failed",
        message: "x",
    });
    assertThrows(() => constructComponentDescriptor(program, "Card", { user, load: { $type: "LoadState.exploded" } }), "Invalid union case 'LoadState.exploded'");
    // A host's `null` and `[]` are the empty value, which the descriptor carries no key for.
    for (const empty of [null, []]) {
        const descriptor = fields(constructComponentDescriptor(program, "Card", { user, load: empty }));
        assertEqual(Object.prototype.hasOwnProperty.call(descriptor, "load"), false);
    }
});
test("binds content as a list whatever the child count", () => {
    const program = prepareNxIrProgram(shapesArtifact());
    const user = { $type: "User", name: "Ada", role: "admin" };
    assertEqual(fields(constructComponentDescriptor(program, "Card", { user }, [{ $type: "span" }])).children, [{ $type: "span" }]);
    assertEqual(fields(constructComponentDescriptor(program, "Card", { user }, [1, 2])).children, [1, 2]);
    assertThrows(() => constructComponentDescriptor(program, "Card", { user, children: [] }, [1]), "supplied both as a property and as content");
});
test("initializes, evaluates and patches a component's host-owned state", () => {
    const program = prepareNxIrProgram(shapesArtifact());
    const user = { $type: "User", name: "Ada", role: "admin" };
    const initialized = initializeComponent(program, "Card", { user });
    assertEqual(initialized.state, { count: 0 });
    assertEqual(initialized.rendered, { $type: "div", count: 0 });
    assertEqual(evaluateComponent(program, "Card", { user }, { count: 3 }).rendered, { $type: "div", count: 3 });
    assertThrows(() => evaluateComponent(program, "Card", { user }, {}), "Missing required Card state field 'count'");
    assertEqual(normalizeComponentState(program, "Card", { count: 2 }), { count: 2 });
    assertEqual(applyComponentStatePatch(program, "Card", { count: 1 }, { count: 5 }), { count: 5 });
    assertEqual(applyComponentStatePatch(program, "Card", { count: 1 }, { $type: "Card.Update", count: 6 }), { count: 6 });
    assertThrows(() => applyComponentStatePatch(program, "Card", { count: 1 }, { $type: "Other.Update" }), "only 'Card.Update' patches it");
    assertThrows(() => applyComponentStatePatch(program, "Card", { count: 1 }, { size: 2 }), "Unknown Card state field 'size'");
});
test("host input for an update record keeps absent fields absent and clears only a clearable field", () => {
    const b = new ArtifactBuilder("main.nx");
    const string = b.primitive("string");
    b.record("User", [b.field("name", string, { required: true }), b.field("email", b.optional(string))]);
    b.record("User.Update", [b.field("name", string), b.field("email", b.optional(string))], { updateTarget: b.ref("User") });
    b.fn("same", b.node([nodeKinds.slot, 0, b.str("patch")]), [[b.str("patch"), b.nominal("User.Update"), 0]]);
    const program = prepareNxIrProgram(b.build({ requiredFeatures: ["update-records-v1"] }));
    // A present `null` or `[]` is a cleared field, encoded as `null`; a missing key stays missing.
    assertEqual(evaluateFunction(program, "same", [{ $type: "User.Update", email: null }]), { $type: "User.Update", email: null });
    assertEqual(evaluateFunction(program, "same", [{ $type: "User.Update", email: [] }]), { $type: "User.Update", email: null });
    assertEqual(evaluateFunction(program, "same", [{ name: "Bo" }]), { $type: "User.Update", name: "Bo" });
    for (const empty of [null, []]) {
        const refused = assertThrows(() => evaluateFunction(program, "same", [{ name: empty }]), "clears a field only where the target declares it optional");
        assertEqual(refused.diagnostics[0].message.includes("name"), true);
    }
});
test("the exported helpers apply, merge, diff and list changed fields on host-held values", () => {
    const b = new ArtifactBuilder("main.nx");
    const string = b.primitive("string");
    b.record("User", [b.field("name", string, { required: true }), b.field("email", b.optional(string)), b.field("age", b.optional(b.primitive("int")))]);
    b.record("User.Update", [b.field("name", string), b.field("email", b.optional(string)), b.field("age", b.optional(b.primitive("int")))], {
        updateTarget: b.ref("User"),
    });
    const program = prepareNxIrProgram(b.build({ requiredFeatures: ["update-records-v1"] }));
    const user = { $type: "User", name: "Ada", email: "a@b", age: 3 };
    // A cleared field is left out of the applied record, which is how an empty optional is encoded.
    assertEqual(applyUpdate(user, { $type: "User.Update", email: null }), { $type: "User", name: "Ada", age: 3 });
    assertEqual(mergeUpdates({ $type: "User.Update", age: null }, { $type: "User.Update", name: "Bo" }), {
        $type: "User.Update",
        age: null,
        name: "Bo",
    });
    assertEqual(diffRecords(user, { ...user, name: "Bo" }), { $type: "User.Update", name: "Bo" });
    // An omitted optional is empty on both sides of a diff, and a field `after` drops is cleared.
    assertEqual(diffRecords(user, { $type: "User", name: "Ada", age: 3 }), { $type: "User.Update", email: null });
    assertEqual(diffRecords({ $type: "User", name: "Ada" }, { $type: "User", name: "Ada", email: null }), { $type: "User.Update" });
    assertEqual(changedFields({ $type: "User.Update", age: null, name: "Ada" }, program), ["name", "age"]);
    assertThrows(() => applyUpdate(user, { $type: "Other.Update" }), "only 'User.Update' patches it");
    // Each helper compares bare `$type` names, which two modules may share, so each checks its own
    // target rather than trusting the caller to have paired the values.
    assertThrows(() => mergeUpdates({ $type: "User.Update", name: "Bo" }, { $type: "Other.Update" }), "the updates target different records");
    assertThrows(() => diffRecords(user, { $type: "Other", name: "Bo" }), "the records have different types");
});
// ------------------------------------------------------------------------------------------------
// Action handlers
// ------------------------------------------------------------------------------------------------
/**
 * `component <Counter step:int = 1 emits { Reset } /> = { state { count:int = 0 } [<Button
 * label="Add" onTapped=<Update count={count + step} /> />, <Button label="Bad"
 * onTapped=<Update count="oops" /> />] }` beside `external component <Button label:string emits
 * { Tapped { } } />` and `action Reset = { }`, plus functions whose descriptors bind a handler
 * under `onReset`, under a name no emit has, and under `onReset` for the wrong emit.
 */
function counterArtifact() {
    const b = new ArtifactBuilder("main.nx");
    const string = b.primitive("string");
    const int = b.primitive("int");
    b.record("Button.Tapped", []);
    b.record("Reset", []);
    b.component("Button", [b.field("label", string, { required: true })], [], -1, {
        external: true,
        emits: [["Tapped", b.ref("Button.Tapped")]],
    });
    b.record("Counter.Update", [b.field("count", int)], { updateTarget: b.ref("Counter") });
    // Counter's frame: step at 0, count at 1, and each handler's `action` at 2.
    const count = b.node([nodeKinds.slot, 1, b.str("count")]);
    const step = b.node([nodeKinds.slot, 0, b.str("step")]);
    const sum = b.node([nodeKinds.binary, 0, count, step]);
    const patch = b.node([nodeKinds.record, ...b.ref("Counter.Update"), ...b.list([b.property("count", sum)]), ...b.content([])]);
    const add = b.handler(b.ref("Button"), "Tapped", b.ref("Button.Tapped"), 2, b.ref("Counter"), patch);
    const oops = b.node([nodeKinds.record, ...b.ref("Counter.Update"), ...b.list([b.property("count", b.string("oops"))]), ...b.content([])]);
    const bad = b.handler(b.ref("Button"), "Tapped", b.ref("Button.Tapped"), 2, b.ref("Counter"), oops);
    const button = (label, handler) => b.node([nodeKinds.component, ...b.ref("Button"), ...b.list([b.property("label", b.string(label)), b.property("onTapped", handler)]), ...b.content([])]);
    const body = b.node([nodeKinds.array, ...b.content([button("Add", add), button("Bad", bad)])]);
    b.component("Counter", [b.field("step", int, { default: b.int(1) })], [b.field("count", int, { default: b.int(0) })], body, {
        emits: [["Reset", b.ref("Reset")]],
    });
    // Root-bound handlers: `action` is the function frame's first slot.
    const reset = b.node([nodeKinds.record, ...b.ref("Reset"), ...b.list([]), ...b.content([])]);
    const onReset = b.handler(b.ref("Counter"), "Reset", b.ref("Reset"), 0, undefined, reset);
    const counter = (property, handler) => b.node([nodeKinds.component, ...b.ref("Counter"), ...b.list([b.property(property, handler), b.property("step", b.int(2))]), ...b.content([])]);
    b.fn("card", counter("onReset", onReset));
    b.fn("nope", counter("onNope", onReset));
    b.fn("wrongEmit", counter("onReset", b.handler(b.ref("Button"), "Tapped", b.ref("Button.Tapped"), 0, undefined, reset)));
    return b.build({ requiredFeatures: ["update-records-v1", "action-handlers-v1"] });
}
/** The token of the rendered `Button` at `index` of a `Counter`'s output. */
function counterToken(rendered, index = 0) {
    const button = fields(rendered[index]);
    const handler = fields(button.onTapped);
    if (typeof handler.token !== "string") {
        throw new Error(`no token in ${stableJson(button)}`);
    }
    return handler.token;
}
function tapped(token) {
    return { $type: "ActionHandlerInvocation", token, action: { $type: "Button.Tapped" } };
}
test("prepares an image listing the handler feature and refuses one naming a feature it does not know", () => {
    const program = prepareNxIrProgram(counterArtifact());
    assertEqual(program.entry.module.artifact.requiredFeatures, ["update-records-v1", "action-handlers-v1"]);
    const counter = program.componentEntrypoints.get("Counter").kind;
    if (counter.tag !== "component") {
        throw new Error("Counter is not a component");
    }
    assertEqual(counter.emits, [{ name: "Reset", action: { slot: 0, name: "Reset" } }]);
    const b = new ArtifactBuilder("main.nx");
    b.fn("root", b.int(1));
    const refused = tryPrepareNxIrModule(b.build({ requiredFeatures: ["action-handlers-v2"] }));
    if (refused.ok || !refused.diagnostics.some((diagnostic) => diagnostic.code === "nx-ir-required-feature" && diagnostic.message.includes("action-handlers-v2"))) {
        throw new Error("expected the unknown feature to be refused by name");
    }
});
test("a descriptor keeps its handler property as a token-less ActionHandler record", () => {
    const program = prepareNxIrProgram(counterArtifact());
    const card = fields(evaluateFunction(program, "card"));
    assertEqual(card, { $type: "Counter", step: 2, onReset: { $type: "ActionHandler", action: "Reset" } });
    assertThrows(() => evaluateFunction(program, "nope"), "Unknown Counter props field 'onNope'");
    assertThrows(() => evaluateFunction(program, "wrongEmit"), "Expected Counter props.onReset to be an action handler for Counter.Reset, got one for Button.Tapped");
    const evaluated = evaluateComponent(program, "Counter", {}, { count: 3 }).rendered;
    assertEqual(fields(evaluated[0]).onTapped, { $type: "ActionHandler", action: "Button.Tapped" });
    if (stableJson(evaluated).includes("$nxKind") || stableJson(card).includes("$nxKind")) {
        throw new Error("an internal value reached canonical output");
    }
});
test("a handler for a same-named component of another module is refused naming both modules", () => {
    const ui = new ArtifactBuilder("ui/button.nx", [], "1");
    ui.record("Button.Tapped", []);
    ui.component("Button", [], [], -1, { external: true, emits: [["Tapped", ui.ref("Button.Tapped")]] });
    const preparedUi = prepareNxIrModule(ui.build());
    // `main.nx` declares a `Button` of its own and binds a handler for it on the imported one.
    const b = new ArtifactBuilder("main.nx", [{ identity: "ui/button.nx", version: "1", fingerprint: "1" }]);
    b.record("Log", []);
    b.record("Button.Tapped", []);
    b.component("Button", [], [], -1, { external: true, emits: [["Tapped", b.ref("Button.Tapped")]] });
    const log = b.node([nodeKinds.record, ...b.ref("Log"), ...b.list([]), ...b.content([])]);
    const handler = b.handler(b.ref("Button"), "Tapped", b.ref("Button.Tapped"), 0, undefined, log);
    b.fn("root", b.node([nodeKinds.component, ...b.ref("Button", "ui/button.nx"), ...b.list([b.property("onTapped", handler)]), ...b.content([])]));
    const program = linkNxIrProgram(prepareNxIrModule(b.build({ requiredFeatures: ["action-handlers-v1"] })), {
        resolve: () => preparedUi,
    });
    const refused = assertThrows(() => evaluateFunction(program, "root"), "Expected Button props.onTapped to be an action handler for ui/button.nx::Button.Tapped, got one for main.nx::Button.Tapped.");
    assertEqual(refused.diagnostics[0].code, "nx-ir-type");
});
test("initialization tokens handlers and dispatch patches the state live within a batch", () => {
    const program = prepareNxIrProgram(counterArtifact());
    const initialized = initializeComponent(program, "Counter", { step: 2 });
    assertEqual(initialized.state, { count: 0 });
    assertEqual(counterToken(initialized.rendered), "h1-1");
    assertEqual(counterToken(initialized.rendered, 1), "h1-2");
    if (!Object.isFrozen(initialized.instance) || !Object.isFrozen(initialized.instance.state)) {
        throw new Error("the instance is not frozen");
    }
    const once = dispatchComponentActions(program, initialized.instance, [tapped("h1-1")]);
    assertEqual(once.state, { count: 2 });
    assertEqual(once.effects, []);
    assertEqual(counterToken(once.rendered), "h2-1");
    const twice = dispatchComponentActions(program, once.instance, [tapped("h2-1"), tapped("h2-1")]);
    assertEqual(twice.state, { count: 6 });
    // An empty batch re-renders with fresh tokens, and the previous ones no longer name anything.
    const idle = dispatchComponentActions(program, twice.instance, []);
    assertEqual(counterToken(idle.rendered), "h4-1");
    assertEqual(idle.state, { count: 6 });
    const stale = assertThrows(() => dispatchComponentActions(program, idle.instance, [tapped("h3-1")]), "Unknown handler token 'h3-1'");
    assertEqual(stale.diagnostics[0].code, "nx-ir-handler-token");
    assertEqual(initialized.instance.state, { count: 0 });
});
test("dispatch refuses the wrong action, an action the component does not emit, and a malformed entry", () => {
    const program = prepareNxIrProgram(counterArtifact());
    const { instance } = initializeComponent(program, "Counter");
    const wrong = assertThrows(() => dispatchComponentActions(program, instance, [{ $type: "ActionHandlerInvocation", token: "h1-1", action: { $type: "Slider.EndChanged" } }]), "Expected an action of type 'Button.Tapped' for handler Button.Tapped, got 'Slider.EndChanged'");
    assertEqual(wrong.diagnostics[0].code, "nx-ir-type");
    const unknown = assertThrows(() => dispatchComponentActions(program, instance, [{ $type: "Other.Changed" }]), "Component 'Counter' does not emit 'Other.Changed'");
    assertEqual(unknown.diagnostics[0].code, "nx-ir-component-action");
    assertThrows(() => dispatchComponentActions(program, instance, [{ $type: "ActionHandlerInvocation", token: 1, action: {} }]), "string 'token'");
    // An emitted action the parent bound nothing for is checked and then a no-op.
    const noop = dispatchComponentActions(program, instance, [{ $type: "Reset" }]);
    assertEqual(noop.effects, []);
    assertEqual(noop.state, { count: 0 });
    assertEqual(counterToken(noop.rendered), "h2-1");
});
test("a failing batch is atomic and leaves the supplied instance usable", () => {
    const program = prepareNxIrProgram(counterArtifact());
    const { instance } = initializeComponent(program, "Counter", { step: 5 });
    assertThrows(() => dispatchComponentActions(program, instance, [tapped("h1-1"), tapped("h9-9")]), "Unknown handler token 'h9-9'");
    const bad = assertThrows(() => dispatchComponentActions(program, instance, [tapped("h1-1"), tapped("h1-2")]), "Expected Counter.Update.count to be a number");
    assertEqual(bad.diagnostics[0].code, "nx-ir-boundary-type");
    const retry = dispatchComponentActions(program, instance, [tapped("h1-1")]);
    assertEqual(retry.state, { count: 5 });
    assertEqual(instance.state, { count: 0 });
});
/**
 * The corpus's `handlers` program: `Page` renders a `Label`, a `TextInput`, a `Button` and two
 * `SearchBox`es inside a `Stack`.
 */
function handlersProgram() {
    const path = fileURLToPath(new URL("../../../../specs/ir-conformance/handlers/expected/main.nx.stripped.nxir", import.meta.url));
    return prepareNxIrProgram(new Uint8Array(readFileSync(path)));
}
function children(instance) {
    return fields(instance.rendered).children.map(fields);
}
/** The rendered descriptors of `type` in a list, in order. */
function ofType(list, type) {
    return list.map(fields).filter((child) => child.$type === type);
}
function searchBoxProps(descriptor) {
    const { $type: _type, ...props } = descriptor;
    return props;
}
test("a parent's handler reaches the child through the parent's instance and runs on the child's emit", () => {
    const program = handlersProgram();
    const page = initializeComponent(program, "Page");
    const [first, second] = ofType(children(page), "SearchBox");
    assertEqual(fields(first.onSearchSubmitted).token, "h1-2");
    const searchBox = initializeComponent(program, "SearchBox", searchBoxProps(first), { parent: page.instance });
    assertEqual(searchBox.rendered, { $type: "TextInput", value: "Find" });
    assertEqual(searchBox.state, { query: "Find" });
    assertEqual(Object.keys(searchBox.instance.props), ["placeholder"]);
    const submitted = dispatchComponentActions(program, searchBox.instance, [{ $type: "SearchSubmitted", searchString: "docs" }]);
    assertEqual(submitted.effects, [{ $type: "DoSearch", search: "docs" }]);
    assertEqual(submitted.state, { query: "Find" });
    // The second box's handler returns the parent's own update record, which is an effect here.
    const again = initializeComponent(program, "SearchBox", searchBoxProps(second), { parent: page.instance });
    const patched = dispatchComponentActions(program, again.instance, [{ $type: "SearchSubmitted", searchString: "docs" }]);
    assertEqual(patched.effects, [{ $type: "Page.Update", query: "docs" }]);
    assertEqual(patched.state, { query: "Again" });
    assertThrows(() => dispatchComponentActions(program, again.instance, [{ $type: "SearchSubmitted", searchString: 3 }]), "Expected SearchSubmitted action.searchString to be a string");
});
test("a handler in a content child stays the parent's and is unowned by the child that renders it", () => {
    const program = handlersProgram();
    const page = initializeComponent(program, "Page");
    const stack = initializeComponent(program, "Stack", { children: fields(page.rendered).children }, { parent: page.instance });
    // Re-rendered by the Stack instance, the handlers take that instance's tokens.
    // `<column>{children}</column>`: the one content child is the list itself.
    const [button] = ofType(fields(stack.rendered).content, "Button");
    assertEqual(fields(button.onTapped).token, "h1-1");
    const dispatched = dispatchComponentActions(program, stack.instance, [tapped("h1-1")]);
    assertEqual(dispatched.effects, [{ $type: "Page.Update", count: 1 }]);
    assertEqual(dispatched.state, {});
});
test("an ActionHandler record in props needs a parent instance and a token that instance holds", () => {
    const program = handlersProgram();
    const page = initializeComponent(program, "Page");
    const [first] = ofType(children(page), "SearchBox");
    const withoutParent = assertThrows(() => initializeComponent(program, "SearchBox", searchBoxProps(first)), "names a handler only through a parent instance");
    assertEqual(withoutParent.diagnostics[0].code, "nx-ir-boundary-field");
    const stale = assertThrows(() => initializeComponent(program, "SearchBox", { onSearchSubmitted: { $type: "ActionHandler", action: "SearchSubmitted", token: "h7-1" } }, { parent: page.instance }), "Unknown handler token 'h7-1'");
    assertEqual(stale.diagnostics[0].code, "nx-ir-handler-token");
    // A handler for another component or emit is refused where it is bound.
    const [button] = ofType(children(page), "Button");
    assertThrows(() => initializeComponent(program, "SearchBox", { onSearchSubmitted: button.onTapped }, { parent: page.instance }), "Expected SearchBox props.onSearchSubmitted to be an action handler for SearchBox.SearchSubmitted, got one for Button.Tapped");
    const instance = page.instance;
    assertEqual(instance.generation, 1);
});
test("initialization takes a complete state in place of the initial one", () => {
    const program = handlersProgram();
    const kept = initializeComponent(program, "SearchBox", { placeholder: "Find" }, { state: { query: "docs" } });
    assertEqual(kept.rendered, { $type: "TextInput", value: "docs" });
    assertEqual(kept.state, { query: "docs" });
    assertEqual(kept.instance.generation, 1);
    assertEqual(kept.instance.props, { placeholder: "Find" });
    // Re-initializing a Counter with the state it holds and new props keeps its count and renumbers.
    const counter = initializeComponent(program, "Counter");
    const stepped = dispatchComponentActions(program, counter.instance, [tapped("h1-1")]);
    const again = initializeComponent(program, "Counter", { step: 5 }, { state: stepped.state });
    assertEqual(again.state, stepped.state);
    assertEqual(fields(children(again)[1].onTapped).token, "h1-1");
    assertEqual(dispatchComponentActions(program, again.instance, [tapped("h1-1")]).state.count, 6);
    const mistyped = assertThrows(() => initializeComponent(program, "SearchBox", {}, { state: { query: 123 } }), "SearchBox state.query");
    assertEqual(mistyped.diagnostics[0].code, "nx-ir-boundary-type");
    const missing = assertThrows(() => initializeComponent(program, "SearchBox", {}, { state: {} }), "Missing required SearchBox state field 'query'");
    assertEqual(missing.diagnostics[0].code, "nx-ir-boundary-field");
});
test("a handler resolved through the parent is the parent's own value", () => {
    const program = handlersProgram();
    const page = initializeComponent(program, "Page");
    const [button] = ofType(children(page), "Button");
    const pageToken = fields(button.onTapped).token;
    const stack = initializeComponent(program, "Stack", { children: fields(page.rendered).children }, { parent: page.instance });
    const [rendered] = ofType(fields(stack.rendered).content, "Button");
    const stackToken = fields(rendered.onTapped).token;
    assertEqual(pageToken, "h1-1");
    assertEqual(stackToken, "h1-1");
    const handler = page.instance.handlers.get(pageToken);
    if (handler === undefined || stack.instance.handlers.get(stackToken) !== handler) {
        throw new Error("the Stack instance holds a copy of the Page handler rather than the handler itself");
    }
    // A host can find the instance that created a handler by looking for it in each ancestor's table.
    const searchBoxHandler = page.instance.handlers.get("h1-2");
    if (searchBoxHandler === undefined || [...stack.instance.handlers.values()].includes(searchBoxHandler) === false) {
        throw new Error("every Page handler in the content reaches the Stack instance's table");
    }
});
// ------------------------------------------------------------------------------------------------
// Runner
// ------------------------------------------------------------------------------------------------
// ------------------------------------------------------------------------------------------------
// Function values
// ------------------------------------------------------------------------------------------------
/**
 * `let <Row Item:object Index:int />: string = "r"`, `external component <List ItemTemplate?:<function
 * Item:object Index:int />: string />`, `let root() = <List ItemTemplate={Row} />`, and a
 * `component <Section Row:<function Item:object Index:int />: string /> = { <Row Item="a" Index=1 /> }`.
 */
function templateArtifact() {
    const b = new ArtifactBuilder("main.nx");
    const object = b.primitive("object");
    const int = b.primitive("int");
    const string = b.primitive("string");
    const template = b.functionType(string, [["Item", object], ["Index", int]]);
    b.fn("Row", b.string("r"), [[b.str("Item"), object, 0], [b.str("Index"), int, 0]]);
    b.component("List", [b.field("ItemTemplate", b.optional(template))], [], -1, { external: true });
    b.fn("root", b.node([nodeKinds.component, ...b.ref("List"), ...b.list([b.property("ItemTemplate", b.node([nodeKinds.reference, ...b.ref("Row")]))]), ...b.content([])]));
    // The prop is slot 0 of the component frame; the body calls it by name.
    const call = b.node([nodeKinds.namedCall, b.node([nodeKinds.slot, 0, b.str("Row")]), ...b.list([b.property("Item", b.string("a")), b.property("Index", b.int(1))])]);
    b.component("Section", [b.field("Row", template, { required: true })], [], call);
    return b.build({ requiredFeatures: ["function-values-v1"] });
}
test("prepares an image listing the function-values feature and refuses one naming a feature it does not know", () => {
    const program = prepareNxIrProgram(templateArtifact());
    assertEqual(program.entry.module.artifact.requiredFeatures, ["function-values-v1"]);
    const b = new ArtifactBuilder("main.nx");
    b.fn("root", b.int(1));
    const refused = tryPrepareNxIrModule(b.build({ requiredFeatures: ["function-values-v2"] }));
    if (refused.ok || !refused.diagnostics.some((diagnostic) => diagnostic.code === "nx-ir-required-feature" && diagnostic.message.includes("function-values-v2"))) {
        throw new Error("expected the unknown feature to be refused by name");
    }
});
test("the validator reads a function type and refuses one whose parameter type index dangles", () => {
    const program = prepareNxIrProgram(templateArtifact());
    const list = program.componentEntrypoints.get("List").kind;
    if (list.tag !== "component") {
        throw new Error("List is not a component");
    }
    assertEqual(list.props[0].ty, {
        kind: "seq",
        item: {
            kind: "function",
            params: [
                { name: "Item", ty: { kind: "primitive", name: "object" }, isContent: false, isOptional: false },
                { name: "Index", ty: { kind: "primitive", name: "int" }, isContent: false, isOptional: false },
            ],
            result: { kind: "primitive", name: "string" },
        },
        mayBeEmpty: true,
        mayBeMany: false,
    });
    const b = new ArtifactBuilder("main.nx");
    b.type([typeKinds.function, b.primitive("string"), 1, b.str("Item"), 99, 0]);
    b.fn("root", b.int(1));
    const refused = tryPrepareNxIrModule(b.build());
    if (refused.ok || !refused.diagnostics.some((diagnostic) => diagnostic.code === "nx-ir-malformed")) {
        throw new Error("expected the dangling type index to be refused");
    }
});
test("a rendered descriptor carries a function value as a Function record", () => {
    const program = prepareNxIrProgram(templateArtifact());
    const root = fields(evaluateFunction(program, "root"));
    assertEqual(root, { $type: "List", ItemTemplate: { $type: "Function", module: "main.nx", name: "Row" } });
    if (stableJson(root).includes("$nxKind")) {
        throw new Error("an internal value reached canonical output");
    }
});
test("callFunction binds by name, drops an argument the function lacks, and names a missing one", () => {
    const program = prepareNxIrProgram(templateArtifact());
    const record = fields(evaluateFunction(program, "root")).ItemTemplate;
    assertEqual(callFunction(program, record, { Item: { $type: "Contact" }, Index: 3, Extra: 1 }), "r");
    assertThrows(() => callFunction(program, record, { Item: { $type: "Contact" } }), "requires argument 'Index'");
    assertThrows(() => callFunction(program, { $type: "Function", module: "main.nx", name: "Nope" }, {}), "'Nope'");
    assertThrows(() => callFunction(program, { $type: "Function", module: "other.nx", name: "Row" }, {}), "'other.nx'");
    assertThrows(() => callFunction(program, "Row", {}), "Function record");
});
test("a Function record from the host reaches a component prop and its body calls the named function", () => {
    const program = prepareNxIrProgram(templateArtifact());
    const instance = initializeComponent(program, "Section", { Row: { $type: "Function", module: "main.nx", name: "Row" } });
    assertEqual(instance.rendered, "r");
    assertEqual(fields(constructComponentDescriptor(program, "Section", { Row: { $type: "Function", module: "main.nx", name: "Row" } })), {
        $type: "Section",
        Row: { $type: "Function", module: "main.nx", name: "Row" },
    });
    assertThrows(() => initializeComponent(program, "Section", { Row: { $type: "Function", module: "main.nx", name: "Nope" } }), "'Nope'");
    assertThrows(() => initializeComponent(program, "Section", { Row: "Row" }), "Expected Section props.Row to be a function value");
});
// ------------------------------------------------------------------------------------------------
// The built-in prelude, and iterating a range
// ------------------------------------------------------------------------------------------------
/** A module that constructs `<Range start=… end=… endInclusive=… />` through the prelude's slot. */
function rangeNode(b, start, end, endInclusive) {
    const inclusive = b.node([nodeKinds.bool, endInclusive ? 1 : 0]);
    return b.node([
        nodeKinds.record,
        b.slot(NX_PRELUDE_MODULE_IDENTITY),
        b.str("Range"),
        ...b.list([b.property("start", b.int(start)), b.property("end", b.int(end)), b.property("endInclusive", inclusive)]),
        ...b.content([]),
    ]);
}
/** `let root() = { for item[, index] in <range> { body } }`, against the built-in prelude. */
function rangeLoop(start, end, endInclusive, options = {}) {
    const b = new ArtifactBuilder("main.nx", [{ identity: NX_PRELUDE_MODULE_IDENTITY, version: NX_PRELUDE_VERSION, fingerprint: "1" }]);
    const iterable = rangeNode(b, start, end, endInclusive);
    const item = b.node([nodeKinds.slot, 0, b.str("item")]);
    const body = options.withIndex === true
        ? b.node([nodeKinds.binary, 0, item, b.node([nodeKinds.slot, 1, b.str("index")])])
        : item;
    b.fn("root", b.node([
        nodeKinds.forRange,
        0,
        b.str("item"),
        options.withIndex === true ? 1 : NX_IR_NONE,
        options.withIndex === true ? b.str("index") : NX_IR_NONE,
        iterable,
        body,
    ]));
    return b.build({ requiredFeatures: [NX_IR_REQUIRED_FEATURE_RANGES_V1] });
}
test("a self-contained program whose only other module is the prelude prepares with no resolver", () => {
    const b = new ArtifactBuilder("main.nx", [{ identity: NX_PRELUDE_MODULE_IDENTITY, version: NX_PRELUDE_VERSION, fingerprint: "1" }]);
    b.fn("root", rangeNode(b, 1, 5, false));
    const program = prepareNxIrProgram(b.build());
    assertEqual(evaluateFunction(program, "root"), { $type: "Range", start: 1, end: 5, endInclusive: false });
});
test("a host resolver that knows nothing of the prelude still links a snippet and a catalog", () => {
    const catalogModule = prepareNxIrModule(catalog("9"));
    const b = new ArtifactBuilder("input.nx", [
        { identity: "drawnui.nx", version: "9", fingerprint: "7" },
        { identity: NX_PRELUDE_MODULE_IDENTITY, version: NX_PRELUDE_VERSION, fingerprint: "1" },
    ]);
    const hi = b.string("hi");
    b.fn("label", b.node([nodeKinds.component, b.slot("drawnui.nx"), b.str("SkiaLabel"), ...b.list([b.property("Text", hi)]), ...b.content([])]));
    b.fn("span", rangeNode(b, 0, 2, true));
    // The host resolves its catalog and nothing else; the runtime serves the prelude.
    const program = linkNxIrProgram(prepareNxIrModule(b.build()), {
        resolve: (identity) => (identity === "drawnui.nx" ? catalogModule : undefined),
    });
    assertEqual(fields(evaluateFunction(program, "label"))["$type"], "SkiaLabel");
    assertEqual(evaluateFunction(program, "span"), { $type: "Range", start: 0, end: 2, endInclusive: true });
});
test("a host that resolves the prelude itself overrides the built-in one", () => {
    const own = new ArtifactBuilder(NX_PRELUDE_MODULE_IDENTITY, [], NX_PRELUDE_VERSION);
    const object = own.primitive("object");
    own.record("Range", [
        own.field("start", object, { required: true }),
        own.field("end", object, { required: true }),
        own.field("endInclusive", own.primitive("boolean"), { required: true }),
        // A field the built-in prelude does not have, so the module in use is unmistakable.
        own.field("label", own.primitive("string"), { default: own.string("host") }),
    ]);
    const hostPrelude = prepareNxIrModule(own.build());
    const b = new ArtifactBuilder("main.nx", [{ identity: NX_PRELUDE_MODULE_IDENTITY, version: NX_PRELUDE_VERSION, fingerprint: "1" }]);
    b.fn("root", rangeNode(b, 1, 5, false));
    const program = linkNxIrProgram(prepareNxIrModule(b.build()), {
        resolve: (identity) => (identity === NX_PRELUDE_MODULE_IDENTITY ? hostPrelude : undefined),
    });
    assertEqual(fields(evaluateFunction(program, "root"))["label"], "host");
});
test("an image compiled against another prelude contract fails linking by version", () => {
    // The declaration is one this runtime's prelude does hold, so only the version can catch it:
    // an image built against a `Range` of another shape names `Range` just the same.
    const b = new ArtifactBuilder("main.nx", [
        { identity: NX_PRELUDE_MODULE_IDENTITY, version: `${NX_PRELUDE_VERSION}0`, fingerprint: "1" },
    ]);
    b.fn("root", rangeNode(b, 1, 5, false));
    const linked = tryLinkNxIrProgram(prepareNxIrModule(b.build()), { resolve: () => undefined });
    assertEqual(linked.ok, false);
    if (!linked.ok) {
        assertEqual(linked.diagnostics[0].code, "nx-ir-link-version");
        assertEqual(linked.diagnostics[0].message.includes(NX_PRELUDE_MODULE_IDENTITY), true);
    }
});
test("a host that opts into version mismatch links an image from another prelude contract", () => {
    const b = new ArtifactBuilder("main.nx", [
        { identity: NX_PRELUDE_MODULE_IDENTITY, version: `${NX_PRELUDE_VERSION}0`, fingerprint: "1" },
    ]);
    b.fn("root", rangeNode(b, 1, 5, false));
    const program = linkNxIrProgram(prepareNxIrModule(b.build()), {
        resolve: () => undefined,
        allowVersionMismatch: true,
    });
    assertEqual(evaluateFunction(program, "root"), { $type: "Range", start: 1, end: 5, endInclusive: false });
});
test("a prelude declaration the runtime lacks fails linking by name", () => {
    const b = new ArtifactBuilder("main.nx", [{ identity: NX_PRELUDE_MODULE_IDENTITY, version: NX_PRELUDE_VERSION, fingerprint: "1" }]);
    b.fn("root", b.node([
        nodeKinds.record,
        b.slot(NX_PRELUDE_MODULE_IDENTITY),
        b.str("Interval"),
        ...b.list([]),
        ...b.content([]),
    ]));
    const linked = tryLinkNxIrProgram(prepareNxIrModule(b.build()), { resolve: () => undefined });
    assertEqual(linked.ok, false);
    if (!linked.ok) {
        const message = linked.diagnostics[0].message;
        assertEqual(message.includes(NX_PRELUDE_MODULE_IDENTITY), true);
        assertEqual(message.includes("Interval"), true);
    }
});
test("the range the runtime iterates is the prelude's declaration", () => {
    // `integerRange` recognizes a range by shape: the name `Range` and three field names, written as
    // literals. It has nothing else to go on — a canonical value carries `$type` as a bare name with
    // no module, and the prelude's `T` is erased to `object` in the image, so neither the declaring
    // module nor the element type is recoverable from the value. Nothing else ties those literals to
    // `prelude.nx`, so this does: rename a field there and the runtime would stop recognizing ranges
    // silently, unless this fails first.
    const b = new ArtifactBuilder("main.nx", [{ identity: NX_PRELUDE_MODULE_IDENTITY, version: NX_PRELUDE_VERSION, fingerprint: "1" }]);
    b.fn("root", rangeNode(b, 1, 5, false));
    const program = linkNxIrProgram(prepareNxIrModule(b.build()), { resolve: () => undefined });
    const prelude = program.modulesByIdentity.get(NX_PRELUDE_MODULE_IDENTITY);
    const range = prelude?.module.declarationsByName.get("Range");
    if (range === undefined || range.kind.tag !== "record") {
        throw new Error("the built-in prelude declares Range as a record");
    }
    assertEqual(range.kind.fields.map((field) => [field.name, field.ty]), [
        ["start", { kind: "primitive", name: "object" }],
        ["end", { kind: "primitive", name: "object" }],
        ["endInclusive", { kind: "primitive", name: "boolean" }],
    ]);
});
test("a half-open and a closed range iterate, and an empty one runs no body", () => {
    assertEqual(evaluateFunction(prepareNxIrProgram(rangeLoop(0, 4, false)), "root"), [0, 1, 2, 3]);
    assertEqual(evaluateFunction(prepareNxIrProgram(rangeLoop(1, 3, true)), "root"), [1, 2, 3]);
    assertEqual(evaluateFunction(prepareNxIrProgram(rangeLoop(5, 2, false)), "root"), []);
    assertEqual(evaluateFunction(prepareNxIrProgram(rangeLoop(1, 0, true)), "root"), []);
    assertEqual(evaluateFunction(prepareNxIrProgram(rangeLoop(3, 3, true)), "root"), [3]);
});
test("the index of a range loop counts from zero", () => {
    assertEqual(evaluateFunction(prepareNxIrProgram(rangeLoop(5, 8, false, { withIndex: true })), "root"), [5, 7, 9]);
});
test("a range a host supplied iterates", () => {
    const b = new ArtifactBuilder("main.nx", [{ identity: NX_PRELUDE_MODULE_IDENTITY, version: NX_PRELUDE_VERSION, fingerprint: "1" }]);
    const span = b.node([nodeKinds.slot, 0, b.str("span")]);
    const item = b.node([nodeKinds.slot, 1, b.str("item")]);
    b.fn("count", b.node([nodeKinds.forRange, 1, b.str("item"), NX_IR_NONE, NX_IR_NONE, span, item]), [[b.str("span"), b.nominal("Range", NX_PRELUDE_MODULE_IDENTITY), 0]]);
    const program = prepareNxIrProgram(b.build({ requiredFeatures: [NX_IR_REQUIRED_FEATURE_RANGES_V1] }));
    assertEqual(evaluateFunction(program, "count", [{ $type: "Range", start: 2, end: 4, endInclusive: true }]), [2, 3, 4]);
});
test("a forRange iterable that is not a range with integer bounds fails evaluation", () => {
    const b = new ArtifactBuilder("main.nx");
    const notARange = b.string("nope");
    const item = b.node([nodeKinds.slot, 0, b.str("item")]);
    b.fn("root", b.node([nodeKinds.forRange, 0, b.str("item"), NX_IR_NONE, NX_IR_NONE, notARange, item]));
    const program = prepareNxIrProgram(b.build({ requiredFeatures: [NX_IR_REQUIRED_FEATURE_RANGES_V1] }));
    assertThrows(() => evaluateFunction(program, "root"), "Range record with integer bounds");
});
/**
 * Every way a value can fail `integerRange`, reached through an `object`-typed parameter so the
 * value arrives at the loop exactly as the host wrote it. A `<Range T=…/>`-typed parameter would
 * not do: nominal normalization rejects a wrong `$type` before the loop is reached, so the
 * branches below would never run.
 */
test("every shape that is not a range with integer bounds fails the same way", () => {
    const b = new ArtifactBuilder("main.nx");
    const span = b.node([nodeKinds.slot, 0, b.str("span")]);
    const item = b.node([nodeKinds.slot, 1, b.str("item")]);
    b.fn("count", b.node([nodeKinds.forRange, 1, b.str("item"), NX_IR_NONE, NX_IR_NONE, span, item]), [[b.str("span"), b.primitive("object"), 0]]);
    const program = prepareNxIrProgram(b.build({ requiredFeatures: [NX_IR_REQUIRED_FEATURE_RANGES_V1] }));
    const refused = [
        42,
        "nope",
        [1, 2, 3],
        // An object that is not a range, and one named as some other record.
        { start: 0, end: 3, endInclusive: false },
        { $type: "Interval", start: 0, end: 3, endInclusive: false },
        // Bounds that are not integers, or are past the exactly-representable range.
        { $type: "Range", start: 0.5, end: 3, endInclusive: false },
        { $type: "Range", start: 0, end: 2.5, endInclusive: false },
        { $type: "Range", start: 0, end: Number.MAX_SAFE_INTEGER + 10, endInclusive: false },
        { $type: "Range", start: Number.NaN, end: 3, endInclusive: false },
        { $type: "Range", start: 0, end: "3", endInclusive: false },
        // `endInclusive` missing, null, or not a boolean.
        { $type: "Range", start: 0, end: 3 },
        { $type: "Range", start: 0, end: 3, endInclusive: null },
        { $type: "Range", start: 0, end: 3, endInclusive: "yes" },
    ];
    for (const value of refused) {
        const error = assertThrows(() => evaluateFunction(program, "count", [value]), "Range record with integer bounds");
        assertEqual(error.diagnostics[0].code, "nx-ir-for");
    }
    // `null` never reaches the loop: an exactly-one site, `object` included, refuses it at the boundary.
    assertEqual(assertThrows(() => evaluateFunction(program, "count", [null]), "got null").diagnostics[0].code, "nx-ir-boundary-type");
    // The shape that does work, so the refusals above are the value's doing and not the fixture's.
    assertEqual(evaluateFunction(program, "count", [{ $type: "Range", start: 2, end: 4, endInclusive: true }]), [2, 3, 4]);
});
/**
 * The count is computed once rather than compared against `end` each turn, which is what lets a
 * closed range ending at the largest exact integer terminate — `i <= end` would never go false
 * there, because `MAX_SAFE_INTEGER + 1` is `MAX_SAFE_INTEGER` as a double.
 */
test("a closed range ending at the largest exact integer terminates", () => {
    const b = new ArtifactBuilder("main.nx");
    const span = b.node([nodeKinds.slot, 0, b.str("span")]);
    const item = b.node([nodeKinds.slot, 1, b.str("item")]);
    b.fn("count", b.node([nodeKinds.forRange, 1, b.str("item"), NX_IR_NONE, NX_IR_NONE, span, item]), [[b.str("span"), b.primitive("object"), 0]]);
    const program = prepareNxIrProgram(b.build({ requiredFeatures: [NX_IR_REQUIRED_FEATURE_RANGES_V1] }));
    const top = Number.MAX_SAFE_INTEGER;
    assertEqual(evaluateFunction(program, "count", [{ $type: "Range", start: top - 2, end: top, endInclusive: true }]), [
        top - 2,
        top - 1,
        top,
    ]);
});
// The limit is a maximum, not a bound to stay under: `count > limit` refuses, so a range of exactly
// the limit runs.
test("a range of exactly the limit runs and one integer more is refused", () => {
    assertEqual(evaluateFunction(prepareNxIrProgram(rangeLoop(0, 10, false)), "root", [], { maxRangeLength: 10 }), [0, 1, 2, 3, 4, 5, 6, 7, 8, 9]);
    const error = assertThrows(() => evaluateFunction(prepareNxIrProgram(rangeLoop(0, 11, false)), "root", [], { maxRangeLength: 10 }), "maxRangeLength");
    assertEqual(error.diagnostics[0].code, "nx-ir-resource-limit");
    assertEqual(error.diagnostics[0].message.includes("11 times"), true);
});
// The limit reaches a component's rendered body, not only a function call and a state default.
test("a lowered range limit holds while a component body is evaluated", () => {
    const b = new ArtifactBuilder("main.nx", [{ identity: NX_PRELUDE_MODULE_IDENTITY, version: NX_PRELUDE_VERSION, fingerprint: "1" }]);
    const item = b.node([nodeKinds.slot, 0, b.str("item")]);
    const body = b.node([
        nodeKinds.forRange,
        0,
        b.str("item"),
        NX_IR_NONE,
        NX_IR_NONE,
        rangeNode(b, 0, 50, false),
        item,
    ]);
    b.component("Widget", [], [], body);
    const program = prepareNxIrProgram(b.build({ requiredFeatures: [NX_IR_REQUIRED_FEATURE_RANGES_V1] }));
    const error = assertThrows(() => evaluateComponent(program, "Widget", {}, {}, { maxRangeLength: 10 }), "maxRangeLength");
    assertEqual(error.diagnostics[0].code, "nx-ir-resource-limit");
    assertEqual(evaluateComponent(program, "Widget", {}, {}).rendered, Array.from({ length: 50 }, (_, index) => index));
});
// A default is program code like any other, so the host's limit holds while it is evaluated: a
// `forRange` in a state default would otherwise run under the built-in million whatever the host
// set to bound untrusted IR.
test("a lowered range limit holds while a state default is evaluated", () => {
    const b = new ArtifactBuilder("main.nx", [{ identity: NX_PRELUDE_MODULE_IDENTITY, version: NX_PRELUDE_VERSION, fingerprint: "1" }]);
    const item = b.node([nodeKinds.slot, 0, b.str("item")]);
    const items = b.node([
        nodeKinds.forRange,
        0,
        b.str("item"),
        NX_IR_NONE,
        NX_IR_NONE,
        rangeNode(b, 0, 50, false),
        item,
    ]);
    const body = b.node([nodeKinds.element, 0, b.str("div"), ...b.list([]), ...b.content([])]);
    b.component("Widget", [], [b.field("items", b.oneOrMore(b.primitive("int")), { default: items })], body);
    const program = prepareNxIrProgram(b.build({ requiredFeatures: [NX_IR_REQUIRED_FEATURE_RANGES_V1] }));
    const error = assertThrows(() => initializeComponent(program, "Widget", {}, { maxRangeLength: 10 }), "maxRangeLength");
    assertEqual(error.diagnostics[0].code, "nx-ir-resource-limit");
    assertEqual(initializeComponent(program, "Widget", {}).state.items, Array.from({ length: 50 }, (_, index) => index));
});
test("an oversized range is refused before the body runs, and a raised limit admits it", () => {
    const program = prepareNxIrProgram(rangeLoop(0, 2_000_000, false));
    const error = assertThrows(() => evaluateFunction(program, "root"), "maxRangeLength");
    assertEqual(error.diagnostics[0].code, "nx-ir-resource-limit");
    assertEqual(error.diagnostics[0].message.includes("1000000"), true);
    const raised = evaluateFunction(program, "root", [], { maxRangeLength: 2_500_000 });
    assertEqual(Array.isArray(raised) && raised.length, 2_000_000);
});
/// A runtime that does not know a feature refuses the image by naming it. `knownFeatures` is private
/// to the module — exporting a mutable set so a test could delete from it would weaken the API — so
/// the two halves are checked separately: this runtime accepts `ranges-v1`, and the refusal path
/// names the feature it does not know.
test("a feature this runtime does not know is refused by name, and ranges-v1 is one it knows", () => {
    const b = new ArtifactBuilder("main.nx");
    b.fn("root", b.int(1));
    assertEqual(tryPrepareNxIrProgram(b.build({ requiredFeatures: [NX_IR_REQUIRED_FEATURE_RANGES_V1] })).ok, true);
    const future = tryPrepareNxIrProgram(b.build({ requiredFeatures: ["ranges-v2"] }));
    assertEqual(future.ok, false);
    if (!future.ok) {
        assertEqual(future.diagnostics[0].code, "nx-ir-required-feature");
        assertEqual(future.diagnostics[0].message, "Unsupported NX IR required feature 'ranges-v2'.");
    }
});
// ------------------------------------------------------------------------------------------------
// The occurrence model: seq types, the empty value, the presence operators and the {} pattern
// ------------------------------------------------------------------------------------------------
/** The `div` (`/`) operator's number, as `docs/nx-ir-format.md` assigns it. */
const DIV = 3;
const EQ = 8;
const AND = 14;
/** `[15, ref, [[str, node]...], [node...]]`: a record construction with named properties. */
function recordNode(b, name, properties) {
    return b.node([nodeKinds.record, ...b.ref(name), ...b.list(properties.map(([key, value]) => b.property(key, value))), ...b.content([])]);
}
/** The `{}` value: an empty `array` node, which is also the `{}` match pattern. */
function emptyNode(b) {
    return b.node([nodeKinds.array, 0]);
}
/**
 * `type Person = { name:string nick?:string }`, `type Book = { title:string author?:Person
 * tags?:string+ }` and the functions of the corpus's `occurrences` program that take a `Book`, so a
 * host value drives each operator: `hasAuthor(b) = b.author?`, `authorName(b) = b.author?.name`,
 * `nick(b) = b.author?.nick`, `byline(b) = "by " + (b.author?.name ?? "anonymous")`,
 * `tagged(b) = b.tags?`, `describe(b) = if b.author is { {} => "anonymous" else => b.author.name }`,
 * plus `orFail(o?:int) = o ?? 1 / 0` and `matchInt(o?:int) = if o is { {} => "none" 1 => "one" else => "other" }`.
 */
function occurrencesArtifact() {
    const b = new ArtifactBuilder("main.nx");
    const string = b.primitive("string");
    const int = b.primitive("int");
    b.record("Person", [b.field("name", string, { required: true }), b.field("nick", b.optional(string))]);
    b.record("Book", [
        b.field("title", string, { required: true }),
        b.field("author", b.optional(b.nominal("Person"))),
        b.field("tags", b.zeroOrMore(string)),
    ]);
    const book = [b.str("b"), b.nominal("Book"), 0];
    const author = () => b.node([nodeKinds.member, b.node([nodeKinds.slot, 0, b.str("b")]), b.str("author")]);
    b.fn("hasAuthor", b.node([nodeKinds.exists, author()]), [book]);
    b.fn("authorName", b.node([nodeKinds.optionalMember, author(), b.str("name")]), [book]);
    b.fn("nick", b.node([nodeKinds.optionalMember, author(), b.str("nick")]), [book]);
    const fallback = b.node([nodeKinds.coalesce, b.node([nodeKinds.optionalMember, author(), b.str("name")]), b.string("anonymous")]);
    b.fn("byline", b.node([nodeKinds.binary, CONCAT, b.string("by "), fallback]), [book]);
    b.fn("tagged", b.node([nodeKinds.exists, b.node([nodeKinds.member, b.node([nodeKinds.slot, 0, b.str("b")]), b.str("tags")])]), [book]);
    const describe = b.node([nodeKinds.ifIs, author(), 1, 1, emptyNode(b), b.string("anonymous"), b.node([nodeKinds.member, author(), b.str("name")])]);
    b.fn("describe", describe, [book]);
    const o = [b.str("o"), b.optional(int), 2];
    const slotO = () => b.node([nodeKinds.slot, 0, b.str("o")]);
    b.fn("orFail", b.node([nodeKinds.coalesce, slotO(), b.node([nodeKinds.binary, DIV, b.int(1), b.int(0)])]), [o]);
    b.fn("matchInt", b.node([nodeKinds.ifIs, slotO(), 2, 1, emptyNode(b), b.string("none"), 1, b.int(1), b.string("one"), b.string("other")]), [o]);
    return b.build({ requiredFeatures: [NX_IR_REQUIRED_FEATURE_OCCURRENCE_V1] });
}
const ada = { $type: "Person", name: "Ada" };
const withAuthor = { $type: "Book", title: "A", author: ada, tags: ["x"] };
const withoutAuthor = { $type: "Book", title: "B" };
test("exists, optionalMember and coalesce evaluate over the empty value, and a fallback is lazy", () => {
    const program = prepareNxIrProgram(occurrencesArtifact());
    assertEqual(evaluateFunction(program, "hasAuthor", [withAuthor]), true);
    assertEqual(evaluateFunction(program, "hasAuthor", [withoutAuthor]), false);
    assertEqual(evaluateFunction(program, "authorName", [withAuthor]), "Ada");
    assertEqual(evaluateFunction(program, "authorName", [withoutAuthor]), []);
    // The step reads an optional field the record stores no key for as the empty value.
    assertEqual(evaluateFunction(program, "nick", [withAuthor]), []);
    assertEqual(evaluateFunction(program, "nick", [{ ...withAuthor, author: { ...ada, nick: "A" } }]), "A");
    assertEqual(evaluateFunction(program, "byline", [withAuthor]), "by Ada");
    assertEqual(evaluateFunction(program, "byline", [withoutAuthor]), "by anonymous");
    // A test on zero-or-more is non-emptiness, whether the host omitted the field or sent `[]`.
    assertEqual(evaluateFunction(program, "tagged", [withAuthor]), true);
    assertEqual(evaluateFunction(program, "tagged", [withoutAuthor]), false);
    assertEqual(evaluateFunction(program, "tagged", [{ ...withoutAuthor, tags: [] }]), false);
    // The right operand of `??` runs only when the left is empty.
    assertEqual(evaluateFunction(program, "orFail", [1]), 1);
    for (const empty of [null, []]) {
        assertThrows(() => evaluateFunction(program, "orFail", [empty]), "Division by zero");
    }
});
test("the {} pattern matches exactly the empty value", () => {
    const program = prepareNxIrProgram(occurrencesArtifact());
    assertEqual(evaluateFunction(program, "describe", [withAuthor]), "Ada");
    assertEqual(evaluateFunction(program, "describe", [withoutAuthor]), "anonymous");
    assertEqual(evaluateFunction(program, "matchInt", [null]), "none");
    assertEqual(evaluateFunction(program, "matchInt", [[]]), "none");
    // A value that holds an item never matches `{}`, and the empty value matches no other pattern.
    assertEqual(evaluateFunction(program, "matchInt", [1]), "one");
    assertEqual(evaluateFunction(program, "matchInt", [2]), "other");
});
test("reads a seq type and refuses the retired array and nullable kinds, the null node and a malformed seq", () => {
    const b = new ArtifactBuilder("main.nx");
    b.fn("f", b.node([nodeKinds.slot, 0, b.str("xs")]), [[b.str("xs"), b.zeroOrMore(b.primitive("int")), 0]]);
    const program = prepareNxIrProgram(b.build());
    const f = program.functionEntrypoints.get("f").kind;
    if (f.tag !== "function") {
        throw new Error("f is not a function");
    }
    assertEqual(f.params[0].ty, { kind: "seq", item: { kind: "primitive", name: "int" }, mayBeEmpty: true, mayBeMany: true });
    assertEqual(evaluateFunction(program, "f", [3]), [3]);
    const refused = (build, message) => {
        const builder = new ArtifactBuilder("main.nx");
        build(builder);
        builder.fn("root", builder.int(1));
        const result = tryPrepareNxIrModule(builder.build());
        if (result.ok || !result.diagnostics.some((diagnostic) => diagnostic.code === "nx-ir-malformed" && diagnostic.message.includes(message))) {
            throw new Error(`expected a malformed diagnostic containing '${message}', got ${result.ok ? "a prepared module" : result.diagnostics.map((diagnostic) => diagnostic.message).join(" ")}`);
        }
    };
    refused((builder) => builder.type([2, builder.primitive("int")]), "Unknown type kind 2 (retired with schema 5)");
    refused((builder) => builder.type([3, builder.primitive("int")]), "Unknown type kind 3 (retired with schema 5)");
    refused((builder) => builder.fn("nothing", builder.node([0])), "Unknown node kind 0 (retired with schema 5)");
    refused((builder) => builder.seq(builder.primitive("int"), 0), "occurrence cell 0");
    refused((builder) => builder.seq(builder.primitive("int"), 4), "occurrence cell 4");
    refused((builder) => builder.oneOrMore(builder.optional(builder.primitive("int"))), "itself a seq type");
});
test("an operator node or a {} pattern without occurrence-v1 is malformed naming the feature", () => {
    const coalesce = new ArtifactBuilder("main.nx");
    coalesce.fn("root", coalesce.node([nodeKinds.coalesce, emptyNode(coalesce), coalesce.int(1)]));
    const pattern = new ArtifactBuilder("main.nx");
    pattern.fn("root", pattern.node([nodeKinds.ifIs, pattern.int(1), 1, 1, emptyNode(pattern), pattern.int(0), pattern.int(2)]));
    for (const [builder, spelling] of [
        [coalesce, "'coalesce' node"],
        [pattern, "'{}' pattern"],
    ]) {
        const result = tryPrepareNxIrModule(builder.build());
        if (result.ok || !result.diagnostics.some((diagnostic) => diagnostic.code === "nx-ir-malformed" && diagnostic.message.includes("occurrence-v1") && diagnostic.message.includes(spelling))) {
            throw new Error(`expected the ${spelling} to be refused naming occurrence-v1`);
        }
        // With the feature listed the same image prepares and evaluates.
        assertEqual(evaluateFunction(prepareNxIrProgram(builder.build({ requiredFeatures: [NX_IR_REQUIRED_FEATURE_OCCURRENCE_V1] })), "root"), builder === coalesce ? 1 : 2);
    }
    // A runtime that does not implement a feature refuses the module by name, as for any feature.
    const future = tryPrepareNxIrProgram(coalesce.build({ requiredFeatures: ["occurrence-v2"] }));
    assertEqual(future.ok, false);
    if (!future.ok) {
        assertEqual(future.diagnostics[0].message, "Unsupported NX IR required feature 'occurrence-v2'.");
    }
});
test("host absence decodes to the empty value where zero is admitted and is rejected where one is required", () => {
    const b = new ArtifactBuilder("main.nx");
    const string = b.primitive("string");
    const props = [
        b.field("subtitle", b.optional(string)),
        b.field("tags", b.zeroOrMore(string)),
        b.field("items", b.oneOrMore(string), { required: true }),
        b.field("title", string, { required: true }),
    ];
    b.component("Card", props, [], -1, { external: true });
    const program = prepareNxIrProgram(b.build());
    const base = { title: "T", items: ["a"] };
    const expected = { $type: "Card", title: "T", items: ["a"] };
    // `null`, `[]` and a missing key are one empty value at a `?` or `*` site, carried as no key.
    assertEqual(constructComponentDescriptor(program, "Card", { ...base, subtitle: null, tags: [] }), expected);
    assertEqual(constructComponentDescriptor(program, "Card", { ...base, subtitle: [], tags: null }), expected);
    assertEqual(constructComponentDescriptor(program, "Card", base), expected);
    // A one-element array at a `?` site is its element; a longer one does not fit.
    assertEqual(fields(constructComponentDescriptor(program, "Card", { ...base, subtitle: ["S"] })).subtitle, "S");
    assertThrows(() => constructComponentDescriptor(program, "Card", { ...base, subtitle: ["S", "T"] }), "Card props.subtitle to hold at most one value");
    // A single value at a `+` or `*` site is a sequence of one.
    const lifted = fields(constructComponentDescriptor(program, "Card", { ...base, items: "a", tags: "t" }));
    assertEqual(lifted.items, ["a"]);
    assertEqual(lifted.tags, ["t"]);
    // A `+` site takes neither `null`, `[]` nor a missing key.
    for (const empty of [null, []]) {
        assertThrows(() => constructComponentDescriptor(program, "Card", { title: "T", items: empty }), "Card props.items to hold at least one value");
    }
    assertThrows(() => constructComponentDescriptor(program, "Card", { title: "T" }), "Missing required Card props field 'items'");
    // An exactly-one site refuses `null` and the empty value, and reads a one-element array as its element.
    assertThrows(() => constructComponentDescriptor(program, "Card", { ...base, title: null }), "Card props.title to hold a value, got null");
    assertThrows(() => constructComponentDescriptor(program, "Card", { ...base, title: [] }), "Card props.title to hold a value, got the empty value");
    assertEqual(fields(constructComponentDescriptor(program, "Card", { ...base, title: ["T"] })).title, "T");
});
test("an empty optional field is an omitted key, written or not, and reads as the empty value", () => {
    const b = new ArtifactBuilder("main.nx");
    const string = b.primitive("string");
    b.record("Person", [b.field("name", string, { required: true }), b.field("nick", b.optional(string))]);
    const person = (nick) => recordNode(b, "Person", [["name", b.string("Ada")], ...(nick === undefined ? [] : [["nick", nick]])]);
    b.fn("omitted", person());
    b.fn("writtenEmpty", person(emptyNode(b)));
    b.fn("written", person(b.string("A")));
    const p = [b.str("p"), b.nominal("Person"), 0];
    b.fn("nickOf", b.node([nodeKinds.member, b.node([nodeKinds.slot, 0, b.str("p")]), b.str("nick")]), [p]);
    b.fn("ageOf", b.node([nodeKinds.member, b.node([nodeKinds.slot, 0, b.str("p")]), b.str("age")]), [p]);
    b.fn("same", b.node([nodeKinds.binary, EQ, person(), person(emptyNode(b))]));
    const program = prepareNxIrProgram(b.build());
    assertEqual(evaluateFunction(program, "omitted"), { $type: "Person", name: "Ada" });
    assertEqual(evaluateFunction(program, "writtenEmpty"), { $type: "Person", name: "Ada" });
    assertEqual(evaluateFunction(program, "written"), { $type: "Person", name: "Ada", nick: "A" });
    assertEqual(evaluateFunction(program, "same"), true);
    // A declared optional field the record stores no key for reads as the empty value; a field the
    // record does not declare is still an error.
    assertEqual(evaluateFunction(program, "nickOf", [ada]), []);
    assertEqual(evaluateFunction(program, "nickOf", [{ ...ada, nick: null }]), []);
    assertEqual(evaluateFunction(program, "nickOf", [{ ...ada, nick: "A" }]), "A");
    assertThrows(() => evaluateFunction(program, "ageOf", [ada]), "does not contain member 'age'");
});
test("an evaluated update record clears only a clearable field, and apply, diff and changed carry the clearing", () => {
    const b = new ArtifactBuilder("main.nx");
    const string = b.primitive("string");
    b.record("User", [b.field("name", string, { required: true }), b.field("email", b.optional(string))]);
    b.record("User.Update", [b.field("name", string), b.field("email", b.optional(string))], { updateTarget: b.ref("User") });
    const ada = () => recordNode(b, "User", [["name", b.string("Ada")], ["email", b.string("a@b")]]);
    const intrinsic = (op, args, order = []) => b.node([nodeKinds.intrinsic, op, ...b.content(args), ...b.list(order.map((name) => [b.str(name)]))]);
    b.fn("cleared", recordNode(b, "User.Update", [["email", emptyNode(b)]]));
    b.fn("renamed", recordNode(b, "User.Update", [["name", b.string("Bo")]]));
    b.fn("uncleared", recordNode(b, "User.Update", [["name", emptyNode(b)]]));
    b.fn("applied", intrinsic(0, [ada(), recordNode(b, "User.Update", [["email", emptyNode(b)]])]));
    b.fn("merged", intrinsic(1, [recordNode(b, "User.Update", [["name", b.string("Bo"),]]), recordNode(b, "User.Update", [["email", emptyNode(b)]])]));
    b.fn("diffed", intrinsic(2, [ada(), recordNode(b, "User", [["name", b.string("Ada")]])]));
    b.fn("unchanged", intrinsic(2, [recordNode(b, "User", [["name", b.string("Ada")]]), recordNode(b, "User", [["name", b.string("Ada")], ["email", emptyNode(b)]])]));
    b.fn("keys", intrinsic(3, [recordNode(b, "User.Update", [["email", emptyNode(b)], ["name", b.string("Bo")]])], ["name", "email"]));
    const program = prepareNxIrProgram(b.build({ requiredFeatures: ["update-records-v1", "update-intrinsics-v1"] }));
    // A present empty field is carried as present, and encoded as `null`: the one place the
    // canonical encoding writes it.
    assertEqual(evaluateFunction(program, "cleared"), { $type: "User.Update", email: null });
    assertEqual(evaluateFunction(program, "renamed"), { $type: "User.Update", name: "Bo" });
    assertThrows(() => evaluateFunction(program, "uncleared"), "User.Update.name to hold a value; an update record clears a field only where the target declares it optional");
    assertEqual(evaluateFunction(program, "applied"), { $type: "User", name: "Ada" });
    assertEqual(evaluateFunction(program, "merged"), { $type: "User.Update", name: "Bo", email: null });
    assertEqual(evaluateFunction(program, "diffed"), { $type: "User.Update", email: null });
    // An omitted optional and one written empty are the same value on both sides of a diff.
    assertEqual(evaluateFunction(program, "unchanged"), { $type: "User.Update" });
    assertEqual(evaluateFunction(program, "keys"), ["name", "email"]);
});
test("a present empty value clears an optional state field and is refused for one that is not", () => {
    const b = new ArtifactBuilder("main.nx");
    const string = b.primitive("string");
    const int = b.primitive("int");
    const body = b.node([nodeKinds.element, 0, b.str("div"), ...b.list([b.property("q", b.node([nodeKinds.slot, 0, b.str("query")]))]), ...b.content([])]);
    b.component("Search", [], [b.field("query", b.optional(string)), b.field("count", int, { default: b.int(0) })], body);
    b.record("Search.Update", [b.field("query", b.optional(string)), b.field("count", int)], { updateTarget: b.ref("Search") });
    const program = prepareNxIrProgram(b.build({ requiredFeatures: ["update-records-v1"] }));
    // Optional state starts empty, which the state carries no key for and the body reads as empty.
    const initialized = initializeComponent(program, "Search");
    assertEqual(initialized.state, { count: 0 });
    assertEqual(initialized.rendered, { $type: "div", q: [] });
    assertEqual(applyComponentStatePatch(program, "Search", { query: "docs", count: 1 }, { $type: "Search.Update", query: null }), { count: 1 });
    assertEqual(applyComponentStatePatch(program, "Search", { query: "docs", count: 1 }, { query: [] }), { count: 1 });
    assertEqual(applyComponentStatePatch(program, "Search", { count: 1 }, { query: "x" }), { count: 1, query: "x" });
    for (const empty of [null, []]) {
        const refused = assertThrows(() => applyComponentStatePatch(program, "Search", { count: 1 }, { count: empty }), "Search state.count");
        assertEqual(refused.diagnostics[0].code, "nx-ir-boundary-type");
    }
    // A complete state may leave an optional field out, or send it as `null`.
    assertEqual(evaluateComponent(program, "Search", {}, { count: 2 }).rendered, { $type: "div", q: [] });
    assertEqual(normalizeComponentState(program, "Search", { count: 2, query: null }), { count: 2 });
    assertEqual(initializeComponent(program, "Search", {}, { state: { count: 2, query: "docs" } }).rendered, { $type: "div", q: "docs" });
});
test("a lone content child binds by the declared occurrence", () => {
    const b = new ArtifactBuilder("main.nx");
    b.record("Item", [b.field("n", b.primitive("int"), { required: true })]);
    const item = b.nominal("Item");
    b.component("One", [b.field("body", item, { required: true, content: true })], [], -1, { external: true });
    b.component("Maybe", [b.field("body", b.optional(item), { content: true })], [], -1, { external: true });
    b.component("Many", [b.field("items", b.oneOrMore(item), { required: true, content: true })], [], -1, { external: true });
    b.component("Any", [b.field("items", b.zeroOrMore(item), { content: true })], [], -1, { external: true });
    const child = () => recordNode(b, "Item", [["n", b.int(1)]]);
    const use = (name, children) => b.node([nodeKinds.component, ...b.ref(name), ...b.list([]), ...b.content(children)]);
    b.fn("one", use("One", [child()]));
    b.fn("maybe", use("Maybe", [child()]));
    b.fn("many", use("Many", [child()]));
    b.fn("any", use("Any", [child()]));
    b.fn("several", use("Many", [child(), child()]));
    b.fn("none", use("Any", []));
    b.fn("noneRequired", use("Many", []));
    b.fn("emptyBody", use("Any", [emptyNode(b)]));
    b.fn("emptyBodyRequired", use("Many", [emptyNode(b)]));
    const program = prepareNxIrProgram(b.build());
    const one = { $type: "Item", n: 1 };
    // One child: the child itself at an exactly-one or `?` property, a one-item array at `+` or `*`.
    assertEqual(evaluateFunction(program, "one"), { $type: "One", body: one });
    assertEqual(evaluateFunction(program, "maybe"), { $type: "Maybe", body: one });
    assertEqual(evaluateFunction(program, "many"), { $type: "Many", items: [one] });
    assertEqual(evaluateFunction(program, "any"), { $type: "Any", items: [one] });
    assertEqual(evaluateFunction(program, "several"), { $type: "Many", items: [one, one] });
    // No body leaves an optional property empty and a required one missing; a written body that
    // produced nothing is the empty value, which a `+` property cannot take.
    assertEqual(evaluateFunction(program, "none"), { $type: "Any" });
    assertThrows(() => evaluateFunction(program, "noneRequired"), "Missing required Many props field 'items'");
    assertEqual(evaluateFunction(program, "emptyBody"), { $type: "Any" });
    assertThrows(() => evaluateFunction(program, "emptyBodyRequired"), "Many props.items to hold at least one value");
    // A host's content argument binds by the same rule.
    assertEqual(fields(constructComponentDescriptor(program, "Many", {}, [one])).items, [one]);
    assertEqual(fields(constructComponentDescriptor(program, "Maybe", {}, [one])).body, one);
});
test("a for over an optional runs once over its item and not at all over the empty value", () => {
    const b = new ArtifactBuilder("main.nx");
    const int = b.primitive("int");
    const loop = b.node([
        nodeKinds.for,
        1,
        b.str("x"),
        NX_IR_NONE,
        NX_IR_NONE,
        b.node([nodeKinds.slot, 0, b.str("o")]),
        b.node([nodeKinds.binary, 2, b.node([nodeKinds.slot, 1, b.str("x")]), b.int(10)]),
    ]);
    b.fn("tenfold", loop, [[b.str("o"), b.optional(int), functionParamOptional]]);
    const program = prepareNxIrProgram(b.build());
    // The product of `?` with an exactly-one body is `?`: the body's value is the loop's value, so
    // an item stays an item rather than becoming a one-element array, as in the interpreter.
    assertEqual(evaluateFunction(program, "tenfold", [3]), 30);
    assertEqual(evaluateFunction(program, "tenfold", [[3]]), 30);
    for (const empty of [null, []]) {
        assertEqual(evaluateFunction(program, "tenfold", [empty]), []);
    }
});
test("a declared result normalizes the body's value, and an empty optional result reaches the host as null", () => {
    const b = new ArtifactBuilder("main.nx");
    const int = b.primitive("int");
    const untaken = () => b.node([nodeKinds.if, b.node([nodeKinds.bool, 0]), b.int(1), NX_IR_NONE]);
    // `let many(): int+ = { 5 }`: an item returned where `int+` is declared is a one-item sequence.
    b.fn("many", b.int(5), [], { result: b.oneOrMore(int) });
    // `let one(): int? = { [5] }`: a one-item sequence where `int?` is declared is its item.
    b.fn("one", b.node([nodeKinds.array, ...b.content([b.int(5)])]), [], { result: b.optional(int), optionalResult: true });
    // `let none(): int? = { if false { 1 } }` and the same result inferred rather than declared.
    b.fn("none", untaken(), [], { result: b.optional(int), optionalResult: true });
    b.fn("inferredNone", untaken(), [], { optionalResult: true });
    // `let noneOfMany(): int* = { if false { 1 } }` keeps `[]`: only a standalone `?` is null.
    b.fn("noneOfMany", untaken(), [], { result: b.zeroOrMore(int) });
    // `let tooFew(): int+ = { if false { 1 } }` is refused, as at any `+` site.
    b.fn("tooFew", untaken(), [], { result: b.oneOrMore(int) });
    // Inside the program the empty value stays `[]`: a caller of `none` sees it, not null.
    b.fn("caller", b.node([nodeKinds.call, b.node([nodeKinds.reference, ...b.ref("none")]), ...b.content([])]), [], {
        result: b.zeroOrMore(int),
    });
    const program = prepareNxIrProgram(b.build());
    assertEqual(evaluateFunction(program, "many"), [5]);
    assertEqual(evaluateFunction(program, "one"), 5);
    assertEqual(evaluateFunction(program, "none"), null);
    assertEqual(evaluateFunction(program, "inferredNone"), null);
    assertEqual(evaluateFunction(program, "noneOfMany"), []);
    assertThrows(() => evaluateFunction(program, "tooFew"), "return value for 'tooFew' to hold at least one value");
    assertEqual(evaluateFunction(program, "caller"), []);
    assertEqual(callFunction(program, { $type: "Function", module: "main.nx", name: "none" }), null);
});
test("every empty is one value under equality, and a condition must be a boolean", () => {
    const b = new ArtifactBuilder("main.nx");
    const untaken = () => b.node([nodeKinds.if, b.node([nodeKinds.bool, 0]), b.int(1), NX_IR_NONE]);
    const loop = b.node([nodeKinds.for, 0, b.str("x"), NX_IR_NONE, NX_IR_NONE, emptyNode(b), b.node([nodeKinds.slot, 0, b.str("x")])]);
    b.fn("same", b.node([nodeKinds.binary, EQ, emptyNode(b), untaken()]));
    b.fn("loop", b.node([nodeKinds.binary, EQ, loop, untaken()]));
    b.fn("notOne", b.node([nodeKinds.binary, EQ, emptyNode(b), b.int(1)]));
    b.fn("badIf", b.node([nodeKinds.if, b.int(1), b.int(1), NX_IR_NONE]));
    b.fn("badAnd", b.node([nodeKinds.binary, AND, b.string("x"), b.node([nodeKinds.bool, 1])]));
    b.fn("badNot", b.node([nodeKinds.unary, 1, emptyNode(b)]));
    const program = prepareNxIrProgram(b.build());
    assertEqual(evaluateFunction(program, "same"), true);
    assertEqual(evaluateFunction(program, "loop"), true);
    assertEqual(evaluateFunction(program, "notOne"), false);
    for (const name of ["badIf", "badAnd", "badNot"]) {
        assertEqual(assertThrows(() => evaluateFunction(program, name), "Expected a boolean").diagnostics[0].code, "nx-ir-type");
    }
});
test("a call by name may leave an optional parameter out, which binds the empty value", () => {
    const b = new ArtifactBuilder("main.nx");
    b.fn("f", b.node([nodeKinds.exists, b.node([nodeKinds.slot, 0, b.str("o")])]), [[b.str("o"), b.optional(b.primitive("int")), functionParamOptional]]);
    const program = prepareNxIrProgram(b.build({ requiredFeatures: [NX_IR_REQUIRED_FEATURE_OCCURRENCE_V1] }));
    const f = { $type: "Function", module: "main.nx", name: "f" };
    assertEqual(callFunction(program, f, {}), false);
    assertEqual(callFunction(program, f, { o: null }), false);
    assertEqual(callFunction(program, f, { o: 3 }), true);
    // A positional call may stop before a trailing optional parameter, or supply it empty.
    assertEqual(evaluateFunction(program, "f", [[]]), false);
    assertEqual(evaluateFunction(program, "f", []), false);
    assertThrows(() => evaluateFunction(program, "f", [1, 2]), "expected at most 1 arguments, got 2");
});
test("a parameter left out takes the default its function declares, which reads the parameters before it", () => {
    const b = new ArtifactBuilder("main.nx");
    const int = b.primitive("int");
    const a = () => b.node([nodeKinds.slot, 0, b.str("a")]);
    const total = b.node([nodeKinds.binary, 0, a(), b.node([nodeKinds.slot, 1, b.str("b")])]);
    const defaultB = b.node([nodeKinds.binary, 0, a(), b.int(10)]);
    b.fn("add", total, [[b.str("a"), int, 0], [b.str("b"), int, defaultB, 0]]);
    const add = b.node([nodeKinds.reference, ...b.ref("add")]);
    b.fn("omitted", b.node([nodeKinds.call, add, 2, b.int(1), NX_IR_NONE]));
    b.fn("trailing", b.node([nodeKinds.call, add, 1, b.int(1)]));
    b.fn("written", b.node([nodeKinds.call, add, 2, b.int(1), b.int(2)]));
    const program = prepareNxIrProgram(b.build());
    assertEqual(evaluateFunction(program, "omitted"), 12);
    assertEqual(evaluateFunction(program, "trailing"), 12);
    assertEqual(evaluateFunction(program, "written"), 3);
    assertEqual(evaluateFunction(program, "add", [5]), 20);
    assertThrows(() => evaluateFunction(program, "add", []), "requires argument 'a'");
});
let failures = 0;
for (const [name, run] of tests) {
    try {
        run();
        console.log(`ok - ${name}`);
    }
    catch (error) {
        failures += 1;
        console.log(`not ok - ${name}: ${error instanceof Error ? error.message : String(error)}`);
    }
}
if (failures > 0) {
    throw new Error(`${failures} test(s) failed`);
}
