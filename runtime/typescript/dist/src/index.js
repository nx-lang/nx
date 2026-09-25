/**
 * The NX IR runtime: prepares schema 5 images, links them by name, and evaluates them.
 *
 * An image carries one module as flat tables of 32-bit cells over one string blob, and the runtime
 * reads it in place. `prepareNxIrModule` validates every section, offset and index of the image and
 * indexes the module's declarations by name; `linkNxIrProgram` resolves the modules the entry names
 * through a host resolver and checks versions and referenced declarations eagerly; every
 * evaluation API then takes the linked program. A prepared module is never copied by linking, so
 * one prepared catalog serves any number of programs.
 */
import { NX_PRELUDE_IMAGE_BASE64, NX_PRELUDE_VERSION } from "./prelude-image.js";
export const NX_IR_SCHEMA_VERSION = 5;
export const NX_IR_RUNTIME_ABI = "nx-ir-runtime-v2";
export const NX_IR_REQUIRED_FEATURE_UPDATE_RECORDS_V1 = "update-records-v1";
export const NX_IR_REQUIRED_FEATURE_PROPERTY_UNIONS_V1 = "property-unions-v1";
export const NX_IR_REQUIRED_FEATURE_UPDATE_INTRINSICS_V1 = "update-intrinsics-v1";
export const NX_IR_REQUIRED_FEATURE_ACTION_HANDLERS_V1 = "action-handlers-v1";
/** Function types, function references as values, and calls of function-typed values by name. */
export const NX_IR_REQUIRED_FEATURE_FUNCTION_VALUES_V1 = "function-values-v1";
/** Iteration over a range: the `forRange` node. Building a range needs no feature. */
export const NX_IR_REQUIRED_FEATURE_RANGES_V1 = "ranges-v1";
/**
 * The presence operators — `exists`, `optionalMember` and `coalesce` nodes — and the `{}` match
 * pattern. A `seq` type alone needs no feature: it is a type kind, not a node.
 */
export const NX_IR_REQUIRED_FEATURE_OCCURRENCE_V1 = "occurrence-v1";
const knownFeatures = new Set([
    NX_IR_REQUIRED_FEATURE_UPDATE_RECORDS_V1,
    NX_IR_REQUIRED_FEATURE_PROPERTY_UNIONS_V1,
    NX_IR_REQUIRED_FEATURE_UPDATE_INTRINSICS_V1,
    NX_IR_REQUIRED_FEATURE_ACTION_HANDLERS_V1,
    NX_IR_REQUIRED_FEATURE_FUNCTION_VALUES_V1,
    NX_IR_REQUIRED_FEATURE_RANGES_V1,
    NX_IR_REQUIRED_FEATURE_OCCURRENCE_V1,
]);
/** The reserved identity of the NX prelude, the module every NX module sees without an import. */
export const NX_PRELUDE_MODULE_IDENTITY = "@nx/prelude.nx";
/**
 * The prelude contract this release carries, which is what a module table that lists the prelude
 * records for it.
 *
 * <para>It names the prelude's declarations rather than its text, so it is unchanged by an edit
 * that changes no declaration. Linking compares it like any other module's version, which is how a
 * package whose built-in prelude is a different contract from the one an image was compiled against
 * is caught at the link rather than at evaluation.</para>
 */
export { NX_PRELUDE_VERSION };
/** The `$type` of the prelude's range record. Its field names are `integerRange`'s. */
const rangeTypeName = "Range";
/** The `$type` of a rendered handler, and of the batch entry that invokes one by token. */
const actionHandlerTypeName = "ActionHandler";
const handlerInvocationTypeName = "ActionHandlerInvocation";
/** The `$type` of a rendered function value: a reference to a declaration by module and name. */
const functionTypeName = "Function";
// ------------------------------------------------------------------------------------------------
// The image as the compiler writes it
// ------------------------------------------------------------------------------------------------
/** The cell value that spells an absent optional operand. */
export const NX_IR_NONE = 0xffffffff;
/** `NXIR`, read as a little-endian 32-bit integer. */
const magic = 0x5249584e;
const headerLength = 16;
const directoryEntryLength = 12;
const sectionKinds = { strings: 0, module: 1, types: 2, constants: 3, nodes: 4, declarations: 5, debug: 6 };
const tableSections = {
    types: sectionKinds.types,
    constants: sectionKinds.constants,
    nodes: sectionKinds.nodes,
    declarations: sectionKinds.declarations,
};
const decoder = new TextDecoder();
/**
 * An opened image. The tables are typed-array views over the bytes the host handed in; a string is
 * decoded the first time it is named and remembered.
 *
 * <para>`open` establishes that every section, offset array and string range lies inside the image
 * and that the string blob is UTF-8. The table entries are checked against their layouts by the
 * reader that prepares the module; after that, every index an entry holds is in range.</para>
 */
