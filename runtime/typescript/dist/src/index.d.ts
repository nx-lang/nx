/**
 * The NX IR runtime: prepares schema 3 images, links them by name, and evaluates them.
 *
 * An image carries one module as flat tables of 32-bit cells over one string blob, and the runtime
 * reads it in place. `prepareNxIrModule` validates every section, offset and index of the image and
 * indexes the module's declarations by name; `linkNxIrProgram` resolves the modules the entry names
 * through a host resolver and checks versions and referenced declarations eagerly; every
 * evaluation API then takes the linked program. A prepared module is never copied by linking, so
 * one prepared catalog serves any number of programs.
 */
export declare const NX_IR_SCHEMA_VERSION = 3;
export declare const NX_IR_RUNTIME_ABI = "nx-ir-runtime-v2";
export declare const NX_IR_REQUIRED_FEATURE_UPDATE_RECORDS_V1 = "update-records-v1";
export declare const NX_IR_REQUIRED_FEATURE_PROPERTY_UNIONS_V1 = "property-unions-v1";
export declare const NX_IR_REQUIRED_FEATURE_UPDATE_INTRINSICS_V1 = "update-intrinsics-v1";
/** The cell value that spells an absent optional operand. */
export declare const NX_IR_NONE = 4294967295;
/** The tables whose entries are cells. */
export type NxIrTable = "types" | "constants" | "nodes" | "declarations";
export interface NxIrModuleEntry {
    readonly identity: string;
    readonly version: string;
    readonly fingerprint: string;
}
/**
 * An opened image. The tables are typed-array views over the bytes the host handed in; a string is
 * decoded the first time it is named and remembered.
 *
 * <para>`open` establishes that every section, offset array and string range lies inside the image
 * and that the string blob is UTF-8. The table entries are checked against their layouts by the
 * reader that prepares the module; after that, every index an entry holds is in range.</para>
 */
