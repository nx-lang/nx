// Copyright (c) The NX Authors.
// Licensed under the MIT license. See LICENSE file in the project root for full license information.

using System.Text.Json.Serialization;

namespace NxLang.Nx;

/// <summary>
/// Structured metadata for a generated NX IR artifact.
/// </summary>
public sealed class NxIrMetadata
{
    /// <summary>
    /// Gets the workspace identity of the module the artifact carries.
    /// </summary>
    [JsonPropertyName("identity")]
    public string Identity { get; init; } = string.Empty;

    /// <summary>
    /// Gets the fingerprint of the module's source text, as a decimal string. It is also the first
    /// module-table entry's fingerprint in the artifact.
    /// </summary>
    [JsonPropertyName("fingerprint")]
    public string Fingerprint { get; init; } = string.Empty;

    /// <summary>
    /// Gets the NX IR schema version.
    /// </summary>
    [JsonPropertyName("schemaVersion")]
    public int SchemaVersion { get; init; }

    /// <summary>
    /// Gets the IR runtime ABI expected by this artifact.
    /// </summary>
    [JsonPropertyName("runtimeAbi")]
    public string RuntimeAbi { get; init; } = string.Empty;

    /// <summary>
    /// Gets required feature flags that a runtime must support before loading this artifact.
    /// </summary>
    [JsonPropertyName("requiredFeatures")]
    public string[] RequiredFeatures { get; init; } = [];

    /// <summary>
    /// Gets the names of the module's top-level functions, in declaration order.
    /// </summary>
    [JsonPropertyName("functionEntrypoints")]
    public string[] FunctionEntrypoints { get; init; } = [];

    /// <summary>
    /// Gets the names of the module's top-level components, in declaration order.
    /// </summary>
    [JsonPropertyName("componentEntrypoints")]
    public string[] ComponentEntrypoints { get; init; } = [];
}