export class NxIrImage {
    schemaVersion;
    runtimeAbi;
    requiredFeatures;
    /** Slot `0` is the image's own module. */
    modules;
    functionEntrypoints;
    componentEntrypoints;
    #stringOffsets;
    #stringBytes;
    #strings;
    #tables;
    #declarationSpans;
    #nodeSpans;
    #sourceBytes;
    #source;
    constructor(schemaVersion, strings, module, tables, debug) {
        this.schemaVersion = schemaVersion;
        this.#stringOffsets = strings.offsets;
        this.#stringBytes = strings.bytes;
        this.#strings = new Array(strings.offsets.length - 1);
        this.runtimeAbi = module.runtimeAbi;
        this.requiredFeatures = module.requiredFeatures;
        this.modules = module.modules;
        this.functionEntrypoints = module.functionEntrypoints;
        this.componentEntrypoints = module.componentEntrypoints;
        this.#tables = tables;
        this.#declarationSpans = debug?.declarationSpans;
        this.#nodeSpans = debug?.nodeSpans;
        this.#sourceBytes = debug?.sourceBytes;
    }
    /**
     * Opens an image, reporting what is wrong with it as diagnostics. A view whose byte offset is
     * not a multiple of four is copied first, since the tables are read as 32-bit cells.
     */
    static open(input, diagnostics) {
        const bytes = alignedBytes(input);
        const fail = (message) => {
            diagnostics.push(diagnostic("nx-ir-malformed", `NX IR image is malformed: ${message}.`));
            return undefined;
        };
        if (bytes.byteLength < headerLength) {
            if (bytes.byteLength < 4 || cellAt(bytes, 0) !== magic) {
                diagnostics.push(diagnostic("nx-ir-format", "The input is not an NX IR image."));
                return undefined;
            }
            return fail("the header is cut short");
        }
        if (cellAt(bytes, 0) !== magic) {
            diagnostics.push(diagnostic("nx-ir-format", "The input is not an NX IR image."));
            return undefined;
        }
        const schemaVersion = cellAt(bytes, 4);
        if (schemaVersion !== NX_IR_SCHEMA_VERSION) {
            diagnostics.push(diagnostic("nx-ir-schema-version", `NX IR schema version ${schemaVersion} is not supported; this runtime reads schema version ${NX_IR_SCHEMA_VERSION}.`));
            return undefined;
        }
        const total = cellAt(bytes, 8);
        if (total !== bytes.byteLength) {
            return fail(`the header says the image is ${total} bytes but ${bytes.byteLength} were given`);
        }
        const directoryCount = cellAt(bytes, 12);
        const directoryEnd = headerLength + directoryCount * directoryEntryLength;
        if (directoryEnd > bytes.byteLength) {
            return fail(`the directory of ${directoryCount} entries does not fit the image`);
        }
        const sections = new Map();
        for (let entry = 0; entry < directoryCount; entry += 1) {
            const at = headerLength + entry * directoryEntryLength;
            const kind = cellAt(bytes, at);
            const offset = cellAt(bytes, at + 4);
            const length = cellAt(bytes, at + 8);
            if (offset % 4 !== 0 || length % 4 !== 0) {
                return fail(`section ${kind} is not four-byte aligned`);
            }
            if (offset < directoryEnd || offset + length > bytes.byteLength) {
                return fail(`section ${kind} lies outside the image`);
            }
            if (kind > sectionKinds.debug) {
                // A kind this reader does not know: skipped, so a section can be added without a schema
                // change.
                continue;
            }
            if (sections.has(kind)) {
                return fail(`section ${kind} is listed twice`);
            }
            sections.set(kind, bytes.subarray(offset, offset + length));
        }
        for (const kind of [sectionKinds.strings, sectionKinds.module, ...Object.values(tableSections)]) {
            if (!sections.has(kind)) {
                return fail(`section ${kind} is missing`);
            }
        }
        const strings = readStrings(sections.get(sectionKinds.strings), fail);
        if (strings === undefined) {
            return undefined;
        }
        const stringCount = strings.offsets.length - 1;
        const tables = {};
        for (const table of Object.keys(tableSections)) {
            const cells = readTable(sections.get(tableSections[table]), table, fail);
            if (cells === undefined) {
                return undefined;
            }
            tables[table] = cells;
        }
        const module = readModule(sections.get(sectionKinds.module), stringCount, tables.declarations.count, fail);
        if (module === undefined) {
            return undefined;
        }
        const stringAt = (index) => {
            const start = strings.offsets[index];
            return decoder.decode(strings.bytes.subarray(start, strings.offsets[index + 1]));
        };
        const debugSection = sections.get(sectionKinds.debug);
        const debug = debugSection === undefined
            ? undefined
            : readDebug(debugSection, tables.declarations.count, tables.nodes.count, fail);
        if (debugSection !== undefined && debug === undefined) {
            return undefined;
        }
        return new NxIrImage(schemaVersion, strings, {
            runtimeAbi: stringAt(module.runtimeAbi),
            requiredFeatures: Array.from(module.features, stringAt),
            modules: Array.from({ length: module.modules.length / 4 }, (_, slot) => ({
                identity: stringAt(module.modules[slot * 4]),
                version: stringAt(module.modules[slot * 4 + 1]),
                fingerprint: (BigInt(module.modules[slot * 4 + 2]) |
                    (BigInt(module.modules[slot * 4 + 3]) << 32n)).toString(),
            })),
            functionEntrypoints: module.functionEntrypoints,
            componentEntrypoints: module.componentEntrypoints,
        }, tables, debug);
    }
    /** The number of strings in the string table. */
    get stringCount() {
        return this.#strings.length;
    }
    /** String `index`, decoded on first use. The index must be inside the table. */
    string(index) {
        let value = this.#strings[index];
        if (value === undefined) {
            const start = this.#stringOffsets[index];
            value = decoder.decode(this.#stringBytes.subarray(start, this.#stringOffsets[index + 1]));
            this.#strings[index] = value;
        }
        return value;
    }
    /** The number of entries in `table`. */
    entryCount(table) {
        return this.#tables[table].count;
    }
    /** Entry `index` of `table`, kind cell first, as a view over the pool. The index must be inside the table. */
    entry(table, index) {
        const { offsets, pool } = this.#tables[table];
        return pool.subarray(offsets[index], offsets[index + 1]);
    }
    /** Whether the image carries its debug section. */
    get hasDebug() {
        return this.#sourceBytes !== undefined;
    }
    /** The module's source text, when the debug section is present. */
    get source() {
        if (this.#source === undefined && this.#sourceBytes !== undefined) {
            this.#source = decoder.decode(this.#sourceBytes);
        }
        return this.#source;
    }
    /** The byte span of declaration `index` in the source, when the debug section records one. */
    declarationSpan(index) {
        return spanAt(this.#declarationSpans, index);
    }
    /** The byte span of node `index` in the source, when the debug section records one. */
    nodeSpan(index) {
        return spanAt(this.#nodeSpans, index);
    }
}
function alignedBytes(input) {
    if (input instanceof ArrayBuffer) {
        return new Uint8Array(input);
    }
    if (input.byteOffset % 4 === 0) {
        return input;
    }
    return input.slice();
}
function cellAt(bytes, offset) {
    return (bytes[offset] | (bytes[offset + 1] << 8) | (bytes[offset + 2] << 16) | (bytes[offset + 3] << 24)) >>> 0;
}
function cellsOf(bytes, start = 0, end = bytes.byteLength) {
    return new Uint32Array(bytes.buffer, bytes.byteOffset + start, (end - start) / 4);
}
function spanAt(spans, index) {
    if (spans === undefined) {
        return undefined;
    }
    const start = spans[index * 2];
    const end = spans[index * 2 + 1];
    if (start === undefined || end === undefined || start === NX_IR_NONE || end === NX_IR_NONE) {
        return undefined;
    }
    return [start, end];
}
/** Reads `count` and the `count + 1` offsets heading an offset-array section, and returns the cells after them. */
function readOffsets(bytes, what, fail) {
    if (bytes.byteLength < 4) {
        return fail(`the ${what} section is empty`);
    }
    const count = cellAt(bytes, 0);
    const restStart = 4 + (count + 1) * 4;
    if (restStart > bytes.byteLength) {
        return fail(`the ${what} offsets do not fit their section`);
    }
    const offsets = cellsOf(bytes, 4, restStart);
    if (offsets[0] !== 0) {
        return fail(`the ${what} offsets do not start at zero`);
    }
    for (let index = 1; index < offsets.length; index += 1) {
        if (offsets[index] < offsets[index - 1]) {
            return fail(`the ${what} offsets decrease`);
        }
    }
    return { offsets, restStart };
}
function readStrings(bytes, fail) {
    const head = readOffsets(bytes, "string", fail);
    if (head === undefined) {
        return undefined;
    }
    const blobLength = head.offsets[head.offsets.length - 1];
    const padded = Math.ceil(blobLength / 4) * 4;
    if (head.restStart + padded !== bytes.byteLength) {
        return fail(`the string blob is ${blobLength} bytes but its section leaves ${bytes.byteLength - head.restStart} for it`);
    }
    const blob = bytes.subarray(head.restStart, head.restStart + blobLength);
    try {
        new TextDecoder("utf-8", { fatal: true }).decode(blob);
    }
    catch {
        return fail("the string blob is not UTF-8");
    }
    for (const offset of head.offsets) {
        // A continuation byte is 0b10xxxxxx; an offset on one is inside a character.
        if (offset < blobLength && (blob[offset] & 0xc0) === 0x80) {
            return fail(`string offset ${offset} is inside a character`);
        }
    }
    return { offsets: head.offsets, bytes: blob };
}
function readTable(bytes, table, fail) {
    const head = readOffsets(bytes, table, fail);
    if (head === undefined) {
        return undefined;
    }
    const pool = cellsOf(bytes, head.restStart);
    const end = head.offsets[head.offsets.length - 1];
    if (end !== pool.length) {
        return fail(`the ${table} pool is ${pool.length} cells but its offsets end at ${end}`);
    }
    return { count: head.offsets.length - 1, offsets: head.offsets, pool };
}
function readModule(bytes, stringCount, declarationCount, fail) {
    const cells = cellsOf(bytes);
    let position = 0;
    const next = () => cells[position++];
    const check = (what, index, count) => {
        if (index === undefined || index >= count) {
            fail(`${what} index ${String(index)} is out of range (the table has ${count})`);
            return false;
        }
        return true;
    };
    const list = (what, count) => {
        const length = next();
        if (length === undefined || position + length > cells.length) {
            return fail(`the ${what} list does not fit the module section`);
        }
        const items = cells.subarray(position, position + length);
        position += length;
        for (const item of items) {
            if (!check(what, item, count)) {
                return undefined;
            }
        }
        return items;
    };
    const runtimeAbi = next();
    if (!check("runtime ABI string", runtimeAbi, stringCount)) {
        return undefined;
    }
    const features = list("required feature string", stringCount);
    if (features === undefined) {
        return undefined;
    }
    const moduleCount = next();
    if (moduleCount === undefined || moduleCount === 0) {
        return fail("the module table is empty");
    }
    if (position + moduleCount * 4 > cells.length) {
        return fail("the module table does not fit the module section");
    }
    const modules = cells.subarray(position, position + moduleCount * 4);
    position += moduleCount * 4;
    for (let slot = 0; slot < moduleCount; slot += 1) {
        if (!check("module identity string", modules[slot * 4], stringCount) ||
            !check("module version string", modules[slot * 4 + 1], stringCount)) {
            return undefined;
        }
    }
    const functionEntrypoints = list("function entrypoint declaration", declarationCount);
    const componentEntrypoints = functionEntrypoints === undefined ? undefined : list("component entrypoint declaration", declarationCount);
    if (functionEntrypoints === undefined || componentEntrypoints === undefined) {
        return undefined;
    }
    if (position !== cells.length) {
        return fail("the module section has cells after the entrypoints");
    }
    return { runtimeAbi: runtimeAbi, features, modules, functionEntrypoints, componentEntrypoints };
}
function readDebug(bytes, declarationCount, nodeCount, fail) {
    const cells = cellsOf(bytes);
    let position = 0;
    const spans = (what, expected) => {
        const count = cells[position++];
        if (count !== expected) {
            return fail(`the debug section has ${String(count)} ${what} spans for ${expected} ${what}s`);
        }
        if (position + count * 2 > cells.length) {
            return fail(`the ${what} spans do not fit the debug section`);
        }
        const items = cells.subarray(position, position + count * 2);
        position += count * 2;
        return items;
    };
    const declarationSpans = spans("declaration", declarationCount);
    const nodeSpans = declarationSpans === undefined ? undefined : spans("node", nodeCount);
    if (declarationSpans === undefined || nodeSpans === undefined) {
        return undefined;
    }
    const sourceLength = cells[position++];
    const sourceStart = position * 4;
    if (sourceLength === undefined || sourceStart + Math.ceil(sourceLength / 4) * 4 !== bytes.byteLength) {
        return fail("the source does not fit the debug section");
    }
    const sourceBytes = bytes.subarray(sourceStart, sourceStart + sourceLength);
    try {
        new TextDecoder("utf-8", { fatal: true }).decode(sourceBytes);
    }
    catch {
        return fail("the source is not UTF-8");
    }
    return { declarationSpans, nodeSpans, sourceBytes };
}
/**
 * The kind numbers of schema 5, as `docs/nx-ir-format.md` assigns them. Kind `0`, the `null`
 * node, was retired with schema 5: the language has no null value, so a reader reports the kind
 * as malformed rather than evaluating it.
 */
export const nodeKinds = {
    bool: 1,
    string: 2,
    number: 3,
    slot: 4,
    reference: 5,
    binary: 6,
    unary: 7,
    call: 8,
    intrinsic: 9,
    if: 10,
    ifIs: 11,
    array: 12,
    for: 13,
    member: 14,
    record: 15,
    unionCase: 16,
    element: 17,
    component: 18,
    actionHandler: 19,
    text: 20,
    namedCall: 21,
    forRange: 22,
    /** `x?`: `[23, operand]`, true when the operand holds at least one item. */
    exists: 23,
    /** `x?.m`: `[24, receiver, str]`, the empty value when the receiver is empty and `x.m` otherwise. */
    optionalMember: 24,
    /** `x ?? y`: `[25, left, right]`, the left operand when it holds an item and the right one otherwise. */
    coalesce: 25,
};
/**
 * The start and the number of integers of a `Range` record, or `undefined` when the value is not one.
 *
 * <para>The count is computed once, so a closed range ending at the largest exact integer terminates,
 * and it is checked against the host's limit before the loop body runs at all. Bounds must be safe
 * integers: a `Range` of another type is constructible in NX but does not iterate, and the checker
 * has already refused one, so a non-integer range here came from a host.</para>
 *
 * <para>A range is recognized by shape, and that is the whole of what the value offers. A canonical
 * value carries `$type` as a bare declaration name with no module — which is why
 * `nominalShapesFor` can answer with more than one shape — and the prelude's `T` is erased to
 * `object` in the image, so neither the declaring module nor the element type is recoverable here.
 * Provenance is decided where it can be: the checker emits a `forRange` node only for a range whose
 * declaration is the prelude's, and `normalizeNominalValue` resolves a host value's site through
 * that module-qualified slot. What is left is a host handing a look-alike to a range-typed site,
 * which is the same exposure every record of a shared name has. `the range the runtime iterates is
 * the prelude's declaration` holds the names below to `prelude.nx`.</para>
 *
 * <para>One consequence is visible only in JavaScript: `4.0` and `4` are one value, so a
 * `<Range T=float64/>` with integral bounds iterates here where the interpreter, which has a
 * separate float, refuses it. Checked NX cannot reach that one — `range-not-iterable` refuses a
 * non-integer range iterable outright — so it takes a host value or a hand-built image. The item's
 * carrier is a separate matter and checked NX does reach it: items bind as plain numbers, so
 * `int32` arithmetic over them widens here and wraps under the interpreter. That divergence belongs
 * to `int32` rather than to ranges — it is the same for any `int32` in this backend — and
 * `docs/nx-ir-format.md` records it.</para>
 */
function integerRange(value) {
    if (!isObject(value)) {
        return undefined;
    }
    const record = value;
    if (record["$type"] !== rangeTypeName) {
        return undefined;
    }
    const start = record["start"];
    const end = record["end"];
    const endInclusive = record["endInclusive"];
    if (typeof start !== "number" ||
        typeof end !== "number" ||
        typeof endInclusive !== "boolean" ||
        !Number.isSafeInteger(start) ||
        !Number.isSafeInteger(end)) {
        return undefined;
    }
    const count = end > start ? end - start + (endInclusive ? 1 : 0) : end === start && endInclusive ? 1 : 0;
    return { start, count };
}
/** The primitive types a `text` node can name: the ones with a canonical text form. */
const textTypes = ["int", "int32", "int64", "float32", "float64", "boolean"];
/**
 * The type kinds. `2` (`array`) and `3` (`nullable`) were retired with schema 5, replaced by
 * `seq`, and stay assigned so no later kind reuses them; a reader reports either as malformed.
 */
export const typeKinds = { primitive: 0, nominal: 1, function: 4, seq: 5 };
/**
 * The bits of a `seq` type's occurrence cell: whether the type admits no value and whether it
 * admits more than one. `?` is `1`, `+` is `2` and `*` is `3`; exactly one is never a `seq`.
 */
export const occurrenceFlags = { empty: 1, many: 2 };
/** The bits of a function type parameter's flags cell: takes body content; is optional (`p?:T`). */
const functionParamFlags = { content: 1, optional: 2 };
/**
 * The bits of a function declaration's result flags: the result type, declared or inferred, is a
 * standalone `T?`, so an entry call returns an empty result to the host as `null`.
 */
const functionResultFlags = { optional: 1 };
export const constantKinds = { int: 0, bigint: 1, float: 2 };
export const declarationKinds = {
    function: 0,
    value: 1,
    record: 2,
    component: 3,
    union: 4,
    typeAlias: 5,
};
const binaryOperators = [
    "add",
    "sub",
    "mul",
    "div",
    "idiv",
    "mod",
    "imod",
    "concat",
    "eq",
    "ne",
    "lt",
    "le",
    "gt",
    "ge",
    "and",
    "or",
    "fadd32",
    "fsub32",
    "fmul32",
    "fdiv32",
];
const unaryOperators = ["neg", "not"];
const intrinsicNames = ["apply", "merge", "diff", "changed"];
const fieldFlags = { content: 1, required: 2 };
const componentFlags = { abstract: 1, external: 2 };
export class NxIrRuntimeError extends Error {
    diagnostics;
    constructor(diagnostics) {
        super(diagnostics.map((diagnostic) => diagnostic.message).join("; "));
        this.name = "NxIrRuntimeError";
        this.diagnostics = diagnostics;
    }
}
/** Whether a site of this type admits the empty value: a `?` or `*` occurrence. */
function admitsEmpty(ty) {
    return ty.kind === "seq" && ty.mayBeEmpty;
}
/** Whether a site of this type holds an array: a `+` or `*` occurrence. */
function admitsMany(ty) {
    return ty.kind === "seq" && ty.mayBeMany;
}
/** The top type, which holds any value — a sequence included, opaquely. */
function isObjectType(ty) {
    return ty.kind === "primitive" && ty.name === "object";
}
/** The default of {@link NxRuntimeOptions.maxRangeLength}. */
export const NX_DEFAULT_MAX_RANGE_LENGTH = 1_000_000;
// ------------------------------------------------------------------------------------------------
// Preparation
// ------------------------------------------------------------------------------------------------
export function prepareNxIrModule(input) {
    const result = tryPrepareNxIrModule(input);
    if (!result.ok) {
        throw new NxIrRuntimeError(result.diagnostics);
    }
    return result.value;
}
/**
 * Opens and validates an image and indexes its module. A view whose byte offset is not a multiple
 * of four is copied first; an `ArrayBuffer` or a fresh `Uint8Array` is read in place.
 */
export function tryPrepareNxIrModule(input) {
    const diagnostics = [];
    const image = NxIrImage.open(input, diagnostics);
    if (image === undefined) {
        return { ok: false, diagnostics };
    }
    if (image.runtimeAbi !== NX_IR_RUNTIME_ABI) {
        diagnostics.push(diagnostic("nx-ir-runtime-abi", `NX IR runtime ABI '${image.runtimeAbi}' is not supported; this runtime implements '${NX_IR_RUNTIME_ABI}'.`));
    }
    for (const feature of image.requiredFeatures) {
        if (!knownFeatures.has(feature)) {
            diagnostics.push(diagnostic("nx-ir-required-feature", `Unsupported NX IR required feature '${feature}'.`));
        }
    }
    if (diagnostics.length > 0) {
        return { ok: false, diagnostics };
    }
    const reader = new TableReader(image, diagnostics);
    reader.validate();
    if (diagnostics.length > 0) {
        return { ok: false, diagnostics };
    }
    const own = image.modules[0];
    const declarations = [];
    const declarationsByName = new Map();
    const module = {
        identity: own.identity,
        version: own.version,
        fingerprint: own.fingerprint,
        artifact: image,
        declarations,
        declarationsByName,
        functionEntrypoints: new Map(),
        componentEntrypoints: new Map(),
        externalReferences: reader.externalReferences,
        nominalShapeSkeletons: new Map(),
    };
    for (let index = 0; index < image.entryCount("declarations"); index += 1) {
        const declaration = reader.declaration(index, module);
        declarations.push(declaration);
        if (declarationsByName.has(declaration.name)) {
            diagnostics.push(diagnostic("nx-ir-duplicate-declaration", `Module '${own.identity}' declares '${declaration.name}' twice.`));
        }
        declarationsByName.set(declaration.name, declaration);
    }
    for (const [slot, names] of reader.localReferences) {
        if (slot !== 0) {
            continue;
        }
        for (const name of names) {
            if (!declarationsByName.has(name)) {
                diagnostics.push(diagnostic("nx-ir-reference", `Module '${own.identity}' references its own declaration '${name}', which it does not declare.`));
            }
        }
    }
    const entrypoints = (indices, tag, target) => {
        for (const index of indices) {
            const declaration = declarations[index];
            if (declaration === undefined || declaration.kind.tag !== tag) {
                diagnostics.push(diagnostic("nx-ir-entrypoint", `${tag} entrypoint ${index} is invalid.`));
                continue;
            }
            target.set(declaration.name, declaration);
        }
    };
    entrypoints(image.functionEntrypoints, "function", module.functionEntrypoints);
    entrypoints(image.componentEntrypoints, "component", module.componentEntrypoints);
    indexNominalShapes(declarations, module.nominalShapeSkeletons);
    if (diagnostics.length > 0) {
        return { ok: false, diagnostics };
    }
    return { ok: true, value: module };
}
/**
 * Indexes every nominal shape a module declares by the `$type` its values carry.
 *
 * An abstract record is indexed too, even though nothing may be an instance of one: a value that
 * names one is a value to reject, and rejecting it by name reads better than reporting it as a type
 * the program does not have. A union contributes one entry per non-constant case, under
 * `Union.case`; a constant case is a bare string with no schema to normalize.
 */
function indexNominalShapes(declarations, target) {
    const add = (skeleton) => {
        const existing = target.get(skeleton.discriminator);
        if (existing === undefined) {
            target.set(skeleton.discriminator, [skeleton]);
        }
        else {
            existing.push(skeleton);
        }
    };
    for (const declaration of declarations) {
        const kind = declaration.kind;
        if (kind.tag === "record") {
            add({
                discriminator: declaration.name,
                declarationName: declaration.name,
                fields: kind.fields,
                bases: kind.bases,
                isAbstract: kind.isAbstract,
            });
        }
        else if (kind.tag === "union") {
            for (const unionCase of kind.cases) {
                if (unionCase.isConstant) {
                    continue;
                }
                add({
                    discriminator: `${declaration.name}.${unionCase.name}`,
                    declarationName: declaration.name,
                    fields: unionCase.fields,
                    bases: kind.bases,
                    isAbstract: false,
                });
            }
        }
    }
}
const NODES = ["node"];
const OPT_NODES = ["optNode"];
const STRS = ["str"];
const REFS = ["ref"];
const PROPERTY = ["str", "node"];
const FIELD = ["str", "type", "optNode", "int"];
const PARAM = ["str", "type", "int"];
const DECLARED_PARAM = ["str", "type", "optNode", "int"];
const ARM = [{ list: NODES }, "node"];
const UNION_CASE = ["str", { list: FIELD }, "int"];
const EMIT = ["str", "ref"];
/**
 * The operands after the kind cell, by table and kind number. A hole is a kind number that is
 * assigned but retired — the `array` and `nullable` types and the `null` node of schema 4 — so an
 * entry of that kind is reported as malformed rather than laid out.
 */
const layouts = {
    types: [["str"], ["ref"], undefined, undefined, ["type", { list: PARAM }], ["itemType", "occurrence"]],
    constants: [["i64"], ["str"], ["f64"]],
    nodes: [
        undefined,
        ["int"],
        ["str"],
        ["const"],
        ["int", "str"],
        ["ref"],
        ["binary", "node", "node"],
        ["unary", "node"],
        ["node", { list: OPT_NODES }],
        ["intrinsic", { list: NODES }, { list: STRS }],
        ["node", "node", "optNode"],
        ["node", { list: ARM }, "optNode"],
        [{ list: NODES }],
        ["int", "str", "optSlot", "optStr", "node", "node"],
        ["node", "str"],
        ["ref", { list: PROPERTY }, { list: NODES }],
        ["ref", "str", { list: PROPERTY }, { list: NODES }],
        ["int", "str", { list: PROPERTY }, { list: NODES }],
        ["ref", { list: PROPERTY }, { list: NODES }],
        ["ref", "str", "ref", "int", "optRef", "node"],
        ["node", "textType"],
        ["node", { list: PROPERTY }],
        // A range loop has a `for`'s layout; only what its iterable evaluates to differs.
        ["int", "str", "optSlot", "optStr", "node", "node"],
        ["node"],
        ["node", "str"],
        ["node", "node"],
    ],
    declarations: [
        ["str", { list: DECLARED_PARAM }, "node", "optType", "int"],
        ["str", "node", "optType"],
        ["str", { list: FIELD }, { list: REFS }, "int", "optRef"],
        ["str", { list: FIELD }, { list: FIELD }, "optNode", "int", { list: EMIT }],
        ["str", { list: UNION_CASE }, { list: REFS }, "optRef"],
        ["str"],
    ],
};
/** A position in one entry's cells. */
class Cursor {
    cells;
    position = 0;
    constructor(cells) {
        this.cells = cells;
    }
    /** The next cell. After validation every read is inside the entry. */
    next() {
        return this.cells[this.position++];
    }
    get finished() {
        return this.position === this.cells.length;
    }
}
/**
 * Checks every entry of every table against its layout, reporting each malformed entry as a
 * diagnostic, and then decodes declarations from cells the check has passed.
 */
class TableReader {
    externalReferences = new Map();
    localReferences = new Map();
    #image;
    #diagnostics;
    #types = new Map();
    constructor(image, diagnostics) {
        this.#image = image;
        this.#diagnostics = diagnostics;
    }
    #fail(message) {
        this.#diagnostics.push(diagnostic("nx-ir-malformed", message));
        return false;
    }
    /** Validates every table. Afterwards every index any entry holds is inside the table it names. */
    validate() {
        for (const table of Object.keys(layouts)) {
            for (let index = 0; index < this.#image.entryCount(table); index += 1) {
                this.#validateEntry(table, index);
            }
        }
    }
    #validateEntry(table, index) {
        const what = `${table.replace(/s$/, "")} ${index}`;
        const entry = this.#image.entry(table, index);
        if (entry.length === 0) {
            this.#fail(`${what} is empty.`);
            return;
        }
        const cursor = new Cursor(entry);
        const kind = cursor.next();
        const layout = layouts[table][kind];
        if (layout === undefined) {
            const retired = (table === "types" && (kind === 2 || kind === 3)) || (table === "nodes" && kind === 0)
                ? ` (retired with schema ${NX_IR_SCHEMA_VERSION})`
                : "";
            this.#fail(`Unknown ${table.replace(/s$/, "")} kind ${kind}${retired} at ${what}.`);
            return;
        }
        // A node's children precede it, and a type's inner type precedes it, so a reader that walks
        // by index never needs to look ahead and no entry can reach itself.
        const bounds = {
            nodes: table === "nodes" ? index : this.#image.entryCount("nodes"),
            types: table === "types" ? index : this.#image.entryCount("types"),
        };
        if (!this.#validateSeq(cursor, layout, bounds, what)) {
            return;
        }
        if (!cursor.finished) {
            this.#fail(`${what} has ${cursor.cells.length - cursor.position} cell(s) more than its layout.`);
            return;
        }
        if (table === "nodes") {
            this.#validateOccurrenceFeature(entry, kind, what);
        }
    }
    /**
     * The presence operators and the `{}` pattern are nodes a runtime that predates them has never
     * seen, so a module that carries one lists `occurrence-v1`; one that carries one without listing
     * it is malformed, and says which feature it should have named.
     */
    #validateOccurrenceFeature(entry, kind, what) {
        if (this.#image.requiredFeatures.includes(NX_IR_REQUIRED_FEATURE_OCCURRENCE_V1)) {
            return;
        }
        const operator = kind === nodeKinds.exists ? "exists" : kind === nodeKinds.optionalMember ? "optionalMember" : kind === nodeKinds.coalesce ? "coalesce" : undefined;
        if (operator !== undefined) {
            this.#fail(`${what} is an '${operator}' node, which requires the feature '${NX_IR_REQUIRED_FEATURE_OCCURRENCE_V1}' the module does not list.`);
            return;
        }
        if (kind !== nodeKinds.ifIs) {
            return;
        }
        // `[11, node, [[[node...], node]...], node?]`: each arm's patterns, looking for an empty
        // `array` node, which is the `{}` pattern.
        const arms = entry[2];
        let position = 3;
        for (let arm = 0; arm < arms; arm += 1) {
            const patterns = entry[position];
            for (let pattern = position + 1; pattern < position + 1 + patterns; pattern += 1) {
                const node = this.#image.entry("nodes", entry[pattern]);
                if (node.length === 2 && node[0] === nodeKinds.array && node[1] === 0) {
                    this.#fail(`${what} matches the '{}' pattern, which requires the feature '${NX_IR_REQUIRED_FEATURE_OCCURRENCE_V1}' the module does not list.`);
                    return;
                }
            }
            position += patterns + 2;
        }
    }
    #validateSeq(cursor, ops, bounds, what) {
        for (const op of ops) {
            if (!this.#validateOp(cursor, op, bounds, what)) {
                return false;
            }
        }
        return true;
    }
    #validateOp(cursor, op, bounds, what) {
        const cell = cursor.cells[cursor.position++];
        if (cell === undefined) {
            return this.#fail(`${what} ends before its layout does.`);
        }
        const inRange = (name, index, count) => index < count ? true : this.#fail(`${what} ${name} index ${index} is out of range.`);
        if (typeof op === "object") {
            for (let element = 0; element < cell; element += 1) {
                if (!this.#validateSeq(cursor, op.list, bounds, what)) {
                    return false;
                }
            }
            return true;
        }
        switch (op) {
            case "int":
            case "optSlot":
                return true;
            case "binary":
                return binaryOperators[cell] !== undefined || this.#fail(`${what} uses unknown binary operator ${cell}.`);
            case "unary":
                return unaryOperators[cell] !== undefined || this.#fail(`${what} uses unknown unary operator ${cell}.`);
            case "intrinsic":
                return intrinsicNames[cell] !== undefined || this.#fail(`${what} uses unknown intrinsic ${cell}.`);
            case "str":
                return inRange("string", cell, this.#image.stringCount);
            case "textType": {
                if (!inRange("string", cell, this.#image.stringCount)) {
                    return false;
                }
                const name = this.#image.string(cell);
                return (textTypes.includes(name) ||
                    this.#fail(`${what} names '${name}', which is not a primitive type with a text form.`));
            }
            case "optStr":
                return cell === NX_IR_NONE || inRange("string", cell, this.#image.stringCount);
            case "type":
                return inRange("type", cell, bounds.types);
            case "itemType": {
                // A `seq` type's item is an exactly-one type: no suffixed type is an item type.
                if (!inRange("type", cell, bounds.types)) {
                    return false;
                }
                return (this.#image.entry("types", cell)[0] !== typeKinds.seq ||
                    this.#fail(`${what} is a seq type whose item type ${cell} is itself a seq type.`));
            }
            case "occurrence":
                return ((cell >= 1 && cell <= (occurrenceFlags.empty | occurrenceFlags.many)) ||
                    this.#fail(`${what} carries the occurrence cell ${cell}, which spells no suffix.`));
            case "const":
                return inRange("constant", cell, this.#image.entryCount("constants"));
            case "node":
                return inRange("node", cell, bounds.nodes);
            case "optNode":
                return cell === NX_IR_NONE || inRange("node", cell, bounds.nodes);
            case "optType":
                return cell === NX_IR_NONE || inRange("type", cell, bounds.types);
            case "ref":
            case "optRef": {
                const name = cursor.cells[cursor.position++];
                if (name === undefined) {
                    return this.#fail(`${what} ends before its layout does.`);
                }
                if (op === "optRef" && cell === NX_IR_NONE) {
                    return true;
                }
                if (!inRange("module slot", cell, this.#image.modules.length) || !inRange("string", name, this.#image.stringCount)) {
                    return false;
                }
                this.#reference(cell, this.#image.string(name));
                return true;
            }
            case "i64":
            case "f64":
                if (cursor.cells[cursor.position++] === undefined) {
                    return this.#fail(`${what} ends before its layout does.`);
                }
                return true;
        }
    }
    #reference(slot, name) {
        const table = slot === 0 ? this.localReferences : this.externalReferences;
        let names = table.get(slot);
        if (names === undefined) {
            names = new Set();
            table.set(slot, names);
        }
        names.add(name);
        return { slot, name };
    }
    #ref(cursor) {
        const slot = cursor.next();
        return { slot, name: this.#image.string(cursor.next()) };
    }
    #optRef(cursor) {
        const slot = cursor.next();
        const name = cursor.next();
        return slot === NX_IR_NONE ? undefined : { slot, name: this.#image.string(name) };
    }
    #list(cursor, element) {
        const count = cursor.next();
        const output = [];
        for (let index = 0; index < count; index += 1) {
            output.push(element());
        }
        return output;
    }
    #optNode(cursor) {
        const cell = cursor.next();
        return cell === NX_IR_NONE ? -1 : cell;
    }
    #optType(cursor) {
        const cell = cursor.next();
        return cell === NX_IR_NONE ? undefined : this.type(cell);
    }
    type(index) {
        const cached = this.#types.get(index);
        if (cached !== undefined) {
            return cached;
        }
        const entry = this.#image.entry("types", index);
        let prepared;
        switch (entry[0]) {
            case typeKinds.primitive:
                prepared = { kind: "primitive", name: this.#image.string(entry[1]) };
                break;
            case typeKinds.nominal:
                prepared = { kind: "nominal", slot: entry[1], name: this.#image.string(entry[2]) };
                break;
            case typeKinds.seq: {
                const occurrence = entry[2];
                prepared = {
                    kind: "seq",
                    item: this.type(entry[1]),
                    mayBeEmpty: (occurrence & occurrenceFlags.empty) !== 0,
                    mayBeMany: (occurrence & occurrenceFlags.many) !== 0,
                };
                break;
            }
            case typeKinds.function: {
                const result = this.type(entry[1]);
                const params = [];
                const count = entry[2];
                for (let position = 3; position < 3 + count * 3; position += 3) {
                    params.push({
                        name: this.#image.string(entry[position]),
                        ty: this.type(entry[position + 1]),
                        isContent: (entry[position + 2] & functionParamFlags.content) !== 0,
                        isOptional: (entry[position + 2] & functionParamFlags.optional) !== 0,
                    });
                }
                prepared = { kind: "function", params, result };
                break;
            }
            default:
                // Every kind the validator admits is handled above.
                fail("nx-ir-malformed", `Unknown type kind ${String(entry[0])} at type ${index}.`);
        }
        this.#types.set(index, prepared);
        return prepared;
    }
    #fields(cursor) {
        return this.#list(cursor, () => {
            const name = this.#image.string(cursor.next());
            const ty = this.type(cursor.next());
            const defaultNode = this.#optNode(cursor);
            const flags = cursor.next();
            return {
                name,
                ty,
                default: defaultNode,
                isContent: (flags & fieldFlags.content) !== 0,
                isRequired: (flags & fieldFlags.required) !== 0,
            };
        });
    }
    /** Decodes a validated declaration. */
    declaration(index, module) {
        const cursor = new Cursor(this.#image.entry("declarations", index));
        const kind = cursor.next();
        const name = this.#image.string(cursor.next());
        let prepared;
        switch (kind) {
            case declarationKinds.function: {
                const params = this.#list(cursor, () => {
                    const paramName = this.#image.string(cursor.next());
                    const ty = this.type(cursor.next());
                    const defaultNode = this.#optNode(cursor);
                    const flags = cursor.next();
                    return {
                        name: paramName,
                        ty,
                        isContent: (flags & functionParamFlags.content) !== 0,
                        isOptional: (flags & functionParamFlags.optional) !== 0,
                        default: defaultNode,
                    };
                });
                const body = cursor.next();
                const result = this.#optType(cursor);
                const isOptionalResult = (cursor.next() & functionResultFlags.optional) !== 0;
                prepared = { tag: "function", params, body, result, isOptionalResult };
                break;
            }
            case declarationKinds.value: {
                const value = cursor.next();
                prepared = { tag: "value", value, ty: this.#optType(cursor) };
                break;
            }
            case declarationKinds.record: {
                const fields = this.#fields(cursor);
                const bases = this.#list(cursor, () => this.#ref(cursor));
                const isAbstract = cursor.next() !== 0;
                const updateTarget = this.#optRef(cursor);
                prepared = { tag: "record", fields, bases, isAbstract, updateTarget };
                break;
            }
            case declarationKinds.component: {
                const props = this.#fields(cursor);
                const state = this.#fields(cursor);
                const body = this.#optNode(cursor);
                const flags = cursor.next();
                const emits = this.#list(cursor, () => {
                    const emitName = this.#image.string(cursor.next());
                    return { name: emitName, action: this.#ref(cursor) };
                });
                prepared = {
                    tag: "component",
                    props,
                    state,
                    body,
                    isAbstract: (flags & componentFlags.abstract) !== 0,
                    isExternal: (flags & componentFlags.external) !== 0,
                    emits,
                };
                break;
            }
            case declarationKinds.union: {
                const cases = this.#list(cursor, () => {
                    const caseName = this.#image.string(cursor.next());
                    const fields = this.#fields(cursor);
                    return { name: caseName, fields, isConstant: cursor.next() !== 0 };
                });
                const bases = this.#list(cursor, () => this.#ref(cursor));
                const propertyTarget = this.#optRef(cursor);
                prepared = { tag: "union", cases, bases, propertyTarget };
                break;
            }
            default:
                prepared = { tag: "typeAlias" };
                break;
        }
        return { index, name, module, kind: prepared };
    }
}
// ------------------------------------------------------------------------------------------------
// Linking
// ------------------------------------------------------------------------------------------------
/**
 * The prelude this release was built with, prepared at most once and reused across linked programs.
 *
 * <para>Prepared lazily rather than at module load, so a host that links no image that reaches the
 * prelude never decodes it. A failure is a result rather than a throw, because every caller reaches
 * it from a `try*` entry point, whose contract is to answer with diagnostics.</para>
 */