export declare class NxIrImage {
    #private;
    readonly schemaVersion: number;
    readonly runtimeAbi: string;
    readonly requiredFeatures: readonly string[];
    /** Slot `0` is the image's own module. */
    readonly modules: readonly NxIrModuleEntry[];
    readonly functionEntrypoints: Uint32Array;
    readonly componentEntrypoints: Uint32Array;
    private constructor();
    /**
     * Opens an image, reporting what is wrong with it as diagnostics. A view whose byte offset is
     * not a multiple of four is copied first, since the tables are read as 32-bit cells.
     */
    static open(input: Uint8Array | ArrayBuffer, diagnostics: NxIrDiagnostic[]): NxIrImage | undefined;
    /** The number of strings in the string table. */
    get stringCount(): number;
    /** String `index`, decoded on first use. The index must be inside the table. */
    string(index: number): string;
    /** The number of entries in `table`. */
    entryCount(table: NxIrTable): number;
    /** Entry `index` of `table`, kind cell first, as a view over the pool. The index must be inside the table. */
    entry(table: NxIrTable, index: number): Uint32Array;
    /** Whether the image carries its debug section. */
    get hasDebug(): boolean;
    /** The module's source text, when the debug section is present. */
    get source(): string | undefined;
    /** The byte span of declaration `index` in the source, when the debug section records one. */
    declarationSpan(index: number): readonly [number, number] | undefined;
    /** The byte span of node `index` in the source, when the debug section records one. */
    nodeSpan(index: number): readonly [number, number] | undefined;
}
/** The kind numbers of schema 3, as `docs/nx-ir-format.md` assigns them. */
export declare const nodeKinds: {
    readonly null: 0;
    readonly bool: 1;
    readonly string: 2;
    readonly number: 3;
    readonly slot: 4;
    readonly reference: 5;
    readonly binary: 6;
    readonly unary: 7;
    readonly call: 8;
    readonly intrinsic: 9;
    readonly if: 10;
    readonly ifIs: 11;
    readonly array: 12;
    readonly for: 13;
    readonly member: 14;
    readonly record: 15;
    readonly unionCase: 16;
    readonly element: 17;
    readonly component: 18;
};
export declare const typeKinds: {
    readonly primitive: 0;
    readonly nominal: 1;
    readonly array: 2;
    readonly nullable: 3;
};
export declare const constantKinds: {
    readonly int: 0;
    readonly bigint: 1;
    readonly float: 2;
};
export declare const declarationKinds: {
    readonly function: 0;
    readonly value: 1;
    readonly record: 2;
    readonly component: 3;
    readonly union: 4;
    readonly typeAlias: 5;
};
export type NxDiagnosticSeverity = "error" | "warning" | "info" | "hint";
export interface NxIrSourceSpan {
    /** The identity of the module whose source the span indexes. */
    readonly identity: string;
    readonly start: number;
    readonly end: number;
}
export interface NxIrDiagnostic {
    readonly severity: NxDiagnosticSeverity;
    readonly code: string;
    readonly message: string;
    /** The span of the expression, when the artifact carries a debug section. */
    readonly source?: NxIrSourceSpan;
    /** The declaration the expression belongs to, as `identity::name`, when it is known. */
    readonly declaration?: string;
}
export type NxResult<T> = {
    readonly ok: true;
    readonly value: T;
} | {
    readonly ok: false;
    readonly diagnostics: readonly NxIrDiagnostic[];
};
export type NxCanonicalValue = null | boolean | number | string | readonly NxCanonicalValue[] | {
    readonly [key: string]: NxCanonicalValue;
};
export declare class NxIrRuntimeError extends Error {
    readonly diagnostics: readonly NxIrDiagnostic[];
    constructor(diagnostics: readonly NxIrDiagnostic[]);
}
export type PreparedType = {
    readonly kind: "primitive";
    readonly name: string;
} | {
    readonly kind: "nominal";
    readonly slot: number;
    readonly name: string;
} | {
    readonly kind: "array";
    readonly element: PreparedType;
} | {
    readonly kind: "nullable";
    readonly inner: PreparedType;
};
export interface NxIrReference {
    readonly slot: number;
    readonly name: string;
}
export interface PreparedField {
    readonly name: string;
    readonly ty: PreparedType;
    /** Node index of the default, or `-1`. */
    readonly default: number;
    readonly isContent: boolean;
    readonly isRequired: boolean;
}
export interface PreparedParam {
    readonly name: string;
    readonly ty: PreparedType;
    readonly isContent: boolean;
}
export interface PreparedUnionCase {
    readonly name: string;
    readonly fields: readonly PreparedField[];
    readonly isConstant: boolean;
}
export type PreparedDeclarationKind = {
    readonly tag: "function";
    readonly params: readonly PreparedParam[];
    readonly body: number;
} | {
    readonly tag: "value";
    readonly value: number;
} | {
    readonly tag: "record";
    readonly fields: readonly PreparedField[];
    readonly bases: readonly NxIrReference[];
    readonly isAbstract: boolean;
    readonly updateTarget: NxIrReference | undefined;
} | {
    readonly tag: "component";
    readonly props: readonly PreparedField[];
    readonly state: readonly PreparedField[];
    /** Node index of the body, or `-1`. */
    readonly body: number;
    readonly isAbstract: boolean;
    readonly isExternal: boolean;
} | {
    readonly tag: "union";
    readonly cases: readonly PreparedUnionCase[];
    readonly bases: readonly NxIrReference[];
    readonly propertyTarget: NxIrReference | undefined;
} | {
    readonly tag: "typeAlias";
};
export interface PreparedDeclaration {
    readonly index: number;
    readonly name: string;
    readonly module: NxPreparedModule;
    readonly kind: PreparedDeclarationKind;
}
/**
 * One artifact, validated and indexed. Linking never copies it, so a host prepares a large module
 * once and links any number of programs against it.
 */
