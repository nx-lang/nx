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
import { NX_PRELUDE_VERSION } from "./prelude-image.js";
export declare const NX_IR_SCHEMA_VERSION = 5;
export declare const NX_IR_RUNTIME_ABI = "nx-ir-runtime-v2";
export declare const NX_IR_REQUIRED_FEATURE_UPDATE_RECORDS_V1 = "update-records-v1";
export declare const NX_IR_REQUIRED_FEATURE_PROPERTY_UNIONS_V1 = "property-unions-v1";
export declare const NX_IR_REQUIRED_FEATURE_UPDATE_INTRINSICS_V1 = "update-intrinsics-v1";
export declare const NX_IR_REQUIRED_FEATURE_ACTION_HANDLERS_V1 = "action-handlers-v1";
/** Function types, function references as values, and calls of function-typed values by name. */
export declare const NX_IR_REQUIRED_FEATURE_FUNCTION_VALUES_V1 = "function-values-v1";
/** Iteration over a range: the `forRange` node. Building a range needs no feature. */
export declare const NX_IR_REQUIRED_FEATURE_RANGES_V1 = "ranges-v1";
/**
 * The presence operators — `exists`, `optionalMember` and `coalesce` nodes — and the `{}` match
 * pattern. A `seq` type alone needs no feature: it is a type kind, not a node.
 */
export declare const NX_IR_REQUIRED_FEATURE_OCCURRENCE_V1 = "occurrence-v1";
/** The reserved identity of the NX prelude, the module every NX module sees without an import. */
export declare const NX_PRELUDE_MODULE_IDENTITY = "@nx/prelude.nx";
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
/**
 * The kind numbers of schema 5, as `docs/nx-ir-format.md` assigns them. Kind `0`, the `null`
 * node, was retired with schema 5: the language has no null value, so a reader reports the kind
 * as malformed rather than evaluating it.
 */
export declare const nodeKinds: {
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
    readonly actionHandler: 19;
    readonly text: 20;
    readonly namedCall: 21;
    readonly forRange: 22;
    /** `x?`: `[23, operand]`, true when the operand holds at least one item. */
    readonly exists: 23;
    /** `x?.m`: `[24, receiver, str]`, the empty value when the receiver is empty and `x.m` otherwise. */
    readonly optionalMember: 24;
    /** `x ?? y`: `[25, left, right]`, the left operand when it holds an item and the right one otherwise. */
    readonly coalesce: 25;
};
/**
 * The type kinds. `2` (`array`) and `3` (`nullable`) were retired with schema 5, replaced by
 * `seq`, and stay assigned so no later kind reuses them; a reader reports either as malformed.
 */
export declare const typeKinds: {
    readonly primitive: 0;
    readonly nominal: 1;
    readonly function: 4;
    readonly seq: 5;
};
/**
 * The bits of a `seq` type's occurrence cell: whether the type admits no value and whether it
 * admits more than one. `?` is `1`, `+` is `2` and `*` is `3`; exactly one is never a `seq`.
 */