let preparedPrelude;
function builtInPrelude() {
    if (preparedPrelude !== undefined) {
        return { ok: true, value: preparedPrelude };
    }
    const decoded = tryDecodeBase64(NX_PRELUDE_IMAGE_BASE64);
    if (!decoded.ok) {
        return decoded;
    }
    const prepared = tryPrepareNxIrModule(decoded.value);
    if (prepared.ok) {
        preparedPrelude = prepared.value;
    }
    return prepared;
}
/** Decodes standard base64 without assuming `atob` or `Buffer`. */
function tryDecodeBase64(encoded) {
    const alphabet = "ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    const body = encoded.replace(/=+$/u, "");
    const bytes = new Uint8Array((body.length * 3) >> 2);
    let written = 0;
    let group = 0;
    let held = 0;
    for (const character of body) {
        const value = alphabet.indexOf(character);
        if (value < 0) {
            return {
                ok: false,
                diagnostics: [
                    diagnostic("nx-ir-prelude-image", `The built-in prelude image is not valid base64: '${character}'.`),
                ],
            };
        }
        group = (group << 6) | value;
        held += 6;
        if (held >= 8) {
            held -= 8;
            bytes[written] = (group >> held) & 0xff;
            written += 1;
        }
    }
    return { ok: true, value: bytes.subarray(0, written) };
}
export function linkNxIrProgram(entry, options) {
    const result = tryLinkNxIrProgram(entry, options);
    if (!result.ok) {
        throw new NxIrRuntimeError(result.diagnostics);
    }
    return result.value;
}
export function tryLinkNxIrProgram(entry, options) {
    const diagnostics = [];
    const modulesByIdentity = new Map();
    // The prelude is the one module no host has to supply: the host's resolver is asked first, and
    // the built-in image answers for the prelude's identity when the host does not. This wraps the
    // resolver rather than seeding a slot, so it answers at any depth of the link, including for a
    // module the host did supply.
    const resolve = (identity) => {
        const supplied = options.resolve(identity);
        if (supplied !== undefined || identity !== NX_PRELUDE_MODULE_IDENTITY) {
            return supplied;
        }
        const prelude = builtInPrelude();
        if (prelude.ok) {
            return prelude.value;
        }
        diagnostics.push(diagnostic("nx-ir-prelude-image", `The built-in prelude could not be prepared: ${prelude.diagnostics
            .map((reported) => reported.message)
            .join(" ")}`));
        return undefined;
    };
    const link = (module) => {
        const existing = modulesByIdentity.get(module.identity);
        if (existing !== undefined) {
            return existing;
        }
        const slots = [];
        const linked = { module, slots };
        // Registered before its slots resolve, so a module that names itself through another
        // module's table links to one object rather than recursing.
        modulesByIdentity.set(module.identity, linked);
        slots.push(linked);
        module.artifact.modules.slice(1).forEach((entryTable, offset) => {
            const slot = offset + 1;
            const resolved = resolve(entryTable.identity);
            if (resolved === undefined) {
                diagnostics.push(diagnostic("nx-ir-link-missing-module", `Module '${module.identity}' links against '${entryTable.identity}', which the resolver did not supply.`));
                slots.push(linked);
                return;
            }
            if (resolved.identity !== entryTable.identity) {
                diagnostics.push(diagnostic("nx-ir-link-identity", `The resolver answered '${entryTable.identity}' with a module whose identity is '${resolved.identity}'.`));
            }
            if (resolved.version !== entryTable.version && options.allowVersionMismatch !== true) {
                diagnostics.push(diagnostic("nx-ir-link-version", `Module '${module.identity}' was compiled against '${entryTable.identity}' version '${entryTable.version}', but the resolved module is version '${resolved.version}'.`));
            }
            for (const name of module.externalReferences.get(slot) ?? []) {
                if (!resolved.declarationsByName.has(name)) {
                    diagnostics.push(diagnostic("nx-ir-link-missing-declaration", `Module '${module.identity}' references '${name}' in '${entryTable.identity}', which does not declare it.`));
                }
            }
            slots.push(link(resolved));
        });
        return linked;
    };
    const linkedEntry = link(entry);
    if (diagnostics.length > 0) {
        return { ok: false, diagnostics };
    }
    return {
        ok: true,
        value: {
            entry: linkedEntry,
            modulesByIdentity,
            functionEntrypoints: entry.functionEntrypoints,
            componentEntrypoints: entry.componentEntrypoints,
            nominalShapesFor: (discriminator) => nominalShapesFor(modulesByIdentity, discriminator),
        },
    };
}
/**
 * The shapes a `$type` names across the linked modules.
 *
 * Each module already holds its own shapes indexed by discriminator, so this is one map lookup per
 * linked module — the modules of a program are few, however many declarations they hold. A base is
 * resolved through the module that wrote it, since its slot is that module's.
 */