export interface NxPreparedModule {
    readonly identity: string;
    readonly version: string;
    readonly fingerprint: string;
    /** The opened image the module is read from. */
    readonly artifact: NxIrImage;
    readonly declarations: readonly PreparedDeclaration[];
    readonly declarationsByName: ReadonlyMap<string, PreparedDeclaration>;
    readonly functionEntrypoints: ReadonlyMap<string, PreparedDeclaration>;
    readonly componentEntrypoints: ReadonlyMap<string, PreparedDeclaration>;
    /** The declaration names this module references in each other module of its table, by slot. */
    readonly externalReferences: ReadonlyMap<number, ReadonlySet<string>>;
    /**
     * Every constructible nominal shape this module declares, keyed by the `$type` a value of it
     * carries. Built once here rather than per link, so preparing a catalog is what costs and
     * linking a snippet against it does not.
     */
    readonly nominalShapeSkeletons: ReadonlyMap<string, readonly NominalShapeSkeleton[]>;
}
/** A prepared module inside one linked program, with its module table resolved. */
export interface LinkedModule {
    readonly module: NxPreparedModule;
    /** Slot `0` is the module itself. */
    readonly slots: readonly LinkedModule[];
}
/**
 * One record or union case of a single module, before any program links it.
 *
 * <para>This is what preparation indexes, so a module the size of a control catalog is walked once
 * however many programs link against it. Its `bases` are still the references the artifact carries,
 * `(slot, name)`; only a link knows which module a slot reaches, so only a link can turn them into
 * the declaration keys a `NominalShape` holds.</para>
 */
export interface NominalShapeSkeleton {
    /** The `$type` a value of this shape carries: a record's name, or `Union.case`. */
    readonly discriminator: string;
    /** The declaring declaration's own name, which is the record's or the union's. */
    readonly declarationName: string;
    readonly fields: readonly PreparedField[];
    readonly bases: readonly NxIrReference[];
    readonly isAbstract: boolean;
}
/**
 * One record or union case as it appears on the wire, in one linked program.
 *
 * `bases` holds declaration keys, `identity::name`, rather than names because that is the only
 * identity that survives separate modules: two records named `Card` are two types, and only the key
 * says which one a base-typed site meant.
 */
export interface NominalShape {
    /** The `$type` a value of this shape carries: a record's name, or `Union.case`. */
    readonly discriminator: string;
    readonly declaration: string;
    readonly fields: readonly PreparedField[];
    readonly bases: readonly string[];
    readonly isAbstract: boolean;
    /** Where the fields' types and defaults are read. */
    readonly linked: LinkedModule;
}
export interface NxPreparedProgram {
    readonly entry: LinkedModule;
    readonly modulesByIdentity: ReadonlyMap<string, LinkedModule>;
    readonly functionEntrypoints: ReadonlyMap<string, PreparedDeclaration>;
    readonly componentEntrypoints: ReadonlyMap<string, PreparedDeclaration>;
    /**
     * The constructible shapes of every linked module that a value's `$type` names. This answers
     * more than one shape when two modules each declare a record of one name; the caller decides
     * what to do about that rather than being handed a guess.
     *
     * <para>Resolved on the question rather than indexed at link time: a linked module already has
     * its own shapes indexed, so this reads one map per module instead of walking every declaration
     * of every module on every link.</para>
     */
    readonly nominalShapesFor: (discriminator: string) => readonly NominalShape[];
}
export interface NxLinkOptions {
    /** Supplies the prepared module of an identity the entry's module table names. */
    readonly resolve: (identity: string) => NxPreparedModule | undefined;
    /**
     * Whether to link a module whose version differs from the one the entry recorded, as long as
     * every declaration the entry references is present. Off by default.
     */
    readonly allowVersionMismatch?: boolean;
}
export interface NxRuntimeOptions {
    readonly maxCallDepth?: number;
}
export interface ComponentInitResult {
    readonly rendered: NxCanonicalValue;
    readonly state: Record<string, NxCanonicalValue>;
}
export interface ComponentEvaluateResult {
    readonly rendered: NxCanonicalValue;
}
export declare function prepareNxIrModule(input: Uint8Array | ArrayBuffer): NxPreparedModule;
/**
 * Opens and validates an image and indexes its module. A view whose byte offset is not a multiple
 * of four is copied first; an `ArrayBuffer` or a fresh `Uint8Array` is read in place.
 */
