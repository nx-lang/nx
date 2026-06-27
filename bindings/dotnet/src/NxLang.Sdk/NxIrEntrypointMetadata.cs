// Copyright (c) Bret Johnson. All rights reserved.
// Licensed under the MIT license. See LICENSE file in the project root for full license information.

using System.Text.Json.Serialization;

namespace NxLang.Nx;

/// <summary>
/// Public entrypoint metadata for a generated NX IR artifact.
/// </summary>
public sealed class NxIrEntrypointMetadata
{
    /// <summary>
    /// Gets the public NX entrypoint name.
    /// </summary>
    [JsonPropertyName("name")]
    public string Name { get; init; } = string.Empty;

    /// <summary>
    /// Gets the resolved IR reference for the entrypoint.
    /// </summary>
    [JsonPropertyName("reference")]
    public NxIrReferenceMetadata Reference { get; init; } = new();
}