function nominalShapesFor(modulesByIdentity, discriminator) {
    const shapes = [];
    for (const linked of modulesByIdentity.values()) {
        for (const skeleton of linked.module.nominalShapeSkeletons.get(discriminator) ?? []) {
            shapes.push({
                discriminator: skeleton.discriminator,
                declaration: `${linked.module.identity}::${skeleton.declarationName}`,
                fields: skeleton.fields,
                bases: skeleton.bases.map((base) => declarationKey(linked, base)),
                isAbstract: skeleton.isAbstract,
                linked,
            });
        }
    }
    return shapes;
}
/**
 * Prepares and links a self-contained artifact: one whose module table holds only itself.
 */
export function prepareNxIrProgram(input) {
    const result = tryPrepareNxIrProgram(input);
    if (!result.ok) {
        throw new NxIrRuntimeError(result.diagnostics);
    }
    return result.value;
}
export function tryPrepareNxIrProgram(input) {
    const prepared = tryPrepareNxIrModule(input);
    if (!prepared.ok) {
        return prepared;
    }
    return tryLinkNxIrProgram(prepared.value, { resolve: () => undefined });
}
/** The key that identifies one declaration across a program: its module's identity and its name. */
export function declarationKey(linked, reference) {
    const target = linked.slots[reference.slot] ?? linked;
    return `${target.module.identity}::${reference.name}`;
}
// ------------------------------------------------------------------------------------------------
// Public evaluation APIs
// ------------------------------------------------------------------------------------------------
/**
 * The program an evaluation API runs: a linked program, or a prepared module whose table names
 * only itself, which is a program on its own. A module that names other modules must be linked.
 */
function programOf(program) {
    if ("entry" in program) {
        return program;
    }
    if (program.artifact.modules.length > 1) {
        fail("nx-ir-unlinked", `Module '${program.identity}' names ${program.artifact.modules.length - 1} other module(s) in its table and must be linked with linkNxIrProgram before it is evaluated.`);
    }
    return linkNxIrProgram(program, { resolve: () => undefined });
}
export function evaluateFunction(program, name, args = [], options = {}) {
    const linkedProgram = programOf(program);
    const declaration = linkedProgram.functionEntrypoints.get(name);
    if (declaration === undefined || declaration.kind.tag !== "function") {
        fail("nx-ir-missing-entrypoint", `Function entrypoint '${name}' was not found.`);
    }
    return entryResult(declaration, invokeFunction(linkedProgram, linkedProgram.entry, declaration, args, options, 0));
}
export function constructComponentDescriptor(program, name, props = {}, content = [], options = {}) {
    const linkedProgram = programOf(program);
    const { declaration, component } = componentDeclaration(linkedProgram, name);
    const { fields: input, handlers } = splitHandlerProperties(linkedProgram.entry, declaration, props, `${name} props`);
    const contentField = component.props.find((field) => field.isContent);
    // A host supplies content as an argument, with no body to have been written or not written, and
    // the argument defaults to the empty array. So no content passed means no content: there is no
    // way for a caller to say "a body that produced nothing", and the declared default stands.
    applyContentBinding(input, contentField?.name, component.props, content, name, false);
    const normalized = normalizeFields(linkedProgram, linkedProgram.entry, declaration, component.props, input, [], `${name} props`, false, options);
    return canonicalizeRendered({ $type: declaration.name, ...normalized, ...handlerObject(handlers) }).value;
}
export function initializeComponent(program, name, props = {}, options = {}) {
    const linkedProgram = programOf(program);
    const { declaration, component } = componentDeclaration(linkedProgram, name);
    if (component.isAbstract || component.body < 0) {
        fail("nx-ir-component", `Component '${name}' cannot be initialized because it has no body.`);
    }
    const path = `${name} props`;
    const resolved = resolveParentHandlersInProps(props, options.parent, path);
    const { fields, handlers: handlerProps } = splitHandlerProperties(linkedProgram.entry, declaration, resolved, path);
    const frame = [];
    const normalizedProps = normalizeFields(linkedProgram, linkedProgram.entry, declaration, component.props, fields, frame, path, false, options);
    const state = options.state === undefined
        ? normalizeFields(linkedProgram, linkedProgram.entry, declaration, component.state, {}, frame, `${name} state`, false, options)
        : normalizeFields(linkedProgram, linkedProgram.entry, declaration, component.state, { ...options.state }, frame, `${name} state`, true, options);
    const { value: rendered, handlers } = canonicalizeRendered(evalNode(component.body, {
        program: linkedProgram,
        linked: linkedProgram.entry,
        declaration,
        frame,
        options,
        depth: 0,
    }), 1);
    const instance = freezeInstance({
        component: name,
        declaration,
        props: normalizedProps,
        handlerProps,
        state,
        handlers,
        generation: 1,
    });
    return { rendered, state: { ...state }, instance };
}
export function evaluateComponent(program, name, props, state, options = {}) {
    const linkedProgram = programOf(program);
    const { declaration, component } = componentDeclaration(linkedProgram, name);
    if (component.isAbstract || component.body < 0) {
        fail("nx-ir-component", `Component '${name}' cannot be evaluated because it has no body.`);
    }
    const path = `${name} props`;
    const { fields } = splitHandlerProperties(linkedProgram.entry, declaration, props, path);
    const frame = [];
    normalizeFields(linkedProgram, linkedProgram.entry, declaration, component.props, fields, frame, path, false, options);
    normalizeFields(linkedProgram, linkedProgram.entry, declaration, component.state, state, frame, `${name} state`, true, options);
    return {
        rendered: canonicalizeRendered(evalNode(component.body, {
            program: linkedProgram,
            linked: linkedProgram.entry,
            declaration,
            frame,
            options,
            depth: 0,
        })).value,
    };
}
/**
 * Dispatches a batch against an instance and returns the next one, without touching the instance
 * given. Each entry is either an action the component emits, which runs the handler the parent
 * bound on the instance's props, or an `ActionHandlerInvocation` naming a handler of the
 * instance's most recent rendered output by token, with the action to feed it. Entries run in
 * order. A handler the component's own body bound reads the state live and patches it with the
 * component's update records; any other handler sees only what it captured, and everything it
 * returns is an effect. The body is rendered once against the state the batch produced. A failure
 * throws before anything is returned, so the instance given stays the state of record.
 */
export function dispatchComponentActions(program, instance, batch, options = {}) {
    const linkedProgram = programOf(program);
    const { declaration, component } = componentDeclaration(linkedProgram, instance.component);
    if (declaration !== instance.declaration) {
        fail("nx-ir-component", `The instance of '${instance.component}' was initialized by another program.`);
    }
    const linked = linkedProgram.entry;
    const ownerKey = `${linked.module.identity}::${declaration.name}`;
    let working = { ...instance.state };
    const effects = [];
    batch.forEach((entry, index) => {
        const path = `dispatch entry ${index}`;
        const object = requireObject(entry, path);
        if (object.$type === handlerInvocationTypeName) {
            const token = object.token;
            if (typeof token !== "string") {
                fail("nx-ir-boundary-type", `Expected ${path} to carry a string 'token' read from rendered output.`);
            }
            const handler = instance.handlers.get(token);
            if (handler === undefined) {
                fail("nx-ir-handler-token", `Unknown handler token '${token}' for the '${instance.component}' instance.`);
            }
            const owned = handler.owner === ownerKey;
            const results = invokeHandler(linkedProgram, handler, requireObject(object.action, `${path}.action`), owned ? { component, state: working } : undefined, options);
            for (const result of results) {
                if (owned && isUpdateRecordFor(result, linkedProgram, ownerKey)) {
                    working = patchComponentState(linkedProgram, linked, declaration, component, working, result, options);
                }
                else {
                    effects.push(result);
                }
            }
            return;
        }
        const typeName = object.$type;
        if (typeof typeName !== "string") {
            fail("nx-ir-boundary-type", `Expected ${path} to be an action record with a '$type' discriminator.`);
        }
        const emit = component.emits.find((candidate) => resolveReference(linked, candidate.action.slot, candidate.action.name).declaration.name === typeName);
        if (emit === undefined) {
            fail("nx-ir-component-action", `Component '${instance.component}' does not emit '${typeName}'.`);
        }
        // The entry is host input, so it is constructed against the emitted action before the handler
        // is looked up: a malformed payload fails whether or not the parent bound one.
        const action = normalizeActionInput(linkedProgram, resolveReference(linked, emit.action.slot, emit.action.name), object, `${typeName} action`, options);
        const handler = instance.handlerProps.get(handlerPropertyName(emit.name));
        if (handler !== undefined) {
            // The parent bound this handler, so everything it returns belongs to the parent, via the host.
            effects.push(...invokeHandler(linkedProgram, handler, action, undefined, options));
        }
    });
    // The body sees the declared props and the state, as it did at initialization; the handler
    // props the parent bound are carried by the instance but were never in scope.
    // A prop or state field the instance carries no key for is an empty optional, which the body
    // reads as the empty value.
    const frame = [];
    component.props.forEach((field, index) => {
        frame[index] = instance.props[field.name] ?? [];
    });
    component.state.forEach((field, index) => {
        frame[component.props.length + index] = working[field.name] ?? [];
    });
    const generation = instance.generation + 1;
    const { value: rendered, handlers } = canonicalizeRendered(evalNode(component.body, { program: linkedProgram, linked, declaration, frame, options, depth: 0 }), generation);
    const next = freezeInstance({
        component: instance.component,
        declaration,
        props: instance.props,
        handlerProps: instance.handlerProps,
        state: working,
        handlers,
        generation,
    });
    return {
        rendered,
        effects: effects.map((effect) => canonicalizeRendered(effect).value),
        state: { ...working },
        instance: next,
    };
}
export function normalizeComponentState(program, name, state, options = {}) {
    const linkedProgram = programOf(program);
    const { declaration, component } = componentDeclaration(linkedProgram, name);
    return normalizeFields(linkedProgram, linkedProgram.entry, declaration, component.state, state, propsFrame(component), `${name} state`, true, options);
}
/**
 * A frame with the prop slots left unbound, for validating state on its own: no default is
 * evaluated there, so nothing reads a prop, and a slot that is read anyway fails as unbound
 * rather than yielding a value the props never held.
 */
function propsFrame(component) {
    return new Array(component.props.length);
}
/**
 * Applies a patch to host-owned component state and returns the validated next state.
 *
 * The patch is either a plain partial state object or the component's own update record,
 * `{ $type: "<Component>.Update", ... }`. Either way a present field replaces the current value, an
 * absent one keeps it, and a present empty value — `null` or `[]` — clears an optional state field,
 * so the next state carries no key for it; for a field that is not optional it is rejected.
 */
export function applyComponentStatePatch(program, name, currentState, patch, options = {}) {
    const linkedProgram = programOf(program);
    const { declaration, component } = componentDeclaration(linkedProgram, name);
    return patchComponentState(linkedProgram, linkedProgram.entry, declaration, component, currentState, patch, options);
}
/** Applies a patch to a component's state with full validation: what `applyComponentStatePatch` and dispatch share. */
function patchComponentState(program, linked, declaration, component, currentState, patch, options) {
    const name = declaration.name;
    const { $type: discriminator, ...fields } = patch;
    const expectedUpdate = `${name}.Update`;
    if (discriminator !== undefined && discriminator !== expectedUpdate) {
        fail("nx-ir-state-patch", `Cannot apply '${String(discriminator)}' to ${name} state; only '${expectedUpdate}' patches it.`);
    }
    const known = new Set(component.state.map((field) => field.name));
    for (const key of Object.keys(fields)) {
        if (!known.has(key)) {
            fail("nx-ir-state-field", `Unknown ${name} state field '${key}'.`);
        }
    }
    return normalizeFields(program, linked, declaration, component.state, { ...currentState, ...fields }, propsFrame(component), `${name} state`, true, options);
}
function componentDeclaration(program, name) {
    const declaration = program.componentEntrypoints.get(name);
    if (declaration === undefined || declaration.kind.tag !== "component") {
        fail("nx-ir-component", `Component '${name}' was not found.`);
    }
    return { declaration, component: declaration.kind };
}
/**
 * A function as a value: a reference to a declaration of a linked module. It captures nothing,
 * so two references are equal exactly when they name one declaration, which is what its JSON
 * form — the canonical `Function` record — compares by.
 */
class FunctionReferenceValue {
    $nxKind = "functionReference";
    linked;
    declaration;
    constructor(linked, declaration) {
        this.linked = linked;
        this.declaration = declaration;
    }
    toJSON() {
        return functionRecord(this);
    }
}
/** The canonical record of a function value: `{ $type: "Function", module, name }`. */
function functionRecord(reference) {
    return { $type: functionTypeName, module: reference.linked.module.identity, name: reference.declaration.name };
}
/** The `Function` record a host supplies where a function value is expected, if `value` is one. */
function asFunctionRecord(value) {
    if (!isObject(value) || value.$type !== functionTypeName) {
        return undefined;
    }
    const { module, name } = value;
    if (typeof module !== "string" || typeof name !== "string") {
        return undefined;
    }
    return { module, name };
}
/**
 * Resolves a host-supplied `Function` record to the declaration it names, failing by name when
 * the linked program has no such module or function.
 */