export declare function tryPrepareNxIrModule(input: Uint8Array | ArrayBuffer): NxResult<NxPreparedModule>;
export declare function linkNxIrProgram(entry: NxPreparedModule, options: NxLinkOptions): NxPreparedProgram;
export declare function tryLinkNxIrProgram(entry: NxPreparedModule, options: NxLinkOptions): NxResult<NxPreparedProgram>;
/**
 * Prepares and links a self-contained artifact: one whose module table holds only itself.
 */
export declare function prepareNxIrProgram(input: Uint8Array | ArrayBuffer): NxPreparedProgram;
export declare function tryPrepareNxIrProgram(input: Uint8Array | ArrayBuffer): NxResult<NxPreparedProgram>;
/** The key that identifies one declaration across a program: its module's identity and its name. */
export declare function declarationKey(linked: LinkedModule, reference: NxIrReference): string;
export declare function evaluateFunction(program: NxPreparedProgram | NxPreparedModule, name: string, args?: readonly NxCanonicalValue[], options?: NxRuntimeOptions): NxCanonicalValue;
export declare function constructComponentDescriptor(program: NxPreparedProgram | NxPreparedModule, name: string, props?: Record<string, NxCanonicalValue>, content?: readonly NxCanonicalValue[]): NxCanonicalValue;
export declare function initializeComponent(program: NxPreparedProgram | NxPreparedModule, name: string, props?: Record<string, NxCanonicalValue>, options?: NxRuntimeOptions): ComponentInitResult;
export declare function evaluateComponent(program: NxPreparedProgram | NxPreparedModule, name: string, props: Record<string, NxCanonicalValue>, state: Record<string, NxCanonicalValue>, options?: NxRuntimeOptions): ComponentEvaluateResult;
export declare function normalizeComponentState(program: NxPreparedProgram | NxPreparedModule, name: string, state: Record<string, NxCanonicalValue>): Record<string, NxCanonicalValue>;
/**
 * Applies a patch to host-owned component state and returns the validated next state.
 *
 * The patch is either a plain partial state object or the component's own update record,
 * `{ $type: "<Component>.Update", ... }`. Either way a present field replaces the current value, an
 * absent one keeps it, and a present `null` sets a nullable field to `null`.
 */
export declare function applyComponentStatePatch(program: NxPreparedProgram | NxPreparedModule, name: string, currentState: Record<string, NxCanonicalValue>, patch: Record<string, NxCanonicalValue>): Record<string, NxCanonicalValue>;
/** A record or update record as the runtime holds it: a `$type` and its fields. */
export type NxRecordObject = {
    readonly $type: string;
    readonly [key: string]: NxCanonicalValue;
};
/** The update record of `T`: the same `$type` family, every field optional. */
export type NxUpdateOf<T extends NxRecordObject> = {
    readonly $type: string;
} & {
    readonly [K in keyof T as K extends "$type" ? never : K]?: T[K];
};
/**
 * `apply(record, update)`: the record with each field present in the update replaced, a present
 * `null` included; every absent field keeps its value. The update must be the record's own
 * `<Type>.Update`.
 */
export declare function applyUpdate<T extends NxRecordObject>(record: T, update: NxUpdateOf<T>): T;
/** `merge(first, second)`: every field present in either update, the second winning. */
export declare function mergeUpdates<T extends NxRecordObject>(first: T, second: T): T;
/**
 * `diff(before, after)`: the `<Type>.Update` carrying exactly the fields whose values differ, each
 * with its value from `after`, comparing records and lists structurally.
 */
export declare function diffRecords<T extends NxRecordObject>(before: T, after: T): NxUpdateOf<T>;
/**
 * `changed(update)`: the names of the fields present in the update, in the order the update
 * record's declaration in `program` lists them. Fails when the program does not declare the
 * update record, since the order is then unknowable from the value.
 */
export declare function changedFields(update: NxRecordObject, program: NxPreparedProgram): string[];
