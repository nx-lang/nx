// Copyright (c) Bret Johnson. All rights reserved.
// Licensed under the MIT license. See LICENSE file in the project root for full license information.

using System;
using System.Text.Json.Serialization;

namespace NxLang.Nx;

/// <summary>
/// Host-neutral JavaScript program-module source and metadata generated from an NX program artifact.
/// </summary>
public sealed class NxGeneratedJSProgramModule
{
    /// <summary>
    /// Gets the generated JavaScript ESM source text.
    /// </summary>
    [JsonPropertyName("sourceText")]
    public string SourceText { get; init; } = string.Empty;

    /// <summary>
    /// Gets the logical module name recorded in generated metadata.
    /// </summary>
    [JsonPropertyName("logicalModuleName")]
    public string LogicalModuleName { get; init; } = string.Empty;

    /// <summary>
    /// Gets the runtime module specifier imported by generated source.
    /// </summary>
    [JsonPropertyName("runtimeImportSpecifier")]
    public string RuntimeImportSpecifier { get; init; } = string.Empty;

    /// <summary>
    /// Gets the NX JavaScript runtime ABI expected by generated source.
    /// </summary>
    [JsonPropertyName("runtimeAbi")]
    public string RuntimeAbi { get; init; } = string.Empty;

    /// <summary>
    /// Gets the originating NX program fingerprint.
    /// </summary>
    [CLSCompliant(false)]
    [JsonPropertyName("programFingerprint")]
    public ulong ProgramFingerprint { get; init; }

    /// <summary>
    /// Gets exported function entrypoints.
    /// </summary>
    [JsonPropertyName("functionExports")]
    public NxGeneratedJSProgramModuleFunctionExport[] FunctionExports { get; init; } = [];

    /// <summary>
    /// Gets exported concrete component and schema entrypoints.
    /// </summary>
    [JsonPropertyName("componentExports")]
    public NxGeneratedJSProgramModuleComponentExport[] ComponentExports { get; init; } = [];
}