function resolveFunctionRecord(program, record, path) {
    const linked = program.modulesByIdentity.get(record.module);
    if (linked === undefined) {
        fail("nx-ir-function-value", `${path} names function '${record.name}' of module '${record.module}', which the program does not link.`);
    }
    const declaration = linked.module.declarationsByName.get(record.name);
    if (declaration === undefined || declaration.kind.tag !== "function") {
        fail("nx-ir-function-value", `${path} names function '${record.name}', which module '${record.module}' does not declare.`);
    }
    return new FunctionReferenceValue(linked, declaration);
}
/** An internal value smuggled through `NxCanonicalValue` positions until the boundary. */
function internal(value) {
    return value;
}
function resolveReference(linked, slot, name) {
    const target = linked.slots[slot];
    if (target === undefined) {
        fail("nx-ir-reference", `Module '${linked.module.identity}' has no module at slot ${slot}.`);
    }
    const declaration = target.module.declarationsByName.get(name);
    if (declaration === undefined) {
        fail("nx-ir-reference", `Module '${target.module.identity}' does not declare '${name}'.`);
    }
    return { linked: target, declaration };
}
/**
 * Calls a function with its arguments by position. An argument that is `undefined`, or past the
 * end of `args`, was left out: the function fills its parameter with the default it declares,
 * evaluated here after the parameters before it, or with empty when the parameter is optional.
 */
function invokeFunction(program, linked, declaration, args, options, depth) {
    const maxCallDepth = options.maxCallDepth ?? 100;
    if (depth > maxCallDepth) {
        fail("nx-ir-resource-limit", `Maximum NX IR call depth ${maxCallDepth} was exceeded.`);
    }
    const kind = declaration.kind;
    if (kind.tag !== "function") {
        fail("nx-ir-call", `'${declaration.name}' is not a function.`);
    }
    if (args.length > kind.params.length) {
        fail("nx-ir-arguments", `Function '${declaration.name}' expected at most ${kind.params.length} arguments, got ${args.length}.`);
    }
    const frame = [];
    const context = { program, linked, declaration, frame, options, depth };
    kind.params.forEach((param, index) => {
        const arg = args[index];
        let value;
        if (arg !== undefined) {
            // Body content reaches the content parameter as the list of children the emitter
            // gathered, and a child that is itself a list is spliced, as it is for a component's
            // content.
            value = param.isContent && Array.isArray(arg) ? spliceContent(arg) : arg;
        }
        else if (param.default >= 0) {
            value = evalNode(param.default, context);
        }
        else if (param.isOptional) {
            value = [];
        }
        else {
            fail("nx-ir-arguments", `Function '${declaration.name}' requires argument '${param.name}'.`);
        }
        frame[index] = normalizeValue(context, param.ty, value, param.name);
    });
    const result = evalNode(kind.body, context);
    return kind.result === undefined
        ? result
        : normalizeValue(context, kind.result, result, `return value for '${declaration.name}'`);
}
function entryAt(context, index) {
    return context.linked.module.artifact.entry("nodes", index);
}
/** Evaluates the `count, node × count` list at `at` and returns the values and the position after it. */
function nodesAt(context, entry, at) {
    const count = entry[at];
    const values = [];
    for (let position = at + 1; position < at + 1 + count; position += 1) {
        values.push(evalNode(entry[position], context));
    }
    return { values, next: at + 1 + count };
}
/**
 * Evaluates the content children at `at`, splicing a child that evaluates to a list into the
 * content: a `for`, a braced sequence or a list-valued call among an element's children
 * contributes its items, not itself. This is the interpreter's rule for body content, and it is
 * what makes two `for` loops side by side, or a list-returning function among the children, read
 * as one list of children.
 */
function contentAt(context, entry, at) {
    return itemsAt(context, entry, at);
}
/**
 * Whether a body was written at `at`, which is the count of children the element was given.
 *
 * A body of one child that contributes nothing still counts, which is the whole reason this is
 * asked separately from how many values the body produced.
 */
function hasBodyAt(entry, at) {
    return entry[at] !== 0;
}
/** Evaluates the `count, node × count` list at `at` as the items of one sequence. */
function itemsAt(context, entry, at) {
    const count = entry[at];
    const items = [];
    for (let position = at + 1; position < at + 1 + count; position += 1) {
        evalItemInto(entry[position], context, items);
    }
    return items;
}
/**
 * Appends what one item contributes to the sequence it sits in.
 *
 * A list value contributes its elements and every other value contributes itself. That is the
 * whole rule. A conditional that takes no branch needs no case here: it evaluates to the empty
 * value, which is the empty sequence, so it splices away like any other list-valued item, and a
 * conditional nested in one is no different.
 */
function evalItemInto(index, context, items) {
    const value = evalNode(index, context);
    if (Array.isArray(value)) {
        items.push(...value);
    }
    else {
        items.push(value);
    }
}
/** The branch an `if` or `if … is` takes, `undefined` when it takes none. */
function takenBranch(context, entry, index) {
    if (entry[0] === nodeKinds.if) {
        if (requireBoolean(context, index, evalNode(entry[1], context), "if condition")) {
            return entry[2];
        }
        return entry[3] === NX_IR_NONE ? undefined : entry[3];
    }
    const scrutinee = evalNode(entry[1], context);
    const arms = entry[2];
    let position = 3;
    for (let arm = 0; arm < arms; arm += 1) {
        const patterns = entry[position];
        let matched = false;
        for (let pattern = position + 1; pattern < position + 1 + patterns; pattern += 1) {
            if (!matched && patternMatches(scrutinee, evalPattern(context, entry[pattern]))) {
                matched = true;
            }
        }
        const body = entry[position + 1 + patterns];
        if (matched) {
            return body;
        }
        position += patterns + 2;
    }
    const otherwise = entry[position];
    return otherwise === NX_IR_NONE ? undefined : otherwise;
}
/**
 * A match arm's pattern as a value. A pattern naming a union case with fields stands for every
 * record of that case, so it is the case's `$type` alone: building the record would ask for fields
 * a pattern never supplies. Every other pattern is evaluated as the expression it is.
 */
function evalPattern(context, index) {
    const entry = entryAt(context, index);
    if (entry[0] === nodeKinds.unionCase) {
        const image = context.linked.module.artifact;
        const unionName = image.string(entry[2]);
        const caseName = image.string(entry[3]);
        const { declaration } = resolveReference(context.linked, entry[1], unionName);
        if (declaration.kind.tag === "union" &&
            declaration.kind.cases.some((candidate) => candidate.name === caseName && !candidate.isConstant)) {
            return { $type: `${unionName}.${caseName}` };
        }
    }
    return evalNode(index, context);
}
function spliceContent(values) {
    return values.flatMap((value) => (Array.isArray(value) ? value : [value]));
}
/** Evaluates the `count, (name, node) × count` list at `at` into an object, in property order. */
function propertiesAt(context, entry, at) {
    const image = context.linked.module.artifact;
    const count = entry[at];
    const properties = {};
    let position = at + 1;
    for (let index = 0; index < count; index += 1) {
        properties[image.string(entry[position])] = evalNode(entry[position + 1], context);
        position += 2;
    }
    return { properties, next: position };
}
function evalNode(index, context) {
    const entry = entryAt(context, index);
    const image = context.linked.module.artifact;
    switch (entry[0]) {
        case nodeKinds.bool:
            return entry[1] !== 0;
        case nodeKinds.string:
            return image.string(entry[1]);
        case nodeKinds.number:
            return evalConstant(context, entry[1], index);
        case nodeKinds.slot: {
            const slot = entry[1];
            if (slot >= context.frame.length || context.frame[slot] === undefined) {
                fail("nx-ir-slot", `Local slot '${image.string(entry[2])}' was not bound.`, context, index);
            }
            return context.frame[slot];
        }
        case nodeKinds.reference:
            return evalReference(context, entry[1], image.string(entry[2]), index);
        case nodeKinds.namedCall:
            return evalNamedCall(context, index, entry);
        case nodeKinds.binary: {
            const operator = binaryOperators[entry[1]];
            // `and` and `or` are the only non-strict operators: the left operand decides whether the
            // right one runs at all, so a guard such as `d != 0 && n / d > 1` never divides by zero.
            if (operator === "and" || operator === "or") {
                const left = requireBoolean(context, index, evalNode(entry[2], context), operator);
                if (left === (operator === "or")) {
                    return left;
                }
                return requireBoolean(context, index, evalNode(entry[3], context), operator);
            }
            return evalBinary(context, index, operator, evalNode(entry[2], context), evalNode(entry[3], context));
        }
        case nodeKinds.text:
            return primitiveText(context, index, evalNode(entry[1], context), image.string(entry[2]));
        case nodeKinds.unary: {
            const operand = evalNode(entry[2], context);
            switch (unaryOperators[entry[1]]) {
                case "neg":
                    return -checkedNumber(context, index, operand, "neg");
                case "not":
                    return !requireBoolean(context, index, operand, "not");
                default:
                    fail("nx-ir-operator", `Unknown unary operator '${String(entry[1])}'.`, context, index);
            }
        }
        // eslint-disable-next-line no-fallthrough
        case nodeKinds.call:
            return evalCall(context, index, entry);
        case nodeKinds.intrinsic:
            return evalIntrinsic(context, index, entry);
        case nodeKinds.if:
        case nodeKinds.ifIs: {
            // A missing `else` is an `else { }`, so a conditional that takes no branch is the empty
            // value. Where the value is an item being collected that splices away to nothing, by the
            // same rule any other sequence-valued item follows; where one value is expected, the
            // expression's type admits zero and the empty value is what it promises.
            const body = takenBranch(context, entry, index);
            return body === undefined ? [] : evalNode(body, context);
        }
        case nodeKinds.array:
            return itemsAt(context, entry, 1);
        case nodeKinds.for: {
            // The iterable is read as its items: a `+` or `*` value is an array of them, and a `?` value
            // is the empty array or the one item it holds, so a `for` over an optional runs at most once.
            const iterable = evalNode(entry[5], context);
            const itemSlot = entry[1];
            const indexSlot = entry[3];
            // An optional that holds an item runs the body once, and the product of `?` with the body's
            // occurrence is the body's own, so the body's value is the loop's value unchanged: an item
            // stays an item rather than becoming a one-element array, as in the interpreter.
            if (!Array.isArray(iterable) && !isEmptyValue(iterable)) {
                context.frame[itemSlot] = iterable;
                if (indexSlot !== NX_IR_NONE) {
                    context.frame[indexSlot] = 0;
                }
                return evalNode(entry[6], context);
            }
            const items = Array.isArray(iterable) ? iterable : [];
            // A `for` concatenates what its body yields, so each iteration contributes on the same terms
            // as an item of a braced value list.
            const results = [];
            items.forEach((item, position) => {
                context.frame[itemSlot] = item;
                if (indexSlot !== NX_IR_NONE) {
                    context.frame[indexSlot] = position;
                }
                evalItemInto(entry[6], context, results);
            });
            return results;
        }
        case nodeKinds.forRange: {
            const iterable = evalNode(entry[5], context);
            const range = integerRange(iterable);
            if (range === undefined) {
                fail("nx-ir-for", "For expression iterable must evaluate to a Range record with integer bounds.", context, index);
            }
            const limit = context.options.maxRangeLength ?? NX_DEFAULT_MAX_RANGE_LENGTH;
            if (range.count > limit) {
                fail("nx-ir-resource-limit", `Iterating this range would run the loop body ${range.count} times, above the maxRangeLength limit of ${limit}.`, context, index);
            }
            const itemSlot = entry[1];
            const indexSlot = entry[3];
            const results = [];
            for (let position = 0; position < range.count; position += 1) {
                context.frame[itemSlot] = range.start + position;
                if (indexSlot !== NX_IR_NONE) {
                    context.frame[indexSlot] = position;
                }
                evalItemInto(entry[6], context, results);
            }
            return results;
        }
        case nodeKinds.member:
            return readMember(context, index, evalNode(entry[1], context), image.string(entry[2]));
        case nodeKinds.optionalMember: {
            // `x?.m`: the receiver is evaluated once; when it is empty so is the result.
            const receiver = optionalItem(evalNode(entry[1], context));
            return receiver === undefined ? [] : readMember(context, index, receiver, image.string(entry[2]));
        }
        case nodeKinds.exists:
            return !isEmptyValue(evalNode(entry[1], context));
        case nodeKinds.coalesce: {
            // `x ?? y`: the right operand runs only when the left is empty.
            const left = evalNode(entry[1], context);
            return isEmptyValue(left) ? evalNode(entry[2], context) : left;
        }
        case nodeKinds.record:
            return evalRecord(context, index, entry);
        case nodeKinds.unionCase:
            return evalUnionCase(context, index, entry);
        case nodeKinds.element: {
            const { properties, next } = propertiesAt(context, entry, 3);
            const content = contentAt(context, entry, next);
            // The interpreter's rule for an element with no declared content field: one child is
            // bound as itself, several as a list.
            if (content.length === 1) {
                properties.content = content[0];
            }
            else if (content.length > 1) {
                properties.content = content;
            }
            return { $type: image.string(entry[2]), ...properties };
        }
        case nodeKinds.component:
            return evalComponentDescriptor(context, index, entry);
        case nodeKinds.actionHandler:
            return evalActionHandler(context, entry);
        default:
            fail("nx-ir-expression", `Unknown NX IR node kind '${String(entry[0])}'.`, context, index);
    }
}
/**
 * Reads a member of a record value. A record stores no entry for an optional field that is empty,
 * so a declared field that is not stored reads as the empty value; an update record's fields are
 * all of that kind, since an absent one is unchanged. The declaration is found by the value's
 * `$type`, as `isUpdateRecordFor` finds it; a name two modules share reads as empty when either
 * declares the field so, which is what a value stamped with that name can be.
 */
function readMember(context, nodeIndex, base, member) {
    const object = requireObject(base, "member access");
    if (Object.prototype.hasOwnProperty.call(object, member)) {
        return object[member];
    }
    if (typeof object.$type === "string" && fieldReadsAsEmpty(context.program, object.$type, member)) {
        return [];
    }
    fail("nx-ir-member", `Object does not contain member '${member}'.`, context, nodeIndex);
}
/** Whether `member` is a field of the shape `discriminator` names that reads as empty when it is not stored. */
function fieldReadsAsEmpty(program, discriminator, member) {
    return program.nominalShapesFor(discriminator).some((shape) => {
        const declaration = shape.linked.module.declarationsByName.get(shape.discriminator);
        const isUpdate = declaration !== undefined && declaration.kind.tag === "record" && declaration.kind.updateTarget !== undefined;
        return shape.fields.some((field) => field.name === member && (isUpdate || admitsEmpty(field.ty)));
    });
}
/**
 * `[19, ref, str, ref, slot, ref?, node]`: the component and emit the handler answers, the action
 * record it accepts, the slot of its `action` binding, its owner, and its body. Nothing is
 * evaluated here beyond copying the frame, which is the by-value capture the interpreter takes.
 */
