// Copyright (c) Bret Johnson. All rights reserved.
// Licensed under the MIT license. See LICENSE file in the project root for full license information.

using System;
using System.Text.Json.Serialization;

namespace NxLang.Nx;

/// <summary>
/// Structured metadata for a generated NX IR artifact.
/// </summary>
public sealed class NxIrMetadata
{
    /// <summary>
    /// Gets the analyzed NX program fingerprint used for cache keys and equivalence checks.
    /// </summary>
    [CLSCompliant(false)]
    [JsonPropertyName("programFingerprint")]
    public ulong ProgramFingerprint { get; init; }

    /// <summary>
    /// Gets the NX IR schema version.
    /// </summary>
    [JsonPropertyName("schemaVersion")]
    public int SchemaVersion { get; init; }

    /// <summary>
    /// Gets the TypeScript IR runtime ABI expected by this artifact.
    /// </summary>
    [JsonPropertyName("runtimeAbi")]
    public string RuntimeAbi { get; init; } = string.Empty;

    /// <summary>
    /// Gets required feature flags that a runtime must support before loading this artifact.
    /// </summary>
    [JsonPropertyName("requiredFeatures")]
    public string[] RequiredFeatures { get; init; } = [];

    /// <summary>
    /// Gets public function entrypoints in the IR artifact.
    /// </summary>
    [JsonPropertyName("functionEntrypoints")]
    public NxIrEntrypointMetadata[] FunctionEntrypoints { get; init; } = [];

    /// <summary>
    /// Gets public component entrypoints in the IR artifact.
    /// </summary>
    [JsonPropertyName("componentEntrypoints")]
    public NxIrEntrypointMetadata[] ComponentEntrypoints { get; init; } = [];
}
