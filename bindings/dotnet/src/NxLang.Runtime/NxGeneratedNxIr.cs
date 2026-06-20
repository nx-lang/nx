// Copyright (c) Bret Johnson. All rights reserved.
// Licensed under the MIT license. See LICENSE file in the project root for full license information.

using System.Text.Json.Serialization;

namespace NxLang.Nx;

/// <summary>
/// NX IR JSON source and metadata generated from an NX program artifact.
/// </summary>
public sealed class NxGeneratedNxIr
{
    /// <summary>
    /// Gets the deterministic NX IR JSON document.
    /// </summary>
    [JsonPropertyName("json")]
    public string Json { get; init; } = string.Empty;

    /// <summary>
    /// Gets structured metadata for the generated IR document.
    /// </summary>
    [JsonPropertyName("metadata")]
    public NxIrMetadata Metadata { get; init; } = new();
}