function evalActionHandler(context, entry) {
    const image = context.linked.module.artifact;
    const component = resolveReference(context.linked, entry[1], image.string(entry[2]));
    const action = resolveReference(context.linked, entry[4], image.string(entry[5]));
    const ownerSlot = entry[7];
    const owner = ownerSlot === NX_IR_NONE ? undefined : resolveReference(context.linked, ownerSlot, image.string(entry[8]));
    return internal({
        $nxKind: "actionHandler",
        linked: context.linked,
        declaration: context.declaration,
        component: `${component.linked.module.identity}::${component.declaration.name}`,
        componentName: component.declaration.name,
        emit: image.string(entry[3]),
        action,
        actionSlot: entry[6],
        owner: owner === undefined ? undefined : `${owner.linked.module.identity}::${owner.declaration.name}`,
        body: entry[9],
        captured: context.frame.slice(),
    });
}
const float64Cells = new Uint32Array(2);
const float64View = new Float64Array(float64Cells.buffer);
const littleEndian = new Uint8Array(float64Cells.buffer)[0] === 0 && (() => {
    float64Cells[0] = 1;
    const little = new Uint8Array(float64Cells.buffer)[0] === 1;
    float64Cells[0] = 0;
    return little;
})();
function evalConstant(context, constantIndex, nodeIndex) {
    const image = context.linked.module.artifact;
    const constant = image.entry("constants", constantIndex);
    switch (constant[0]) {
        case constantKinds.int:
            // Two cells, low word first; the high word is signed. Every value the emitter writes with
            // this kind is inside the safe range, so the product is exact.
            return (constant[2] | 0) * 4294967296 + constant[1];
        case constantKinds.float:
            float64Cells[littleEndian ? 0 : 1] = constant[1];
            float64Cells[littleEndian ? 1 : 0] = constant[2];
            return float64View[0];
        case constantKinds.bigint:
            return { $type: "nx.int", value: image.string(constant[1]) };
        default:
            fail("nx-ir-literal", `Unknown constant kind '${String(constant[0])}'.`, context, nodeIndex);
    }
}
function evalReference(context, slot, name, nodeIndex) {
    const { linked, declaration } = resolveReference(context.linked, slot, name);
    if (declaration.kind.tag === "function") {
        return internal(new FunctionReferenceValue(linked, declaration));
    }
    if (declaration.kind.tag === "value") {
        const valueContext = { ...context, linked, declaration, frame: [] };
        const value = evalNode(declaration.kind.value, valueContext);
        return declaration.kind.ty === undefined
            ? value
            : normalizeValue(valueContext, declaration.kind.ty, value, `value '${declaration.name}'`);
    }
    fail("nx-ir-reference", `Declaration '${name}' cannot be used as a value.`, context, nodeIndex);
}
function evalCall(context, nodeIndex, entry) {
    const callee = evalNode(entry[1], context);
    if (!isFunctionReference(callee)) {
        fail("nx-ir-call", "NX IR call callee did not evaluate to a function reference.", context, nodeIndex);
    }
    // An argument the call left out is `NX_IR_NONE`, passed on as `undefined` for the function to fill.
    const count = entry[2];
    const args = [];
    for (let position = 3; position < 3 + count; position += 1) {
        const node = entry[position];
        args.push(node === NX_IR_NONE ? undefined : evalNode(node, context));
    }
    return invokeFunction(context.program, callee.linked, callee.declaration, args, context.options, context.depth + 1);
}
/**
 * A call of a function-typed value by name: the callee is a slot or reference holding a function
 * value, and the arguments are bound to that function's parameters under the subset rule.
 */
function evalNamedCall(context, nodeIndex, entry) {
    const callee = evalNode(entry[1], context);
    if (!isFunctionReference(callee)) {
        fail("nx-ir-call", "NX IR named call callee did not evaluate to a function value.", context, nodeIndex);
    }
    const { properties } = propertiesAt(context, entry, 2);
    return invokeFunctionByName(context.program, callee, properties, context.options, context.depth + 1);
}
/**
 * Invokes a function value with arguments by name. The caller supplied every parameter of the
 * function *type* it holds, so an argument the declaration does not name is dropped, and a
 * parameter the declaration names must be present unless it has a default or is optional, in
 * which case the function fills it.
 */
function invokeFunctionByName(program, callee, args, options, depth) {
    const kind = callee.declaration.kind;
    if (kind.tag !== "function") {
        fail("nx-ir-call", `'${callee.declaration.name}' is not a function.`);
    }
    const positional = kind.params.map((param) => Object.prototype.hasOwnProperty.call(args, param.name) ? args[param.name] : undefined);
    return invokeFunction(program, callee.linked, callee.declaration, positional, options, depth);
}
/**
 * Calls the function a canonical `Function` record names with arguments keyed by parameter name,
 * and returns the canonical result. An argument the function does not declare is dropped, as the
 * subset rule allows; a parameter it declares and the arguments lack is a diagnostic naming it.
 */
export function callFunction(program, value, args = {}, options = {}) {
    const linkedProgram = programOf(program);
    const record = asFunctionRecord(value);
    if (record === undefined) {
        fail("nx-ir-function-value", "callFunction expects a Function record: { $type: \"Function\", module, name }.");
    }
    const callee = resolveFunctionRecord(linkedProgram, record, "callFunction");
    return entryResult(callee.declaration, invokeFunctionByName(linkedProgram, callee, args, options, 0));
}
/**
 * An entry call's result as the host reads it: canonical, and `null` where the function's result
 * type is a standalone `T?` and holds nothing — the host's spelling of an absent single value.
 * Every other result keeps its encoding, so an empty `T*` stays `[]`.
 */
function entryResult(declaration, value) {
    const isOptionalResult = declaration.kind.tag === "function" && declaration.kind.isOptionalResult;
    return isOptionalResult && isEmptyValue(value) ? null : canonicalizeRendered(value).value;
}
function evalRecord(context, nodeIndex, entry) {
    const image = context.linked.module.artifact;
    const name = image.string(entry[2]);
    const { linked, declaration } = resolveReference(context.linked, entry[1], name);
    if (declaration.kind.tag !== "record") {
        fail("nx-ir-record", `'${name}' is not a record.`, context, nodeIndex);
    }
    const record = declaration.kind;
    const { properties, next } = propertiesAt(context, entry, 3);
    const content = contentAt(context, entry, next);
    const contentField = record.fields.find((field) => field.isContent)?.name;
    applyContentBinding(properties, contentField, record.fields, content, name, hasBodyAt(entry, next));
    const normalized = record.updateTarget !== undefined
        ? normalizePatchFields(context, record.fields, properties, name)
        : normalizeFields(context.program, linked, declaration, record.fields, properties, [], name, false, context.options);
    return { $type: name, ...normalized };
}
function evalUnionCase(context, nodeIndex, entry) {
    const image = context.linked.module.artifact;
    const unionName = image.string(entry[2]);
    const caseName = image.string(entry[3]);
    const { linked, declaration } = resolveReference(context.linked, entry[1], unionName);
    if (declaration.kind.tag !== "union") {
        fail("nx-ir-union", `'${unionName}' is not a union.`, context, nodeIndex);
    }
    const unionCase = declaration.kind.cases.find((candidate) => candidate.name === caseName);
    if (unionCase === undefined) {
        fail("nx-ir-union", `'${unionName}' has no case '${caseName}'.`, context, nodeIndex);
    }
    // A constant case carries nothing beyond its own name.
    if (unionCase.isConstant) {
        return caseName;
    }
    const { properties, next } = propertiesAt(context, entry, 4);
    const content = contentAt(context, entry, next);
    const path = `${unionName}.${caseName}`;
    const contentField = unionCase.fields.find((field) => field.isContent)?.name;
    applyContentBinding(properties, contentField, unionCase.fields, content, path, hasBodyAt(entry, next));
    const normalized = normalizeFields(context.program, linked, declaration, unionCase.fields, properties, [], path, false, context.options);
    return { $type: path, ...normalized };
}
function evalComponentDescriptor(context, nodeIndex, entry) {
    const image = context.linked.module.artifact;
    const name = image.string(entry[2]);
    const { linked, declaration } = resolveReference(context.linked, entry[1], name);
    if (declaration.kind.tag !== "component") {
        fail("nx-ir-component", `'${name}' is not a component.`, context, nodeIndex);
    }
    const component = declaration.kind;
    const { properties, next } = propertiesAt(context, entry, 3);
    const content = contentAt(context, entry, next);
    const { fields: props, handlers } = splitHandlerProperties(linked, declaration, properties, `${name} props`);
    const contentField = component.props.find((field) => field.isContent)?.name;
    applyContentBinding(props, contentField, component.props, content, name, hasBodyAt(entry, next));
    const normalized = normalizeFields(context.program, linked, declaration, component.props, props, [], `${name} props`, false, context.options);
    return { $type: name, ...normalized, ...handlerObject(handlers) };
}
// ------------------------------------------------------------------------------------------------
// Action handlers: properties, canonical output, instances and dispatch
// ------------------------------------------------------------------------------------------------
/** The property a parent binds a handler for `emit` under: `onTapped` for `Tapped`. */
function handlerPropertyName(emit) {
    return `on${emit}`;
}
function handlerObject(handlers) {
    const output = {};
    for (const [name, handler] of handlers) {
        output[name] = internal(handler);
    }
    return output;
}
/**
 * Splits a component's handler properties from its declared props. A property named `on<Emit>` for
 * an emit the component declares is a handler property: it is not a prop, a body cannot read it,
 * and it goes around normalization to ride on the descriptor or the instance. Its value must be a
 * handler for that very emit of that very component. A handler under any other name matches no
 * emit, and an `ActionHandler` record names a handler only through a parent instance.
 */
function splitHandlerProperties(linked, declaration, input, path) {
    const component = declaration.kind;
    if (component.tag !== "component") {
        fail("nx-ir-component", `'${declaration.name}' is not a component.`);
    }
    const componentKey = `${linked.module.identity}::${declaration.name}`;
    const emitsByProperty = new Map(component.emits.map((emit) => [handlerPropertyName(emit.name), emit]));
    const fields = {};
    const handlers = new Map();
    for (const [key, value] of Object.entries(input)) {
        const emit = emitsByProperty.get(key);
        if (emit === undefined) {
            if (isActionHandler(value)) {
                fail("nx-ir-boundary-field", `Unknown ${path} field '${key}': '${declaration.name}' emits nothing a handler named '${key}' would answer.`);
            }
            fields[key] = value;
            continue;
        }
        if (isCanonicalActionHandler(value)) {
            fail("nx-ir-boundary-field", `Unknown ${path} field '${key}': an ActionHandler record names a handler only through a parent instance.`);
        }
        if (!isActionHandler(value)) {
            fail("nx-ir-type", `Expected ${path}.${key} to be an action handler for ${declaration.name}.${emit.name}.`);
        }
        if (value.component !== componentKey || value.emit !== emit.name) {
            // Two components of one name in different modules are told apart by their keys.
            const [expected, got] = value.componentName === declaration.name ? [componentKey, value.component] : [declaration.name, value.componentName];
            fail("nx-ir-type", `Expected ${path}.${key} to be an action handler for ${expected}.${emit.name}, got one for ${got}.${value.emit}.`);
        }
        handlers.set(key, value);
    }
    return { fields, handlers };
}
/**
 * Replaces every `ActionHandler` record in host-supplied props, at any depth, by the handler the
 * parent instance holds under its token. Without a parent such a record names nothing.
 */
function resolveParentHandlers(value, parent, path) {
    if (Array.isArray(value)) {
        return value.map((item, index) => resolveParentHandlers(item, parent, `${path}[${index}]`));
    }
    if (!isObject(value)) {
        return value;
    }
    if (isCanonicalActionHandler(value)) {
        if (parent === undefined) {
            fail("nx-ir-boundary-field", `Unknown field ${path}: an ActionHandler record names a handler only through a parent instance.`);
        }
        const token = value.token;
        const handler = typeof token === "string" ? parent.handlers.get(token) : undefined;
        if (handler === undefined) {
            fail("nx-ir-handler-token", `Unknown handler token '${String(token)}' at ${path} for the '${parent.component}' instance.`);
        }
        return internal(handler);
    }
    const output = {};
    for (const [key, item] of Object.entries(value)) {
        output[key] = resolveParentHandlers(item, parent, `${path}.${key}`);
    }
    return output;
}
function resolveParentHandlersInProps(props, parent, path) {
    return resolveParentHandlers(props, parent, path);
}
/**
 * Turns a rendered tree into canonical output, replacing every handler by its `ActionHandler`
 * record: the public name of the action it accepts and, when a generation is given, a token
 * `h<generation>-<n>` numbered by a walk that visits lists in order and object keys in sorted
 * order, which is the interpreter's walk, so the two runtimes agree on every token. The handlers
 * met are returned by token, for the instance that owns the output.
 */
function canonicalizeRendered(value, generation) {
    const handlers = new Map();
    const walk = (item) => {
        if (Array.isArray(item)) {
            return item.map(walk);
        }
        if (!isObject(item)) {
            return item;
        }
        if (isActionHandler(item)) {
            const record = { $type: actionHandlerTypeName, action: item.action.declaration.name };
            if (generation !== undefined) {
                const token = `h${generation}-${handlers.size + 1}`;
                handlers.set(token, item);
                record.token = token;
            }
            return record;
        }
        if (isFunctionReference(item)) {
            return functionRecord(item);
        }
        // Keys are visited in sorted order so the numbering matches, and written back in their own
        // order so the output reads as the declaration does.
        const canonical = new Map();
        for (const key of Object.keys(item).sort()) {
            canonical.set(key, walk(item[key]));
        }
        // An update record is the one place the canonical encoding writes `null`: a present empty
        // field is a cleared one, and key presence is what carries that, so the value is `null`
        // rather than the empty array an empty `*` field would be.
        const isUpdate = typeof item.$type === "string" && item.$type.endsWith(".Update");
        const output = {};
        for (const key of Object.keys(item)) {
            const value = canonical.get(key);
            output[key] = isUpdate && key !== "$type" && isEmptyValue(value) ? null : value;
        }
        return output;
    };
    return { value: walk(value), handlers };
}
function freezeInstance(instance) {
    Object.freeze(instance.props);
    Object.freeze(instance.state);
    return Object.freeze(instance);
}
/** Constructs a host-supplied action against the record the emit declares, defaults and all. */
function normalizeActionInput(program, action, input, path, options) {
    const expected = action.declaration.name;
    const kind = action.declaration.kind;
    if (kind.tag !== "record") {
        fail("nx-ir-type", `'${expected}' is not an action record.`);
    }
    const { $type: discriminator, ...rest } = input;
    if (discriminator !== undefined && discriminator !== expected) {
        fail("nx-ir-type", `Expected ${path} to be a '${expected}' action, got '${String(discriminator)}'.`);
    }
    return { $type: expected, ...normalizeFields(program, action.linked, action.declaration, kind.fields, rest, [], path, false, options) };
}
/**
 * Runs a handler: the action is constructed against the record the handler accepts, the body runs
 * over the captured frame with `action` in its slot, and, for a handler its own component
 * dispatches, `live` names that component and its working state, whose slots the body reads
 * instead of what was captured. The result is one record or a non-empty list of them.
 */
