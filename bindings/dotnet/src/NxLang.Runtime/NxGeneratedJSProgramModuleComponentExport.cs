// Copyright (c) Bret Johnson. All rights reserved.
// Licensed under the MIT license. See LICENSE file in the project root for full license information.

using System.Text.Json.Serialization;

namespace NxLang.Nx;

/// <summary>
/// Component and schema export metadata for a generated program module.
/// </summary>
public sealed class NxGeneratedJSProgramModuleComponentExport
{
    /// <summary>
    /// Gets the NX component name.
    /// </summary>
    [JsonPropertyName("componentName")]
    public string ComponentName { get; init; } = string.Empty;

    /// <summary>
    /// Gets the JavaScript component descriptor export name.
    /// </summary>
    [JsonPropertyName("componentExportName")]
    public string ComponentExportName { get; init; } = string.Empty;

    /// <summary>
    /// Gets the JavaScript schema export name.
    /// </summary>
    [JsonPropertyName("schemaExportName")]
    public string SchemaExportName { get; init; } = string.Empty;

    /// <summary>
    /// Gets the JavaScript initial-state helper export name when the component is stateful.
    /// </summary>
    [JsonPropertyName("initialStateExportName")]
    public string? InitialStateExportName { get; init; }

    /// <summary>
    /// Gets the JavaScript render helper export name when the component is stateful.
    /// </summary>
    [JsonPropertyName("renderExportName")]
    public string? RenderExportName { get; init; }
}
