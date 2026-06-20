// Copyright (c) Bret Johnson. All rights reserved.
// Licensed under the MIT license. See LICENSE file in the project root for full license information.

using System.Text.Json.Serialization;

namespace NxLang.Nx;

/// <summary>
/// Module-qualified declaration reference metadata in an NX IR artifact.
/// </summary>
public sealed class NxIrReferenceMetadata
{
    /// <summary>
    /// Gets the stable IR module identifier.
    /// </summary>
    [JsonPropertyName("module")]
    public string Module { get; init; } = string.Empty;

    /// <summary>
    /// Gets the stable IR declaration identifier.
    /// </summary>
    [JsonPropertyName("declaration")]
    public string Declaration { get; init; } = string.Empty;

    /// <summary>
    /// Gets the declaration name.
    /// </summary>
    [JsonPropertyName("name")]
    public string Name { get; init; } = string.Empty;

    /// <summary>
    /// Gets the declaration kind.
    /// </summary>
    [JsonPropertyName("kind")]
    public string Kind { get; init; } = string.Empty;
}