function invokeHandler(program, handler, action, live, options) {
    const label = `${handler.componentName}.${handler.emit}`;
    const expected = handler.action.declaration.name;
    if (action.$type !== expected) {
        fail("nx-ir-type", `Expected an action of type '${expected}' for handler ${label}, got '${String(action.$type)}'.`);
    }
    const normalizedAction = normalizeActionInput(program, handler.action, action, `${label} action`, options);
    const frame = handler.captured.slice();
    if (live !== undefined) {
        live.component.state.forEach((field, index) => {
            frame[live.component.props.length + index] = live.state[field.name] ?? [];
        });
    }
    frame[handler.actionSlot] = normalizedAction;
    const result = evalNode(handler.body, {
        program,
        linked: handler.linked,
        declaration: handler.declaration,
        frame,
        options,
        depth: 0,
    });
    const results = Array.isArray(result) ? [...result] : [result];
    if (results.length === 0) {
        fail("nx-ir-handler-result", `Handler ${label} returned an empty list; a handler returns an action, an update record, or a list of them.`);
    }
    for (const item of results) {
        if (!isObject(item) || typeof item.$type !== "string") {
            fail("nx-ir-handler-result", `Handler ${label} returned ${JSON.stringify(item)}; a handler returns an action, an update record, or a list of them.`);
        }
    }
    return results;
}
/** Whether `value` is the update record of the component with `ownerKey`. */
function isUpdateRecordFor(value, program, ownerKey) {
    if (!isObject(value) || typeof value.$type !== "string") {
        return false;
    }
    return program.nominalShapesFor(value.$type).some((shape) => {
        // A record's discriminator is its declaration name; a union case's never names an update record.
        const declaration = shape.linked.module.declarationsByName.get(shape.discriminator);
        return (declaration !== undefined &&
            declaration.kind.tag === "record" &&
            declaration.kind.updateTarget !== undefined &&
            declarationKey(shape.linked, declaration.kind.updateTarget) === ownerKey);
    });
}
function isActionHandler(value) {
    return typeof value === "object" && value !== null && value.$nxKind === "actionHandler";
}
/** A rendered handler as a host sees it: the record canonical output carries in a handler's place. */
function isCanonicalActionHandler(value) {
    return isObject(value) && value.$type === actionHandlerTypeName;
}
function evalIntrinsic(context, nodeIndex, entry) {
    const intrinsic = intrinsicNames[entry[1]] ?? String(entry[1]);
    const { values: args, next } = nodesAt(context, entry, 2);
    const expectArity = (arity) => {
        if (args.length !== arity) {
            fail("nx-ir-intrinsic", `Intrinsic '${intrinsic}' expects ${arity} arguments, got ${args.length}.`, context, nodeIndex);
        }
    };
    switch (intrinsic) {
        case "apply":
            expectArity(2);
            return applyRecordUpdate(intrinsicRecord(args[0], intrinsic), intrinsicRecord(args[1], intrinsic));
        case "merge":
            expectArity(2);
            return mergeUpdateRecords(intrinsicRecord(args[0], intrinsic), intrinsicRecord(args[1], intrinsic));
        case "diff":
            expectArity(2);
            return diffRecordValues(intrinsicRecord(args[0], intrinsic), intrinsicRecord(args[1], intrinsic));
        case "changed": {
            expectArity(1);
            const update = intrinsicRecord(args[0], intrinsic);
            const image = context.linked.module.artifact;
            const order = Array.from(entry.subarray(next + 1, next + 1 + entry[next]), (name) => image.string(name));
            return changedFieldsInOrder(update, order.length > 0 ? order : declaredFieldOrder(update, context.program));
        }
        default:
            fail("nx-ir-intrinsic", `Unknown intrinsic '${intrinsic}'.`, context, nodeIndex);
    }
}
function intrinsicRecord(value, intrinsic) {
    if (!isObject(value) || typeof value.$type !== "string") {
        fail("nx-ir-intrinsic", `Intrinsic '${intrinsic}' expects a record value, got ${JSON.stringify(value)}.`);
    }
    return value;
}
/**
 * A host-held value as the runtime holds it: every `null` — the canonical spelling of a cleared
 * update field, and what a JSON host writes for an absent optional — is the empty value. Nothing
 * else in a canonical value is `null`, so the decoding is safe at any depth.
 */
function decodeHostValue(value) {
    if (value === null) {
        return [];
    }
    if (Array.isArray(value)) {
        return value.map(decodeHostValue);
    }
    if (!isObject(value)) {
        return value;
    }
    const output = {};
    for (const [key, item] of Object.entries(value)) {
        output[key] = decodeHostValue(item);
    }
    return output;
}
function decodeHostRecord(value) {
    return decodeHostValue(value);
}
/**
 * `apply(record, update)`: the record with each field present in the update replaced, and each
 * field the update clears — present as `null` or the empty value — left out of the result, which
 * is how the canonical encoding writes an empty optional field; every absent field keeps its
 * value. The update must be the record's own `<Type>.Update`.
 */
export function applyUpdate(record, update) {
    return canonicalizeRendered(applyRecordUpdate(decodeHostRecord(record), decodeHostRecord(update))).value;
}
/**
 * `merge(first, second)`: every field present in either update, the second winning, a cleared
 * field included. A cleared field is `null` in the result, as the canonical encoding spells it.
 */
export function mergeUpdates(first, second) {
    return canonicalizeRendered(mergeUpdateRecords(decodeHostRecord(first), decodeHostRecord(second))).value;
}
/**
 * `diff(before, after)`: the `<Type>.Update` carrying exactly the fields whose values differ, each
 * with its value from `after`, comparing records and lists structurally. A field either record
 * leaves out is empty there, so a field `after` clears is present and `null` in the result.
 */
export function diffRecords(before, after) {
    return canonicalizeRendered(diffRecordValues(decodeHostRecord(before), decodeHostRecord(after))).value;
}
/**
 * `changed(update)`: the names of the fields present in the update, cleared ones included, in the
 * order the update record's declaration in `program` lists them. Fails when the program does not
 * declare the update record, since the order is then unknowable from the value.
 */
export function changedFields(update, program) {
    return changedFieldsInOrder(update, declaredFieldOrder(update, program));
}
function declaredFieldOrder(update, program) {
    const shapes = program.nominalShapesFor(update.$type);
    if (shapes.length !== 1) {
        fail("nx-ir-intrinsic", `Cannot order the fields of '${update.$type}': the program does not declare it.`);
    }
    return shapes[0].fields.map((field) => field.name);
}
function changedFieldsInOrder(update, order) {
    const keys = Object.keys(update).filter((key) => key !== "$type");
    const position = (key) => {
        const index = order.indexOf(key);
        return index < 0 ? order.length : index;
    };
    return keys.sort((left, right) => position(left) - position(right));
}
/**
 * Every present field of the update replaces the record's. A present empty value clears the
 * field, and a record stores no entry for an empty optional field, so the key is removed rather
 * than set to the empty value.
 */
function applyRecordUpdate(record, update) {
    const expected = `${record.$type}.Update`;
    if (update.$type !== expected) {
        fail("nx-ir-intrinsic", `Cannot apply '${update.$type}' to a '${record.$type}': only '${expected}' patches it.`);
    }
    const output = { ...record };
    for (const [key, value] of Object.entries(update)) {
        if (key === "$type") {
            continue;
        }
        if (isEmptyValue(value)) {
            delete output[key];
        }
        else {
            output[key] = value;
        }
    }
    return output;
}
function mergeUpdateRecords(first, second) {
    if (first.$type !== second.$type) {
        fail("nx-ir-intrinsic", `Cannot merge '${first.$type}' with '${second.$type}': the updates target different records.`);
    }
    const { $type: _second, ...later } = second;
    return { ...first, ...later };
}
function diffRecordValues(before, after) {
    if (before.$type !== after.$type) {
        fail("nx-ir-intrinsic", `Cannot diff '${before.$type}' against '${after.$type}': the records have different types.`);
    }
    const output = { $type: `${before.$type}.Update` };
    // A field either record leaves out is an empty optional there, so a field only one of them
    // carries still compares, and one `after` leaves out is present and empty in the result.
    for (const key of new Set([...Object.keys(before), ...Object.keys(after)])) {
        if (key === "$type") {
            continue;
        }
        const next = fieldOrEmpty(after, key);
        if (!valuesEqual(fieldOrEmpty(before, key), next)) {
            output[key] = next;
        }
    }
    return output;
}
/** A record's field, or the empty value for a key the record does not store. */
function fieldOrEmpty(record, key) {
    return Object.prototype.hasOwnProperty.call(record, key) ? record[key] : [];
}
/**
 * The one equality `==`, match patterns and `diff` share: numbers, strings and booleans by value,
 * lists by their items in order, records by their fields, and a function value by the
 * declaration it names. Every value compares as a sequence, so an item equals a one-element list
 * holding an equal item. The empty value equals only the empty value, being the empty sequence,
 * and every empty is the one empty value, so an omitted optional field compares equal to one
 * written empty.
 */
function valuesEqual(left, right) {
    if (Array.isArray(left) && Array.isArray(right)) {
        return left.length === right.length && left.every((item, index) => valuesEqual(item, right[index]));
    }
    if (Array.isArray(left)) {
        return left.length === 1 && valuesEqual(left[0], right);
    }
    if (Array.isArray(right)) {
        return right.length === 1 && valuesEqual(left, right[0]);
    }
    if (isFunctionReference(left) || isFunctionReference(right)) {
        return isFunctionReference(left) && isFunctionReference(right) && left.declaration === right.declaration;
    }
    if (isObject(left) || isObject(right)) {
        if (!isObject(left) || !isObject(right)) {
            return false;
        }
        const leftKeys = Object.keys(left);
        return (leftKeys.length === Object.keys(right).length &&
            leftKeys.every((key) => Object.prototype.hasOwnProperty.call(right, key) && valuesEqual(left[key], right[key])));
    }
    return left === right;
}
// ------------------------------------------------------------------------------------------------
// Boundary normalization
// ------------------------------------------------------------------------------------------------
/**
 * Binds an element's body to its content property.
 *
 * `hasBody` is whether a body was written, which is not the same as whether it produced anything.
 * An element with no body leaves the content property to its declared default; a body that was
 * written and produced nothing binds the empty value, so `<Box>{}</Box>` means what `<Box items={}
 * />` means, and so do a `for` that iterates zero times and a conditional child that is not taken.
 * Falling back to the default for either would disagree with the interpreter and with generated
 * code, which both bind the empty value.
 *
 * <para>The lone-child rule every engine shares: a content property declared `+` or `*` binds an
 * array however many children were supplied, one included; one declared exactly-one or `?` binds
 * a single child as the child itself. That is the child's value lifted once to the declared type,
 * which is what `normalizeValue` does at any sequence site.</para>
 */
function applyContentBinding(input, contentField, fields, content, path, hasBody) {
    if (content.length === 0 && !hasBody) {
        return;
    }
    if (contentField === undefined) {
        fail("nx-ir-boundary-field", `${path} does not accept content.`);
    }
    if (Object.prototype.hasOwnProperty.call(input, contentField)) {
        fail("nx-ir-boundary-field", `${path} field '${contentField}' was supplied both as a property and as content.`);
    }
    const declared = fields.find((field) => field.name === contentField)?.ty;
    const bindsList = declared !== undefined && admitsMany(declared);
    input[contentField] = bindsList || content.length !== 1 ? [...content] : content[0];
}
/**
 * Normalizes host or program input against a declaration's fields, binding each field's slot in
 * `frame` as it goes so a later field's default can read an earlier field.
 *
 * <para>A field that is not written binds its default, or the empty value when its type admits
 * zero — an optional field, `p?:T`, is typed by its read type `T?` — and is otherwise missing. An
 * optional field whose value is empty, written or not, is stored as no key at all: the canonical
 * encoding omits it, and a read of the declared field yields the empty value. Its slot in `frame`
 * still holds the empty value, so a default or a body reads it as such.</para>
 *
 * `linked` and `declaration` are where the fields were declared: defaults are node indices of that
 * module, and a nominal type resolves through that module's table. `options` are the caller's own,
 * so a default is evaluated under the host's limits.
 */
function normalizeFields(program, linked, declaration, fields, input, frame, path, requireExplicit, options) {
    const known = new Set(fields.map((field) => field.name));
    for (const key of Object.keys(input)) {
        if (!known.has(key)) {
            fail("nx-ir-boundary-field", `Unknown ${path} field '${key}'.`);
        }
    }
    // The host's limits hold while a default is evaluated too: a `forRange` in a field default is as
    // much of a loop as one in a body, and a host that lowered the limit to bound untrusted IR means
    // it everywhere.
    const context = { program, linked, declaration, frame, options, depth: 0 };
    const firstSlot = frame.length;
    const output = {};
    fields.forEach((field, offset) => {
        let value;
        if (Object.prototype.hasOwnProperty.call(input, field.name)) {
            value = normalizeValue(context, field.ty, input[field.name], `${path}.${field.name}`);
        }
        else if (!requireExplicit && field.default >= 0) {
            value = normalizeValue(context, field.ty, evalNode(field.default, context), `${path}.${field.name}`);
        }
        else if (!field.isRequired && admitsEmpty(field.ty)) {
            value = [];
        }
        else {
            fail("nx-ir-boundary-field", `Missing required ${path} field '${field.name}'.`, context);
        }
        if (!(admitsEmpty(field.ty) && isEmptyValue(value))) {
            output[field.name] = value;
        }
        frame[firstSlot + offset] = value;
    });
    return output;
}
/**
 * Normalizes the fields of an update record: only the fields supplied, each checked against its
 * declared type. An absent field means "unchanged", so it stays absent; a present empty value —
 * `null` or `[]` from a host, the empty value from the program — clears the field, and is accepted
 * only where the target declares the field optional, which its type says by admitting zero. A
 * cleared field is stored as present and empty, so key presence carries "cleared"; the canonical
 * encoder writes it as `null`.
 */
function normalizePatchFields(context, fields, input, path) {
    const byName = new Map(fields.map((field) => [field.name, field]));
    for (const key of Object.keys(input)) {
        if (!byName.has(key)) {
            fail("nx-ir-boundary-field", `Unknown ${path} field '${key}'.`);
        }
    }
    const output = {};
    for (const field of fields) {
        if (!Object.prototype.hasOwnProperty.call(input, field.name)) {
            continue;
        }
        const value = input[field.name];
        if ((value === null || isEmptyValue(value)) && !admitsEmpty(field.ty)) {
            fail("nx-ir-boundary-type", `Expected ${path}.${field.name} to hold a value; an update record clears a field only where the target declares it optional.`);
        }
        output[field.name] = normalizeValue(context, field.ty, value, `${path}.${field.name}`);
    }
    return output;
}
/**
 * Normalizes a value to a declared type, which is where a value takes its site's occurrence.
 *
 * <para>At a `seq` site the value is read as its items: a single value is one item, the language's
 * one-level lift — `xs={3.0}` at `float64+` is a one-element sequence, and the IR records the
 * value at its own type rather than wrapping it — and a host's `null`, like a missing key, is no
 * items. No items is the empty value where the occurrence admits zero and an error where it does
 * not; more than one is an error where it admits one at most. A `+` or `*` value is always an
 * array and a `?` value that holds an item is the item itself, so `[x]` at a `?` site is `x`. The
 * lift applies once, since an item type is never a `seq`.</para>
 *
 * <para>An exactly-one site takes exactly one value: a one-element sequence reaching it — a `for`
 * over an optional, say — is read as its element, and `null`, the empty value and a longer
 * sequence are what it cannot take. `object` is the one exception: it may hold a sequence,
 * opaquely.</para>
 */
