export declare const NX_IR_FORMAT_ID = "nx-ir-json";
export declare const NX_IR_SCHEMA_VERSION = 2;
export declare const NX_IR_RUNTIME_ABI = "nx-ir-runtime-v1";
export type NxDiagnosticSeverity = "error" | "warning" | "info" | "hint";
export interface NxIrDiagnostic {
    readonly severity: NxDiagnosticSeverity;
    readonly code: string;
    readonly message: string;
    readonly path?: string;
    readonly source?: NxIrSourceSpan;
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
export interface NxIrProgram {
    readonly format: string;
    readonly schemaVersion: number;
    readonly runtimeAbi: string;
    readonly programFingerprint: string;
    readonly requiredFeatures: readonly string[];
    readonly functionEntrypoints: readonly NxIrEntrypoint[];
    readonly componentEntrypoints: readonly NxIrEntrypoint[];
    readonly modules: readonly NxIrModule[];
    readonly sources: readonly NxIrSourceEntry[];
}
export interface NxIrEntrypoint {
    readonly name: string;
    readonly reference: NxIrReference;
}
export interface NxIrModule {
    readonly id: string;
    readonly runtimeId: number;
    readonly provenance: {
        readonly kind: string;
        readonly [key: string]: unknown;
    };
    readonly imports: readonly NxIrReference[];
    readonly declarations: readonly NxIrDeclaration[];
}
export interface NxIrReference {
    readonly module: string;
    readonly declaration: string;
    readonly name: string;
    readonly kind: string;
}
export interface NxIrDeclaration {
    readonly id: string;
    readonly reference: NxIrReference;
    readonly span: NxIrSourceSpan;
    readonly kind: NxIrDeclarationKind;
}
export type NxIrDeclarationKind = NxIrFunctionDeclaration | NxIrValueDeclaration | NxIrRecordDeclaration | NxIrComponentDeclaration | NxIrUnionDeclaration | NxIrTypeAliasDeclaration;
export interface NxIrFunctionDeclaration {
    readonly tag: "function";
    readonly params: readonly NxIrParam[];
    readonly body: NxIrExpression;
    readonly returnType?: NxIrSemanticType;
}
export interface NxIrValueDeclaration {
    readonly tag: "value";
    readonly value: NxIrExpression;
    readonly ty?: NxIrSemanticType;
}
export interface NxIrRecordDeclaration {
    readonly tag: "record";
    readonly fields: readonly NxIrRecordField[];
    /**
     * The record's abstract bases, nearest first.
     *
     * Fields arrive already flattened, so this answers only what flattening cannot: a value stamped
     * with this record's name is acceptable wherever any of these is expected.
     */
    readonly bases?: readonly NxIrReference[];
    /**
     * Whether the record was declared `abstract`, and so has no values of its own.
     *
     * A base-typed site accepts a value of a record that extends this one, never one of this one.
     */
    readonly isAbstract?: boolean;
}
export interface NxIrComponentDeclaration {
    readonly tag: "component";
    readonly isAbstract: boolean;
    readonly isExternal: boolean;
    readonly props: readonly NxIrComponentField[];
    readonly state: readonly NxIrComponentField[];
    readonly body?: NxIrExpression | null;
}
export interface NxIrUnionDeclaration {
    readonly tag: "union";
    readonly cases: readonly NxIrUnionCase[];
    /** The union's abstract bases, nearest first, inherited by every case. */
    readonly bases?: readonly NxIrReference[];
}
export interface NxIrTypeAliasDeclaration {
    readonly tag: "typeAlias";
}
export interface NxIrParam {
    readonly name: string;
    readonly slot: string;
    readonly ty: NxIrTypeRef;
    readonly isContent: boolean;
    readonly span: NxIrSourceSpan;
}
export interface NxIrRecordField {
    readonly name: string;
    readonly slot: string;
    readonly ty: NxIrTypeRef;
    readonly isContent: boolean;
    readonly isRequired: boolean;
    readonly default?: NxIrExpression | null;
    readonly span: NxIrSourceSpan;
}
export interface NxIrComponentField extends NxIrRecordField {
    readonly ownerModule: string;
}
export interface NxIrUnionCase {
    readonly name: string;
    readonly fields: readonly NxIrRecordField[];
    /**
     * Whether this case declares no fields in a union that declares no base.
     *
     * A constant case carries nothing beyond its own name, so its wire form is that bare string
     * rather than a `$type` object.
     */
    readonly isConstant: boolean;
    readonly span: NxIrSourceSpan;
}
export interface NxIrExpression {
    readonly id: string;
    readonly span: NxIrSourceSpan;
    readonly ty?: NxIrSemanticType;
    readonly op: {
        readonly tag: string;
        readonly [key: string]: unknown;
    };
}
export interface NxIrTypeRef {
    readonly kind: string;
    readonly name?: string;
    readonly reference?: NxIrReference;
    readonly display?: string;
    readonly element?: NxIrTypeRef;
    readonly inner?: NxIrTypeRef;
    readonly params?: readonly NxIrTypeRef[];
    readonly returnType?: NxIrTypeRef;
}
export interface NxIrSemanticType {
    readonly display: string;
    readonly shape: {
        readonly kind: string;
        readonly [key: string]: unknown;
    };
}
export interface NxIrSourceSpan {
    readonly source?: string;
    readonly start: number;
    readonly end: number;
}
export interface NxIrSourceEntry {
    readonly identity: string;
    readonly source: string;
}
export interface NxPreparedProgram {
    readonly ir: NxIrProgram;
    readonly modulesById: ReadonlyMap<string, NxIrModule>;
    readonly declarationsById: ReadonlyMap<string, PreparedDeclaration>;
    readonly functionEntrypoints: ReadonlyMap<string, PreparedDeclaration>;
    readonly componentEntrypoints: ReadonlyMap<string, PreparedDeclaration>;
    readonly sourcesByIdentity: ReadonlyMap<string, string>;
    /**
     * Every constructible nominal shape, keyed by the `$type` a value of it carries.
     *
     * A value arriving at a base-typed boundary names its own type and nothing more, so this is how
     * that name is turned back into the schema to normalize it with. One key can hold several shapes:
     * two modules may each declare a record of the same name.
     */
    readonly nominalShapesByDiscriminator: ReadonlyMap<string, readonly NominalShape[]>;
}
/**
 * One record or union case as it appears on the wire.
 *
 * <para>`bases` holds declaration ids rather than names because that is the only identity that
 * survives separate modules: two records named `Card` are two types, and only the id says which
 * one a base-typed site meant.</para>
 */
export interface NominalShape {
    /** The `$type` a value of this shape carries: a record's name, or `Union.case`. */
    readonly discriminator: string;
    readonly declaration: string;
    readonly fields: readonly NxIrRecordField[];
    readonly bases: readonly string[];
    /** Whether this shape is an abstract record, which no value may be an instance of. */
    readonly isAbstract: boolean;
}
export interface PreparedDeclaration {
    readonly module: NxIrModule;
    readonly declaration: NxIrDeclaration;
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
export declare class NxIrRuntimeError extends Error {
    readonly diagnostics: readonly NxIrDiagnostic[];
    constructor(diagnostics: readonly NxIrDiagnostic[]);
}
export declare function prepareNxIrProgram(input: string | NxIrProgram): NxPreparedProgram;
export declare function tryPrepareNxIrProgram(input: string | NxIrProgram): NxResult<NxPreparedProgram>;
export declare function evaluateFunction(program: NxPreparedProgram, name: string, args?: readonly NxCanonicalValue[], options?: NxRuntimeOptions): NxCanonicalValue;
export declare function constructComponentDescriptor(program: NxPreparedProgram, name: string, props?: Record<string, NxCanonicalValue>, content?: readonly NxCanonicalValue[]): NxCanonicalValue;
export declare function initializeComponent(program: NxPreparedProgram, name: string, props?: Record<string, NxCanonicalValue>, options?: NxRuntimeOptions): ComponentInitResult;
export declare function evaluateComponent(program: NxPreparedProgram, name: string, props: Record<string, NxCanonicalValue>, state: Record<string, NxCanonicalValue>, options?: NxRuntimeOptions): ComponentEvaluateResult;
export declare function normalizeComponentState(program: NxPreparedProgram, name: string, state: Record<string, NxCanonicalValue>): Record<string, NxCanonicalValue>;
export declare function applyComponentStatePatch(program: NxPreparedProgram, name: string, currentState: Record<string, NxCanonicalValue>, patch: Record<string, NxCanonicalValue>): Record<string, NxCanonicalValue>;
