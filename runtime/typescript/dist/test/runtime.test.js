/**
 * The runtime's own behavior: header checks, preparation, linking and boundary normalization, over
 * small artifacts built in the test. Evaluation of real programs is the corpus's job.
 */
import { NX_IR_NONE, NX_IR_RUNTIME_ABI, NX_IR_SCHEMA_VERSION, NxIrRuntimeError, applyComponentStatePatch, applyUpdate, changedFields, constructComponentDescriptor, declarationKinds, diffRecords, evaluateComponent, evaluateFunction, initializeComponent, linkNxIrProgram, mergeUpdates, nodeKinds, normalizeComponentState, prepareNxIrModule, prepareNxIrProgram, tryLinkNxIrProgram, tryPrepareNxIrModule, tryPrepareNxIrProgram, typeKinds, } from "../src/index.js";
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
        ]);
        this.componentEntrypoints.push(index);
        return index;
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
test("refuses schema 2 naming both versions, and unknown ABIs and features", () => {
    const b = new ArtifactBuilder("main.nx");
    b.fn("root", b.int(1));
    const artifact = b.build();
    const old = artifact.slice();
    new DataView(old.buffer).setUint32(4, 2, true);
    const schema2 = tryPrepareNxIrModule(old);
    assertEqual(schema2.ok, false);
    if (!schema2.ok) {
        const message = schema2.diagnostics[0].message;
        assertEqual(message.includes("schema version 2"), true);
        assertEqual(message.includes("schema version 3"), true);
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
 * A `$type` discriminator is a bare name, and under schema 3 a program spans several modules, so
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
// Runner
// ------------------------------------------------------------------------------------------------
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