export declare const occurrenceFlags: {
    readonly empty: 1;
    readonly many: 2;
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
/**
 * A value in the canonical JSON encoding. The empty value — an absent optional, an untaken
 * branch, an empty `for` — is the empty array, and the runtime never holds `null` as an NX value.
 * `null` is here for the host boundary only: a decoder reads it as the empty value wherever the
 * site admits zero, and the encoder writes it for a cleared field of an update record, which is
 * the one place the canonical encoding spells `null`.
 */
export type NxCanonicalValue = null | boolean | number | string | readonly NxCanonicalValue[] | {
    readonly [key: string]: NxCanonicalValue;
};
export declare class NxIrRuntimeError extends Error {
    readonly diagnostics: readonly NxIrDiagnostic[];
    constructor(diagnostics: readonly NxIrDiagnostic[]);
}
/**
 * A type as the image spells it. A `seq` carries an occurrence over an exactly-one item type:
 * `?` is `mayBeEmpty` alone, `+` is `mayBeMany` alone and `*` is both. An item type is never a
 * `seq`, so an exactly-one type is any of the other three.
 */
export type PreparedType = {
    readonly kind: "primitive";
    readonly name: string;
} | {
    readonly kind: "nominal";
    readonly slot: number;
    readonly name: string;
} | {
    readonly kind: "seq";
    readonly item: PreparedType;
    readonly mayBeEmpty: boolean;
    readonly mayBeMany: boolean;
} | {
    readonly kind: "function";
    readonly params: readonly PreparedParam[];
    readonly result: PreparedType;
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
    /** The parameter's read type: `T?` for one declared `p?:T`. */
    readonly ty: PreparedType;
    readonly isContent: boolean;
    /** Whether the parameter carries the `?` mark, so a call may leave it out. */
    readonly isOptional: boolean;
}
/** A parameter of a function declaration, which, unlike one of a function type, may have a default. */
export interface PreparedDeclaredParam extends PreparedParam {
    /**
     * The node the function evaluates for the parameter when a call leaves it out, reading the
     * parameters before it through their slots, or `-1` when it has no default.
     */
    readonly default: number;
}
export interface PreparedUnionCase {
    readonly name: string;
    readonly fields: readonly PreparedField[];
    readonly isConstant: boolean;
}
/** One action a component emits: the local name a parent binds as `on<Name>`, and its record. */
export interface PreparedEmit {
    readonly name: string;
    readonly action: NxIrReference;
}
export type PreparedDeclarationKind = {
    readonly tag: "function";
    readonly params: readonly PreparedDeclaredParam[];
    readonly body: number;
    /** The declared result type the body's value is normalized to, when the source declares one. */
    readonly result: PreparedType | undefined;
    /** The result type, declared or inferred, is a standalone `T?`. */
    readonly isOptionalResult: boolean;
} | {
    readonly tag: "value";
    readonly value: number;
    /** The declared type the value is normalized to, when the source declares one. */
    readonly ty: PreparedType | undefined;
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
    /** The effective emits, inherited included, in declaration order. */
    readonly emits: readonly PreparedEmit[];
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
    /**
     * The most integers one range may hold when a `forRange` iterates it. One million by default,
     * which is the interpreter's operation budget.
     *
     * <para>A range makes an enormous loop one token long, so the count is checked before the body
     * runs at all rather than discovered part-way through.</para>
     */
    readonly maxRangeLength?: number;
}
/** The default of {@link NxRuntimeOptions.maxRangeLength}. */
export declare const NX_DEFAULT_MAX_RANGE_LENGTH = 1000000;
/**
 * A handler as the runtime holds it between the render that created it and the dispatch that
 * runs it. Opaque to hosts: it reaches them only as an `ActionHandler` record in canonical output,
 * and comes back only as a token.
 */
export interface ActionHandlerValue {
    readonly $nxKind: "actionHandler";
    /** The module and declaration the handler node was written in, where its body is evaluated. */
    readonly linked: LinkedModule;
    readonly declaration: PreparedDeclaration;
    /** The declaration key of the component whose emit the handler answers. */
    readonly component: string;
    /** That component's declaration name, for diagnostics. */
    readonly componentName: string;
    readonly emit: string;
    /** The action record the handler accepts; its declaration name is the public action name. */
    readonly action: {
        readonly linked: LinkedModule;
        readonly declaration: PreparedDeclaration;
    };
    readonly actionSlot: number;
    /** The declaration key of the component whose body bound the handler, or `undefined` at the root. */
    readonly owner: string | undefined;
    readonly body: number;
    /** The frame as it stood when the handler was created: the by-value capture. */
    readonly captured: readonly NxCanonicalValue[];
}
/**
 * A component instance: what dispatch needs to run a batch against a component that initialization
 * or a previous dispatch rendered. Opaque to hosts and never modified; every dispatch returns a new
 * one, and the one it was given stays valid for a retry.
 */
export interface NxComponentInstance {
    readonly component: string;
    readonly declaration: PreparedDeclaration;
    /** The declared props, normalized. */
    readonly props: Readonly<Record<string, NxCanonicalValue>>;
    /** The handlers the parent bound, by property name (`onTapped`). Never in the body's scope. */
    readonly handlerProps: ReadonlyMap<string, ActionHandlerValue>;
    readonly state: Readonly<Record<string, NxCanonicalValue>>;
    /** The handlers in the most recent rendered output, by the token that output carries. */
    readonly handlers: ReadonlyMap<string, ActionHandlerValue>;
    /** The render generation the tokens belong to. */
    readonly generation: number;
}
export interface ComponentInitOptions extends NxRuntimeOptions {
    /**
     * The instance whose rendered output the props were read from. Every `ActionHandler` record in
     * the props, at any depth, is replaced by the handler that instance holds under the record's
     * token, which is how a parent's binding reaches a child the host initializes.
     */
    readonly parent?: NxComponentInstance;
    /**
     * The state to use in place of the initial one, validated as a complete state for the component.
     * Initializing again with the state an instance holds and new props is how a host re-renders an
     * instance whose props changed without losing its state.
     */
    readonly state?: Readonly<Record<string, NxCanonicalValue>>;
}
export interface ComponentInitResult {
    readonly rendered: NxCanonicalValue;
    readonly state: Record<string, NxCanonicalValue>;
    readonly instance: NxComponentInstance;
}
export interface ComponentEvaluateResult {
    readonly rendered: NxCanonicalValue;
}
export interface ComponentDispatchResult {
    /** The body rendered against the next state, its handlers carrying fresh tokens. */
    readonly rendered: NxCanonicalValue;
    /** Everything the handlers returned for the host, in dispatch order. */
    readonly effects: readonly NxCanonicalValue[];
    readonly state: Record<string, NxCanonicalValue>;
    readonly instance: NxComponentInstance;
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
export declare function constructComponentDescriptor(program: NxPreparedProgram | NxPreparedModule, name: string, props?: Record<string, NxCanonicalValue>, content?: readonly NxCanonicalValue[], options?: NxRuntimeOptions): NxCanonicalValue;
export declare function initializeComponent(program: NxPreparedProgram | NxPreparedModule, name: string, props?: Record<string, NxCanonicalValue>, options?: ComponentInitOptions): ComponentInitResult;
export declare function evaluateComponent(program: NxPreparedProgram | NxPreparedModule, name: string, props: Record<string, NxCanonicalValue>, state: Record<string, NxCanonicalValue>, options?: NxRuntimeOptions): ComponentEvaluateResult;
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
export declare function dispatchComponentActions(program: NxPreparedProgram | NxPreparedModule, instance: NxComponentInstance, batch: readonly NxCanonicalValue[], options?: NxRuntimeOptions): ComponentDispatchResult;
export declare function normalizeComponentState(program: NxPreparedProgram | NxPreparedModule, name: string, state: Record<string, NxCanonicalValue>, options?: NxRuntimeOptions): Record<string, NxCanonicalValue>;
/**
 * Applies a patch to host-owned component state and returns the validated next state.
 *
 * The patch is either a plain partial state object or the component's own update record,
 * `{ $type: "<Component>.Update", ... }`. Either way a present field replaces the current value, an
 * absent one keeps it, and a present empty value — `null` or `[]` — clears an optional state field,
 * so the next state carries no key for it; for a field that is not optional it is rejected.
 */
export declare function applyComponentStatePatch(program: NxPreparedProgram | NxPreparedModule, name: string, currentState: Record<string, NxCanonicalValue>, patch: Record<string, NxCanonicalValue>, options?: NxRuntimeOptions): Record<string, NxCanonicalValue>;
/**
 * Calls the function a canonical `Function` record names with arguments keyed by parameter name,
 * and returns the canonical result. An argument the function does not declare is dropped, as the
 * subset rule allows; a parameter it declares and the arguments lack is a diagnostic naming it.
 */
export declare function callFunction(program: NxPreparedProgram | NxPreparedModule, value: NxCanonicalValue, args?: Record<string, NxCanonicalValue>, options?: NxRuntimeOptions): NxCanonicalValue;
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
 * `apply(record, update)`: the record with each field present in the update replaced, and each
 * field the update clears — present as `null` or the empty value — left out of the result, which
 * is how the canonical encoding writes an empty optional field; every absent field keeps its
 * value. The update must be the record's own `<Type>.Update`.
 */
export declare function applyUpdate<T extends NxRecordObject>(record: T, update: NxUpdateOf<T>): T;
/**
 * `merge(first, second)`: every field present in either update, the second winning, a cleared
 * field included. A cleared field is `null` in the result, as the canonical encoding spells it.
 */
export declare function mergeUpdates<T extends NxRecordObject>(first: T, second: T): T;
/**
 * `diff(before, after)`: the `<Type>.Update` carrying exactly the fields whose values differ, each
 * with its value from `after`, comparing records and lists structurally. A field either record
 * leaves out is empty there, so a field `after` clears is present and `null` in the result.
 */
export declare function diffRecords<T extends NxRecordObject>(before: T, after: T): NxUpdateOf<T>;
/**
 * `changed(update)`: the names of the fields present in the update, cleared ones included, in the
 * order the update record's declaration in `program` lists them. Fails when the program does not
 * declare the update record, since the order is then unknowable from the value.
 */
export declare function changedFields(update: NxRecordObject, program: NxPreparedProgram): string[];
/**
 * The canonical text form of a `float32`, which the runtime carries as the `number` it widens to.
 *
 * `String(value)` prints that widening's digits, `0.10000000149011612` for the `float32` nearest
 * `0.1`. This prints the shortest digits that round-trip to the same `float32`, `0.1`, in the same
 * ECMAScript layout every other number prints in. A `float32` needs at most nine significant
 * digits, so the search ends there.
 */
export declare function float32Text(value: number): string;