function normalizeValue(context, ty, value, path) {
    if (ty.kind === "seq") {
        const items = value === null ? [] : Array.isArray(value) ? value : [value];
        if (items.length === 0) {
            if (ty.mayBeEmpty) {
                return [];
            }
            fail("nx-ir-boundary-type", `Expected ${path} to hold at least one value, got the empty value.`);
        }
        if (!ty.mayBeMany && items.length > 1) {
            fail("nx-ir-boundary-type", `Expected ${path} to hold at most one value, got a sequence of ${items.length}.`);
        }
        const normalized = items.map((item, index) => normalizeValue(context, ty.item, item, `${path}[${index}]`));
        return ty.mayBeMany ? normalized : normalized[0];
    }
    if (Array.isArray(value) && !isObjectType(ty)) {
        if (value.length === 1) {
            return normalizeValue(context, ty, value[0], path);
        }
        fail("nx-ir-boundary-type", value.length === 0
            ? `Expected ${path} to hold a value, got the empty value.`
            : `Expected ${path} to hold one value, got a sequence of ${value.length}.`);
    }
    if (value === null) {
        fail("nx-ir-boundary-type", `Expected ${path} to hold a value, got null.`);
    }
    switch (ty.kind) {
        case "primitive":
            return normalizePrimitiveValue(ty.name, value, path);
        case "nominal":
            return normalizeNominalValue(context, ty, value, path);
        case "function": {
            // A function value from the program is already a reference; one from a host is the
            // canonical record, resolved to the declaration it names. The checker related the value's
            // declaration to the type by name, so no parameter is re-checked here.
            if (isFunctionReference(value)) {
                return value;
            }
            const record = asFunctionRecord(value);
            if (record === undefined) {
                fail("nx-ir-boundary-type", `Expected ${path} to be a function value.`);
            }
            return internal(resolveFunctionRecord(context.program, record, path));
        }
        default:
            fail("nx-ir-schema", `Unknown type kind '${String(ty.kind)}'.`);
    }
}
function normalizePrimitiveValue(name, value, path) {
    switch (name) {
        case "int":
        case "int64":
        case "float64":
            if (typeof value !== "number") {
                fail("nx-ir-boundary-type", `Expected ${path} to be a number.`);
            }
            return value;
        // A host format such as JSON cannot spell the narrow types, so a number takes the width of its
        // site on the terms a literal written there does.
        case "int32":
            if (typeof value !== "number") {
                fail("nx-ir-boundary-type", `Expected ${path} to be a number.`);
            }
            if (!Number.isInteger(value)) {
                fail("nx-ir-boundary-type", `Expected ${path} to be an int32, got ${value}.`);
            }
            if (value < -2147483648 || value > 2147483647) {
                fail("nx-ir-boundary-type", `Expected ${path} to be an int32, got ${value} (out of range for int32).`);
            }
            return value;
        case "float32": {
            if (typeof value !== "number") {
                fail("nx-ir-boundary-type", `Expected ${path} to be a number.`);
            }
            const rounded = Math.fround(value);
            if (Number.isInteger(value) && rounded !== value) {
                fail("nx-ir-boundary-type", `Expected ${path} to be a float32, got ${value} (not exact as a float32).`);
            }
            return rounded;
        }
        case "string":
            if (typeof value !== "string") {
                fail("nx-ir-boundary-type", `Expected ${path} to be a string.`);
            }
            return value;
        case "boolean":
            if (typeof value !== "boolean") {
                fail("nx-ir-boundary-type", `Expected ${path} to be a boolean.`);
            }
            return value;
        case "object":
            return value;
        default:
            fail("nx-ir-schema", `Unknown primitive type '${name}'.`);
    }
}
function normalizeNominalValue(context, ty, value, path) {
    const { linked, declaration } = resolveReference(context.linked, ty.slot, ty.name);
    const display = declaration.name;
    const kind = declaration.kind;
    const key = `${linked.module.identity}::${declaration.name}`;
    if (kind.tag === "record" && kind.updateTarget !== undefined) {
        // An update record has no subtypes, so a discriminator must name it exactly.
        const object = requireObject(value, path);
        const { $type: discriminator, ...rest } = object;
        if (discriminator !== undefined && discriminator !== display) {
            fail("nx-ir-boundary-type", `Expected ${path} to be a ${display}, got '${String(discriminator)}'.`);
        }
        const declaringContext = { ...context, linked, declaration, frame: [] };
        return { $type: display, ...normalizePatchFields(declaringContext, kind.fields, rest, path) };
    }
    if (kind.tag === "record") {
        const object = requireObject(value, path);
        // The declared type supplies the field list, so a discriminator carried by the value selects
        // nothing and is dropped rather than used. It is still checked before it is dropped, because
        // this path also takes host input: a discriminator naming some other type is a value of the
        // wrong type. A derived value is not the wrong type: `User extends Base` is acceptable
        // wherever `Base` is, and it is normalized against its own schema.
        const discriminator = object.$type;
        if (discriminator !== undefined && discriminator !== display) {
            const subtype = resolveSubtype(context.program, String(discriminator), key, display, path);
            const { $type: _derived, ...derived } = object;
            return {
                $type: subtype.discriminator,
                ...normalizeFields(context.program, subtype.linked, subtype.linked.module.declarationsByName.get(subtype.discriminator) ?? declaration, subtype.fields, derived, [], path, false, context.options),
            };
        }
        // Nothing is an instance of an abstract record.
        if (kind.isAbstract) {
            fail("nx-ir-boundary-type", discriminator === undefined
                ? `Expected ${path} to be a concrete type extending ${display}, got an object with no '$type' discriminator naming one.`
                : `Expected ${path} to be a concrete type extending ${display}, got abstract '${display}'.`);
        }
        const { $type: _discard, ...rest } = object;
        return {
            $type: display,
            ...normalizeFields(context.program, linked, declaration, kind.fields, rest, [], path, false, context.options),
        };
    }
    if (kind.tag === "union") {
        // A constant case arrives as its bare name rather than as a `$type` object.
        if (typeof value === "string") {
            const constantCase = kind.cases.find((item) => item.name === value && item.isConstant);
            if (constantCase === undefined) {
                fail("nx-ir-boundary-type", `Invalid constant union case for ${path}: '${value}' is not a case of ${display}.`);
            }
            return value;
        }
        const object = requireObject(value, path);
        const typeName = object.$type;
        if (typeof typeName !== "string") {
            fail("nx-ir-boundary-type", `Expected ${path} to include a '$type' discriminator.`);
        }
        const prefix = `${display}.`;
        if (!typeName.startsWith(prefix)) {
            fail("nx-ir-boundary-type", `Expected ${path} to be a ${display} union case.`);
        }
        const caseName = typeName.slice(prefix.length);
        const unionCase = kind.cases.find((item) => item.name === caseName);
        if (unionCase === undefined) {
            fail("nx-ir-boundary-type", `Invalid union case '${typeName}' for ${path}.`);
        }
        const { $type: _discard, ...rest } = object;
        return {
            $type: typeName,
            ...normalizeFields(context.program, linked, declaration, unionCase.fields, rest, [], path, false, context.options),
        };
    }
    return value;
}
/**
 * Finds the shape a value's `$type` names, given that a value of `expected` was asked for.
 *
 * The discriminator is a name, not an identity, so this can find more than one shape: two modules
 * may each declare a `Card` extending the same base. That is reported rather than guessed at.
 */
function resolveSubtype(program, discriminator, expected, display, path) {
    const candidates = program.nominalShapesFor(discriminator).filter((shape) => shape.bases.includes(expected));
    if (candidates.length > 1) {
        fail("nx-ir-boundary-type", `Ambiguous subtype at ${path}: ${candidates.length} declarations named '${discriminator}' extend ${display}, and a '$type' discriminator cannot tell them apart.`);
    }
    const only = candidates[0];
    if (only === undefined) {
        fail("nx-ir-boundary-type", `Expected ${path} to be a ${display}, got '${discriminator}'.`);
    }
    if (only.isAbstract) {
        fail("nx-ir-boundary-type", `Expected ${path} to be a concrete type extending ${display}, got abstract '${discriminator}'.`);
    }
    return only;
}
// ------------------------------------------------------------------------------------------------
// Operators
// ------------------------------------------------------------------------------------------------
/** Every operator but `and` and `or`, which `evalNode` applies without evaluating both operands. */
function evalBinary(context, nodeIndex, operator, lhs, rhs) {
    const number = (value) => checkedNumber(context, nodeIndex, value, operator);
    switch (operator) {
        case "add":
            return number(lhs) + number(rhs);
        case "sub":
            return number(lhs) - number(rhs);
        case "mul":
            return number(lhs) * number(rhs);
        case "div": {
            const divisor = number(rhs);
            if (divisor === 0) {
                fail("nx-ir-division-by-zero", "Division by zero.", context, nodeIndex);
            }
            return number(lhs) / divisor;
        }
        // A `float32` operation computed on `float64` operands and rounded to the nearest `float32` is
        // the `float32` operation itself, so its result is the value the interpreter computes.
        case "fadd32":
            return Math.fround(number(lhs) + number(rhs));
        case "fsub32":
            return Math.fround(number(lhs) - number(rhs));
        case "fmul32":
            return Math.fround(number(lhs) * number(rhs));
        case "fdiv32": {
            const divisor = number(rhs);
            if (divisor === 0) {
                fail("nx-ir-division-by-zero", "Division by zero.", context, nodeIndex);
            }
            return Math.fround(number(lhs) / divisor);
        }
        case "idiv": {
            const dividend = checkedInteger(context, nodeIndex, number(lhs), operator);
            const divisor = checkedInteger(context, nodeIndex, number(rhs), operator);
            if (divisor === 0) {
                fail("nx-ir-division-by-zero", "Division by zero.", context, nodeIndex);
            }
            return normalizeSignedZero(Math.trunc(dividend / divisor));
        }
        case "mod": {
            const divisor = number(rhs);
            if (divisor === 0) {
                fail("nx-ir-division-by-zero", "Division by zero.", context, nodeIndex);
            }
            return normalizeSignedZero(number(lhs) % divisor);
        }
        case "imod": {
            const dividend = checkedInteger(context, nodeIndex, number(lhs), operator);
            const divisor = checkedInteger(context, nodeIndex, number(rhs), operator);
            if (divisor === 0) {
                fail("nx-ir-division-by-zero", "Division by zero.", context, nodeIndex);
            }
            return normalizeSignedZero(dividend % divisor);
        }
        case "concat":
            // Strings only: the emitter wraps an operand that is not one in a `text` node, which is
            // where a primitive's text form is decided. Coercing here would print a `float32` as the
            // `float64` it is carried in.
            if (typeof lhs !== "string" || typeof rhs !== "string") {
                fail("nx-ir-operator", "Operator 'concat' requires string operands.", context, nodeIndex);
            }
            return lhs + rhs;
        case "eq":
            return valuesEqual(lhs, rhs);
        case "ne":
            return !valuesEqual(lhs, rhs);
        case "lt":
            return number(lhs) < number(rhs);
        case "le":
            return number(lhs) <= number(rhs);
        case "gt":
            return number(lhs) > number(rhs);
        case "ge":
            return number(lhs) >= number(rhs);
        default:
            fail("nx-ir-operator", `Unknown binary operator '${String(operator)}'.`, context, nodeIndex);
    }
}
/**
 * The canonical text form of a `float32`, which the runtime carries as the `number` it widens to.
 *
 * `String(value)` prints that widening's digits, `0.10000000149011612` for the `float32` nearest
 * `0.1`. This prints the shortest digits that round-trip to the same `float32`, `0.1`, in the same
 * ECMAScript layout every other number prints in. A `float32` needs at most nine significant
 * digits, so the search ends there.
 */
export function float32Text(value) {
    const target = Math.fround(value);
    if (!Number.isFinite(target) || target === 0) {
        return String(target);
    }
    for (let precision = 1; precision <= 9; precision += 1) {
        const candidate = Number(target.toPrecision(precision));
        if (Math.fround(candidate) === target) {
            return String(candidate);
        }
    }
    return String(target);
}
/**
 * A `text` node: the canonical text form of a primitive value, by the static type the node names.
 *
 * For every type but `float32` that form is what `String()` prints for the carried value, which
 * is the ECMAScript number-to-string conversion the form is defined as. An integer outside the
 * safe range is carried as its digits and prints as them.
 */
function primitiveText(context, nodeIndex, value, type) {
    const refuse = () => fail("nx-ir-operator", `A text conversion from '${type}' cannot render ${describeValue(value)}.`, context, nodeIndex);
    switch (type) {
        case "boolean":
            return typeof value === "boolean" ? String(value) : refuse();
        case "float32":
            return typeof value === "number" ? float32Text(value) : refuse();
        case "float64":
            return typeof value === "number" ? String(value) : refuse();
        case "int":
        case "int32":
        case "int64":
            if (typeof value === "number") {
                return String(value);
            }
            if (typeof value === "object" && value !== null && !Array.isArray(value)) {
                const wide = value;
                if (wide.$type === "nx.int" && typeof wide.value === "string") {
                    return wide.value;
                }
            }
            return refuse();
        default:
            return fail("nx-ir-operator", `A text conversion names '${type}', which has no text form.`, context, nodeIndex);
    }
}
function describeValue(value) {
    return Array.isArray(value) ? "a list" : `a ${typeof value}`;
}
function checkedNumber(context, nodeIndex, value, operation) {
    if (typeof value !== "number") {
        fail("nx-ir-number", `Operator '${operation}' requires JavaScript-safe numeric values.`, context, nodeIndex);
    }
    return value;
}
function checkedInteger(context, nodeIndex, value, operation) {
    if (!Number.isInteger(value)) {
        fail("nx-ir-number", `Operator '${operation}' requires integer operands for integer results.`, context, nodeIndex);
    }
    return value;
}
function normalizeSignedZero(value) {
    return Object.is(value, -0) ? 0 : value;
}
/** A condition or a logical operand, which the checker typed as a boolean. */
function requireBoolean(context, nodeIndex, value, what) {
    if (typeof value !== "boolean") {
        fail("nx-ir-type", `Expected a boolean for '${what}', got ${describeValue(value)}.`, context, nodeIndex);
    }
    return value;
}
/** The empty value: the empty sequence, which is also the absent value. */
function isEmptyValue(value) {
    return Array.isArray(value) && value.length === 0;
}
/**
 * The item an optional value holds, or `undefined` when it is empty. A `?` value that holds an
 * item is the item itself; a one-element sequence reaching an optional receiver — a `for` over an
 * optional, before any typed site normalized it — is read as that element.
 */
function optionalItem(value) {
    if (Array.isArray(value) && value.length <= 1) {
        return value[0];
    }
    return value;
}
/**
 * Whether a match arm's pattern matches the scrutinee. The `{}` pattern — the empty value —
 * matches exactly the empty value; a record pattern matches by `$type`; anything else by the
 * language's equality.
 */
function patternMatches(value, pattern) {
    if (isEmptyValue(pattern) || isEmptyValue(value)) {
        return isEmptyValue(pattern) && isEmptyValue(value);
    }
    if (isObject(value) && isObject(pattern) && typeof pattern.$type === "string") {
        return value.$type === pattern.$type;
    }
    return valuesEqual(value, pattern);
}
function isFunctionReference(value) {
    return value instanceof FunctionReferenceValue;
}
function requireObject(value, path) {
    if (!isObject(value)) {
        fail("nx-ir-boundary-type", `Expected ${path} to be an object.`);
    }
    return value;
}
function isObject(value) {
    return typeof value === "object" && value !== null && !Array.isArray(value);
}
/**
 * Raises a runtime diagnostic. With a context, the diagnostic names the declaration the node
 * belongs to, and its span when the artifact carries a debug section.
 */
function fail(code, message, context, nodeIndex) {
    throw new NxIrRuntimeError([diagnostic(code, message, context, nodeIndex)]);
}
function diagnostic(code, message, context, nodeIndex) {
    const output = { severity: "error", code, message };
    if (context !== undefined) {
        const identity = context.linked.module.identity;
        output.declaration = `${identity}::${context.declaration.name}`;
        const span = nodeIndex === undefined ? undefined : context.linked.module.artifact.nodeSpan(nodeIndex);
        if (span !== undefined) {
            output.source = { identity, start: span[0], end: span[1] };
        }
    }
    return output;
}
