/**
 * The runtime's own behavior: header checks, preparation, linking and boundary normalization, over
 * small artifacts built in the test. Evaluation of real programs is the corpus's job.
 */
import { readFileSync } from "node:fs";
import { fileURLToPath } from "node:url";
import { NX_IR_NONE, NX_IR_RUNTIME_ABI, NX_IR_SCHEMA_VERSION, NxIrRuntimeError, applyComponentStatePatch, applyUpdate, callFunction, changedFields, constructComponentDescriptor, declarationKinds, diffRecords, dispatchComponentActions, evaluateComponent, evaluateFunction, float32Text, initializeComponent, linkNxIrProgram, mergeUpdates, nodeKinds, normalizeComponentState, prepareNxIrModule, prepareNxIrProgram, tryLinkNxIrProgram, tryPrepareNxIrModule, tryPrepareNxIrProgram, typeKinds, } from "../src/index.js";
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
    nullable(inner) {
        return this.type([typeKinds.nullable, inner]);
    }
    /** `[4, result, [[name, type, flags]...]]`: a function type with named parameters. */
    functionType(result, params = []) {
        return this.type([
            typeKinds.function,
            result,
            params.length,
            ...params.flatMap(([name, ty, content]) => [this.str(name), ty, content === true ? 1 : 0]),
        ]);
    }
    array(element) {
        return this.type([typeKinds.array, element]);
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
    fn(name, body, params = []) {
        const index = this.declaration([declarationKinds.function, this.str(name), ...this.list(params), body]);
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
test("refuses schema 3 naming both versions, and unknown ABIs and features", () => {
    const b = new ArtifactBuilder("main.nx");
    b.fn("root", b.int(1));
    const artifact = b.build();
    assertEqual(new DataView(artifact.buffer, artifact.byteOffset).getUint32(4, true), 4);
    const old = artifact.slice();
    new DataView(old.buffer).setUint32(4, 3, true);
    const schema3 = tryPrepareNxIrModule(old);
    assertEqual(schema3.ok, false);
    if (!schema3.ok) {
        const message = schema3.diagnostics[0].message;
        assertEqual(message.includes("schema version 3"), true);
        assertEqual(message.includes("schema version 4"), true);
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
 * idle | failed { message }`, and a `Card` component with a `user: User`, `theme: Theme?`, a
 * `state: LoadState?` and content `children: object[]`, plus state `{ count: int = 0 }`.
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
        b.field("theme", b.nullable(b.nominal("Theme"))),
        b.field("load", b.nullable(b.nominal("LoadState"))),
        b.field("children", b.array(b.primitive("object")), { content: true }),
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
    assertEqual(constructComponentDescriptor(program, "Card", { user }), {
        $type: "Card",
        user,
        theme: null,
        load: null,
        children: null,
    });
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
    assertEqual(fields(constructComponentDescriptor(program, "Card", { user, load: null })).load, null);
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
test("host input for an update record keeps absent fields absent and rejects null where not nullable", () => {
    const b = new ArtifactBuilder("main.nx");
    const string = b.primitive("string");
    b.record("User", [b.field("name", string, { required: true }), b.field("email", b.nullable(string))]);
    b.record("User.Update", [b.field("name", string), b.field("email", b.nullable(string))], { updateTarget: b.ref("User") });
    b.fn("same", b.node([nodeKinds.slot, 0, b.str("patch")]), [[b.str("patch"), b.nominal("User.Update"), 0]]);
    const program = prepareNxIrProgram(b.build({ requiredFeatures: ["update-records-v1"] }));
    assertEqual(evaluateFunction(program, "same", [{ $type: "User.Update", email: null }]), { $type: "User.Update", email: null });
    assertEqual(evaluateFunction(program, "same", [{ name: "Bo" }]), { $type: "User.Update", name: "Bo" });
    assertThrows(() => evaluateFunction(program, "same", [{ name: null }]), "an update record sets a field to null only where the field is nullable");
});
test("the exported helpers apply, merge, diff and list changed fields on host-held values", () => {
    const b = new ArtifactBuilder("main.nx");
    const string = b.primitive("string");
    b.record("User", [b.field("name", string, { required: true }), b.field("email", b.nullable(string)), b.field("age", b.nullable(b.primitive("int")))]);
    b.record("User.Update", [b.field("name", string), b.field("email", b.nullable(string)), b.field("age", b.nullable(b.primitive("int")))], {
        updateTarget: b.ref("User"),
    });
    const program = prepareNxIrProgram(b.build({ requiredFeatures: ["update-records-v1"] }));
    const user = { $type: "User", name: "Ada", email: "a@b", age: 3 };
    assertEqual(applyUpdate(user, { $type: "User.Update", email: null }), { $type: "User", name: "Ada", email: null, age: 3 });
    assertEqual(mergeUpdates({ $type: "User.Update", age: null }, { $type: "User.Update", name: "Bo" }), {
        $type: "User.Update",
        age: null,
        name: "Bo",
    });
    assertEqual(diffRecords(user, { ...user, name: "Bo" }), { $type: "User.Update", name: "Bo" });
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
 * `let <Row Item:object Index:int />: string = "r"`, `external component <List ItemTemplate:(<function
 * Item:object Index:int />: string)? />`, `let root() = <List ItemTemplate={Row} />`, and a
 * `component <Section Row:<function Item:object Index:int />: string /> = { <Row Item="a" Index=1 /> }`.
 */
function templateArtifact() {
    const b = new ArtifactBuilder("main.nx");
    const object = b.primitive("object");
    const int = b.primitive("int");
    const string = b.primitive("string");
    const template = b.functionType(string, [["Item", object], ["Index", int]]);
    b.fn("Row", b.string("r"), [[b.str("Item"), object, 0], [b.str("Index"), int, 0]]);
    b.component("List", [b.field("ItemTemplate", b.nullable(template))], [], -1, { external: true });
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
        kind: "nullable",
        inner: { kind: "function", params: [{ name: "Item", ty: { kind: "primitive", name: "object" }, isContent: false }, { name: "Index", ty: { kind: "primitive", name: "int" }, isContent: false }], result: { kind: "primitive", name: "string" } },
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
    assertThrows(() => callFunction(program, record, { Item: null }), "requires argument 'Index'");
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
