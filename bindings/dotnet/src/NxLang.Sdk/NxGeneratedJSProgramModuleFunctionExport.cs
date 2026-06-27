// Copyright (c) Bret Johnson. All rights reserved.
// Licensed under the MIT license. See LICENSE file in the project root for full license information.

using System.Text.Json.Serialization;

namespace NxLang.Nx;

/// <summary>
/// Function entrypoint export metadata for a generated program module.
/// </summary>
public sealed class NxGeneratedJSProgramModuleFunctionExport
{
    /// <summary>
    /// Gets the NX entrypoint name.
    /// </summary>
    [JsonPropertyName("entrypointName")]
    public string EntrypointName { get; init; } = string.Empty;

    /// <summary>
    /// Gets the JavaScript export name.
    /// </summary>
    [JsonPropertyName("exportName")]
    public string ExportName { get; init; } = string.Empty;
}
