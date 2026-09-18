/**
 * The NX IR runtime: prepares schema 4 images, links them by name, and evaluates them.
 *
 * An image carries one module as flat tables of 32-bit cells over one string blob, and the runtime
 * reads it in place. `prepareNxIrModule` validates every section, offset and index of the image and
 * indexes the module's declarations by name; `linkNxIrProgram` resolves the modules the entry names
 * through a host resolver and checks versions and referenced declarations eagerly; every
 * evaluation API then takes the linked program. A prepared module is never copied by linking, so
 * one prepared catalog serves any number of programs.
 */
export const NX_IR_SCHEMA_VERSION = 4;
export const NX_IR_RUNTIME_ABI = "nx-ir-runtime-v2";
export const NX_IR_REQUIRED_FEATURE_UPDATE_RECORDS_V1 = "update-records-v1";
export const NX_IR_REQUIRED_FEATURE_PROPERTY_UNIONS_V1 = "property-unions-v1";
export const NX_IR_REQUIRED_FEATURE_UPDATE_INTRINSICS_V1 = "update-intrinsics-v1";
export const NX_IR_REQUIRED_FEATURE_ACTION_HANDLERS_V1 = "action-handlers-v1";
/** Function types, function references as values, and calls of function-typed values by name. */
export const NX_IR_REQUIRED_FEATURE_FUNCTION_VALUES_V1 = "function-values-v1";
const knownFeatures = new Set([
    NX_IR_REQUIRED_FEATURE_UPDATE_RECORDS_V1,
    NX_IR_REQUIRED_FEATURE_PROPERTY_UNIONS_V1,
    NX_IR_REQUIRED_FEATURE_UPDATE_INTRINSICS_V1,
    NX_IR_REQUIRED_FEATURE_ACTION_HANDLERS_V1,
    NX_IR_REQUIRED_FEATURE_FUNCTION_VALUES_V1,
]);
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
/** The kind numbers of schema 4, as `docs/nx-ir-format.md` assigns them. */
export const nodeKinds = {
    null: 0,
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
};
/** The primitive types a `text` node can name: the ones with a canonical text form. */
const textTypes = ["int", "int32", "int64", "float32", "float64", "boolean"];
export const typeKinds = { primitive: 0, nominal: 1, array: 2, nullable: 3, function: 4 };
/** Bit 0 of a function type parameter's flags cell: the parameter takes body content. */
const functionParamFlags = { content: 1 };
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
const STRS = ["str"];
const REFS = ["ref"];
const PROPERTY = ["str", "node"];
const FIELD = ["str", "type", "optNode", "int"];
const PARAM = ["str", "type", "int"];
const ARM = [{ list: NODES }, "node"];
const UNION_CASE = ["str", { list: FIELD }, "int"];
const EMIT = ["str", "ref"];
/** The operands after the kind cell, by table and kind number. */
const layouts = {
    types: [["str"], ["ref"], ["type"], ["type"], ["type", { list: PARAM }]],
    constants: [["i64"], ["str"], ["f64"]],
    nodes: [
        [],
        ["int"],
        ["str"],
        ["const"],
        ["int", "str"],
        ["ref"],
        ["binary", "node", "node"],
        ["unary", "node"],
        ["node", { list: NODES }],
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
    ],
    declarations: [
        ["str", { list: PARAM }, "node"],
        ["str", "node"],
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
            this.#fail(`Unknown ${table.replace(/s$/, "")} kind ${kind} at ${what}.`);
            return;
        }
        // A node's children precede it, and a type's inner type precedes it, so a reader that walks
        // by index never needs to look ahead and no entry can reach itself.
        const bounds = {
            nodes: table === "nodes" ? index : this.#image.entryCount("nodes"),
            types: table === "types" ? index : this.#image.entryCount("types"),
        };
        if (this.#validateSeq(cursor, layout, bounds, what) && !cursor.finished) {
            this.#fail(`${what} has ${cursor.cells.length - cursor.position} cell(s) more than its layout.`);
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
            case "const":
                return inRange("constant", cell, this.#image.entryCount("constants"));
            case "node":
                return inRange("node", cell, bounds.nodes);
            case "optNode":
                return cell === NX_IR_NONE || inRange("node", cell, bounds.nodes);
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
            case typeKinds.array:
                prepared = { kind: "array", element: this.type(entry[1]) };
                break;
            case typeKinds.function: {
                const result = this.type(entry[1]);
                const params = [];
                const count = entry[2];
                for (let position = 3; position < 3 + count * 3; position += 3) {
                    params.push({
                        name: this.#image.string(entry[position]),
                        ty: this.type(entry[position + 1]),
                        isContent: (entry[position + 2] & functionParamFlags.content) !== 0,
                    });
                }
                prepared = { kind: "function", params, result };
                break;
            }
            default:
                prepared = { kind: "nullable", inner: this.type(entry[1]) };
                break;
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
                    return { name: paramName, ty, isContent: cursor.next() !== 0 };
                });
                prepared = { tag: "function", params, body: cursor.next() };
                break;
            }
            case declarationKinds.value:
                prepared = { tag: "value", value: cursor.next() };
                break;
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
            const resolved = options.resolve(entryTable.identity);
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
    return canonicalizeRendered(invokeFunction(linkedProgram, linkedProgram.entry, declaration, args, options, 0)).value;
}
export function constructComponentDescriptor(program, name, props = {}, content = []) {
    const linkedProgram = programOf(program);
    const { declaration, component } = componentDeclaration(linkedProgram, name);
    const { fields: input, handlers } = splitHandlerProperties(linkedProgram.entry, declaration, props, `${name} props`);
    const contentField = component.props.find((field) => field.isContent);
    applyContentBinding(input, contentField?.name, component.props, content, name);
    const normalized = normalizeFields(linkedProgram, linkedProgram.entry, declaration, component.props, input, [], `${name} props`, false);
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
    const normalizedProps = normalizeFields(linkedProgram, linkedProgram.entry, declaration, component.props, fields, frame, path, false);
    const state = options.state === undefined
        ? normalizeFields(linkedProgram, linkedProgram.entry, declaration, component.state, {}, frame, `${name} state`, false)
        : normalizeFields(linkedProgram, linkedProgram.entry, declaration, component.state, { ...options.state }, frame, `${name} state`, true);
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
    normalizeFields(linkedProgram, linkedProgram.entry, declaration, component.props, fields, frame, path, false);
    normalizeFields(linkedProgram, linkedProgram.entry, declaration, component.state, state, frame, `${name} state`, true);
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
            const results = invokeHandler(linkedProgram, handler, requireObject(object.action ?? null, `${path}.action`), owned ? { component, state: working } : undefined, options);
            for (const result of results) {
                if (owned && isUpdateRecordFor(result, linkedProgram, ownerKey)) {
                    working = patchComponentState(linkedProgram, linked, declaration, component, working, result);
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
        const action = normalizeActionInput(linkedProgram, resolveReference(linked, emit.action.slot, emit.action.name), object, `${typeName} action`);
        const handler = instance.handlerProps.get(handlerPropertyName(emit.name));
        if (handler !== undefined) {
            // The parent bound this handler, so everything it returns belongs to the parent, via the host.
            effects.push(...invokeHandler(linkedProgram, handler, action, undefined, options));
        }
    });
    // The body sees the declared props and the state, as it did at initialization; the handler
    // props the parent bound are carried by the instance but were never in scope.
    const frame = [];
    component.props.forEach((field, index) => {
        frame[index] = instance.props[field.name] ?? null;
    });
    component.state.forEach((field, index) => {
        frame[component.props.length + index] = working[field.name] ?? null;
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
export function normalizeComponentState(program, name, state) {
    const linkedProgram = programOf(program);
    const { declaration, component } = componentDeclaration(linkedProgram, name);
    const frame = new Array(component.props.length).fill(null);
    return normalizeFields(linkedProgram, linkedProgram.entry, declaration, component.state, state, frame, `${name} state`, true);
}
/**
 * Applies a patch to host-owned component state and returns the validated next state.
 *
 * The patch is either a plain partial state object or the component's own update record,
 * `{ $type: "<Component>.Update", ... }`. Either way a present field replaces the current value, an
 * absent one keeps it, and a present `null` sets a nullable field to `null`.
 */
export function applyComponentStatePatch(program, name, currentState, patch) {
    const linkedProgram = programOf(program);
    const { declaration, component } = componentDeclaration(linkedProgram, name);
    return patchComponentState(linkedProgram, linkedProgram.entry, declaration, component, currentState, patch);
}
/** Applies a patch to a component's state with full validation: what `applyComponentStatePatch` and dispatch share. */
function patchComponentState(program, linked, declaration, component, currentState, patch) {
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
    const frame = new Array(component.props.length).fill(null);
    return normalizeFields(program, linked, declaration, component.state, { ...currentState, ...fields }, frame, `${name} state`, true);
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
function invokeFunction(program, linked, declaration, args, options, depth) {
    const maxCallDepth = options.maxCallDepth ?? 100;
    if (depth > maxCallDepth) {
        fail("nx-ir-resource-limit", `Maximum NX IR call depth ${maxCallDepth} was exceeded.`);
    }
    const kind = declaration.kind;
    if (kind.tag !== "function") {
        fail("nx-ir-call", `'${declaration.name}' is not a function.`);
    }
    if (args.length !== kind.params.length) {
        fail("nx-ir-arguments", `Function '${declaration.name}' expected ${kind.params.length} arguments, got ${args.length}.`);
    }
    const frame = [];
    const context = { program, linked, declaration, frame, options, depth };
    kind.params.forEach((param, index) => {
        // Body content reaches the content parameter as the list of children the emitter gathered,
        // and a child that is itself a list is spliced, as it is for a component's content.
        const arg = args[index];
        const value = param.isContent && Array.isArray(arg) ? spliceContent(arg) : arg;
        frame[index] = normalizeValue(context, param.ty, value, param.name);
    });
    return evalNode(kind.body, context);
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
    return spliceContent(nodesAt(context, entry, at).values);
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
        case nodeKinds.null:
            return null;
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
                const left = truthy(evalNode(entry[2], context));
                if (left === (operator === "or")) {
                    return left;
                }
                return truthy(evalNode(entry[3], context));
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
                    return !truthy(operand);
                default:
                    fail("nx-ir-operator", `Unknown unary operator '${String(entry[1])}'.`, context, index);
            }
        }
        // eslint-disable-next-line no-fallthrough
        case nodeKinds.call:
            return evalCall(context, index, entry);
        case nodeKinds.intrinsic:
            return evalIntrinsic(context, index, entry);
        case nodeKinds.if: {
            const condition = evalNode(entry[1], context);
            if (truthy(condition)) {
                return evalNode(entry[2], context);
            }
            return entry[3] === NX_IR_NONE ? null : evalNode(entry[3], context);
        }
        case nodeKinds.ifIs: {
            const scrutinee = evalNode(entry[1], context);
            const arms = entry[2];
            let position = 3;
            for (let arm = 0; arm < arms; arm += 1) {
                const patterns = entry[position];
                let matched = false;
                for (let pattern = position + 1; pattern < position + 1 + patterns; pattern += 1) {
                    if (!matched && patternMatches(scrutinee, evalNode(entry[pattern], context))) {
                        matched = true;
                    }
                }
                const body = entry[position + 1 + patterns];
                if (matched) {
                    return evalNode(body, context);
                }
                position += patterns + 2;
            }
            const otherwise = entry[position];
            return otherwise === NX_IR_NONE ? null : evalNode(otherwise, context);
        }
        case nodeKinds.array:
            return nodesAt(context, entry, 1).values;
        case nodeKinds.for: {
            const iterable = evalNode(entry[5], context);
            if (!Array.isArray(iterable)) {
                fail("nx-ir-for", "For expression iterable must evaluate to an array.", context, index);
            }
            const itemSlot = entry[1];
            const indexSlot = entry[3];
            return iterable.map((item, position) => {
                context.frame[itemSlot] = item;
                if (indexSlot !== NX_IR_NONE) {
                    context.frame[indexSlot] = position;
                }
                return evalNode(entry[6], context);
            });
        }
        case nodeKinds.member: {
            const base = evalNode(entry[1], context);
            const member = image.string(entry[2]);
            const object = requireObject(base, "member access");
            if (!Object.prototype.hasOwnProperty.call(object, member)) {
                fail("nx-ir-member", `Object does not contain member '${member}'.`, context, index);
            }
            return object[member];
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
        return evalNode(declaration.kind.value, {
            ...context,
            linked,
            declaration,
            frame: [],
        });
    }
    fail("nx-ir-reference", `Declaration '${name}' cannot be used as a value.`, context, nodeIndex);
}
function evalCall(context, nodeIndex, entry) {
    const callee = evalNode(entry[1], context);
    if (!isFunctionReference(callee)) {
        fail("nx-ir-call", "NX IR call callee did not evaluate to a function reference.", context, nodeIndex);
    }
    const args = nodesAt(context, entry, 2).values;
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
 * parameter the declaration names must be present.
 */
function invokeFunctionByName(program, callee, args, options, depth) {
    const kind = callee.declaration.kind;
    if (kind.tag !== "function") {
        fail("nx-ir-call", `'${callee.declaration.name}' is not a function.`);
    }
    const positional = kind.params.map((param) => {
        if (!Object.prototype.hasOwnProperty.call(args, param.name)) {
            fail("nx-ir-arguments", `Function '${callee.declaration.name}' requires argument '${param.name}'.`);
        }
        return args[param.name];
    });
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
    return canonicalizeRendered(invokeFunctionByName(linkedProgram, callee, args, options, 0)).value;
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
    applyContentBinding(properties, contentField, record.fields, content, name);
    const normalized = record.updateTarget !== undefined
        ? normalizePatchFields(context, record.fields, properties, name)
        : normalizeFields(context.program, linked, declaration, record.fields, properties, [], name, false);
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
    applyContentBinding(properties, contentField, unionCase.fields, content, path);
    const normalized = normalizeFields(context.program, linked, declaration, unionCase.fields, properties, [], path, false);
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
    applyContentBinding(props, contentField, component.props, content, name);
    const normalized = normalizeFields(context.program, linked, declaration, component.props, props, [], `${name} props`, false);
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
        const output = {};
        for (const key of Object.keys(item)) {
            output[key] = canonical.get(key);
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
function normalizeActionInput(program, action, input, path) {
    const expected = action.declaration.name;
    const kind = action.declaration.kind;
    if (kind.tag !== "record") {
        fail("nx-ir-type", `'${expected}' is not an action record.`);
    }
    const { $type: discriminator, ...rest } = input;
    if (discriminator !== undefined && discriminator !== expected) {
        fail("nx-ir-type", `Expected ${path} to be a '${expected}' action, got '${String(discriminator)}'.`);
    }
    return { $type: expected, ...normalizeFields(program, action.linked, action.declaration, kind.fields, rest, [], path, false) };
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
    const normalizedAction = normalizeActionInput(program, handler.action, action, `${label} action`);
    const frame = handler.captured.slice();
    if (live !== undefined) {
        live.component.state.forEach((field, index) => {
            frame[live.component.props.length + index] = live.state[field.name] ?? null;
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
    if (value === null || typeof value !== "object" || Array.isArray(value)) {
        fail("nx-ir-intrinsic", `Intrinsic '${intrinsic}' expects a record value, got ${JSON.stringify(value)}.`);
    }
    const record = value;
    if (typeof record.$type !== "string") {
        fail("nx-ir-intrinsic", `Intrinsic '${intrinsic}' expects a record value, got ${JSON.stringify(value)}.`);
    }
    return record;
}
/**
 * `apply(record, update)`: the record with each field present in the update replaced, a present
 * `null` included; every absent field keeps its value. The update must be the record's own
 * `<Type>.Update`.
 */
export function applyUpdate(record, update) {
    return applyRecordUpdate(record, update);
}
/** `merge(first, second)`: every field present in either update, the second winning. */
export function mergeUpdates(first, second) {
    return mergeUpdateRecords(first, second);
}
/**
 * `diff(before, after)`: the `<Type>.Update` carrying exactly the fields whose values differ, each
 * with its value from `after`, comparing records and lists structurally.
 */
export function diffRecords(before, after) {
    return diffRecordValues(before, after);
}
/**
 * `changed(update)`: the names of the fields present in the update, in the order the update
 * record's declaration in `program` lists them. Fails when the program does not declare the
 * update record, since the order is then unknowable from the value.
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
function applyRecordUpdate(record, update) {
    const expected = `${record.$type}.Update`;
    if (update.$type !== expected) {
        fail("nx-ir-intrinsic", `Cannot apply '${update.$type}' to a '${record.$type}': only '${expected}' patches it.`);
    }
    const { $type: _update, ...fields } = update;
    return { ...record, ...fields };
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
    // A field either record leaves out reads as `null`, so a field only one of them carries still
    // compares.
    for (const key of new Set([...Object.keys(before), ...Object.keys(after)])) {
        if (key === "$type") {
            continue;
        }
        const next = after[key] ?? null;
        if (!valuesEqual(before[key] ?? null, next)) {
            output[key] = next;
        }
    }
    return output;
}
function valuesEqual(left, right) {
    if (Array.isArray(left) || Array.isArray(right)) {
        return (Array.isArray(left) &&
            Array.isArray(right) &&
            left.length === right.length &&
            left.every((item, index) => valuesEqual(item, right[index] ?? null)));
    }
    if (left !== null && typeof left === "object") {
        if (right === null || typeof right !== "object") {
            return false;
        }
        const leftRecord = left;
        const rightRecord = right;
        const leftKeys = Object.keys(leftRecord);
        const rightKeys = Object.keys(rightRecord);
        return (leftKeys.length === rightKeys.length &&
            leftKeys.every((key) => valuesEqual(leftRecord[key] ?? null, rightRecord[key] ?? null)));
    }
    return left === right;
}
// ------------------------------------------------------------------------------------------------
// Boundary normalization
// ------------------------------------------------------------------------------------------------
function applyContentBinding(input, contentField, fields, content, path) {
    if (content.length === 0) {
        return;
    }
    if (contentField === undefined) {
        fail("nx-ir-boundary-field", `${path} does not accept content.`);
    }
    if (Object.prototype.hasOwnProperty.call(input, contentField)) {
        fail("nx-ir-boundary-field", `${path} field '${contentField}' was supplied both as a property and as content.`);
    }
    const declared = fields.find((field) => field.name === contentField)?.ty;
    const bindsList = declared !== undefined && isListType(declared);
    input[contentField] = bindsList || content.length > 1 ? [...content] : content[0];
}
/**
 * Whether a content property's declared type holds a list, looking through nullability.
 *
 * A list-typed content property binds a list however many children were supplied, including exactly
 * one. Collapsing a single child to the child itself would then fail normalization, and it would
 * disagree with the interpreter, which lists the single child.
 */
function isListType(ty) {
    if (ty.kind === "nullable") {
        return isListType(ty.inner);
    }
    return ty.kind === "array";
}
/**
 * Normalizes host or program input against a declaration's fields, binding each field's slot in
 * `frame` as it goes so a later field's default can read an earlier field.
 *
 * `linked` and `declaration` are where the fields were declared: defaults are node indices of that
 * module, and a nominal type resolves through that module's table.
 */
function normalizeFields(program, linked, declaration, fields, input, frame, path, requireExplicit) {
    const known = new Set(fields.map((field) => field.name));
    for (const key of Object.keys(input)) {
        if (!known.has(key)) {
            fail("nx-ir-boundary-field", `Unknown ${path} field '${key}'.`);
        }
    }
    const context = { program, linked, declaration, frame, options: {}, depth: 0 };
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
        else if (!field.isRequired && !requireExplicit) {
            value = null;
        }
        else {
            fail("nx-ir-boundary-field", `Missing required ${path} field '${field.name}'.`, context);
        }
        output[field.name] = value;
        frame[firstSlot + offset] = value;
    });
    return output;
}
/**
 * Normalizes the fields of an update record: only the fields supplied, each checked against its
 * declared type. An absent field means "unchanged", so it stays absent; a present `null` is
 * accepted only where the field is nullable.
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
        if (value === null && field.ty.kind !== "nullable") {
            fail("nx-ir-boundary-type", `Expected ${path}.${field.name} to be non-null; an update record sets a field to null only where the field is nullable.`);
        }
        output[field.name] = normalizeValue(context, field.ty, value, `${path}.${field.name}`);
    }
    return output;
}
function normalizeValue(context, ty, value, path) {
    switch (ty.kind) {
        case "primitive":
            return normalizePrimitiveValue(ty.name, value, path);
        case "nominal":
            return normalizeNominalValue(context, ty, value, path);
        case "array": {
            // A single value at a list-typed site is a list of one. That is the language's rule, not a
            // leniency: `Shadows={ <SkiaShadow /> }` and `xs={3.0}` both evaluate to one-element lists
            // under the interpreter, and the IR records the value at its own type rather than wrapping
            // it, leaving the coercion to normalization.
            const items = Array.isArray(value) ? value : [value];
            return items.map((item, index) => normalizeValue(context, ty.element, item, `${path}[${index}]`));
        }
        case "nullable":
            return value === null ? null : normalizeValue(context, ty.inner, value, path);
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
                ...normalizeFields(context.program, subtype.linked, subtype.linked.module.declarationsByName.get(subtype.discriminator) ?? declaration, subtype.fields, derived, [], path, false),
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
            ...normalizeFields(context.program, linked, declaration, kind.fields, rest, [], path, false),
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
            ...normalizeFields(context.program, linked, declaration, unionCase.fields, rest, [], path, false),
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
            return deepEqual(lhs, rhs);
        case "ne":
            return !deepEqual(lhs, rhs);
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
    if (value === null) {
        return "null";
    }
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
function truthy(value) {
    return Boolean(value);
}
function patternMatches(value, pattern) {
    if (isObject(value) && isObject(pattern) && typeof pattern.$type === "string") {
        return value.$type === pattern.$type;
    }
    return deepEqual(value, pattern);
}
function deepEqual(lhs, rhs) {
    return JSON.stringify(lhs) === JSON.stringify(rhs);
}
function isFunctionReference(value) {
    return value instanceof FunctionReferenceValue;
}
function requireObject(value, path) {
    if (!isObject(value) || Array.isArray(value)) {
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
