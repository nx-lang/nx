// Copyright (c) Bret Johnson. All rights reserved.
// Licensed under the MIT license. See LICENSE file in the project root for full license information.

using System.Collections.Generic;
using System.Text.Json.Serialization;

namespace NxLang.Nx;

/// <summary>
/// What to emit from a program artifact. A module's version is not an emit option: it is
/// <see cref="NxWorkspaceModule.Version"/>, given when the workspace is built.
/// </summary>
public sealed class NxIrEmitOptions
{
    /// <summary>
    /// Gets the identities of the modules to emit an artifact for. <see langword="null"/> emits the
    /// entry module alone; an empty list emits every module of the program, entry first.
    /// </summary>
    [JsonPropertyName("modules")]
    [JsonIgnore(Condition = JsonIgnoreCondition.WhenWritingNull)]
    public IReadOnlyList<string>? Modules { get; init; }

    /// <summary>
    /// Gets whether each artifact carries its debug section: spans and source text.
    /// </summary>
    [JsonPropertyName("debug")]
    public bool Debug { get; init; }
}
