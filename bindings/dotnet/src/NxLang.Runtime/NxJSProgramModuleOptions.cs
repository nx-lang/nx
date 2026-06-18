// Copyright (c) Bret Johnson. All rights reserved.
// Licensed under the MIT license. See LICENSE file in the project root for full license information.

namespace NxLang.Nx;

/// <summary>
/// Options for host-neutral JavaScript program-module generation.
/// </summary>
public sealed class NxJSProgramModuleOptions
{
    /// <summary>
    /// Gets or initializes the logical module name recorded in generated metadata.
    /// </summary>
    public string? LogicalModuleName { get; init; }

    /// <summary>
    /// Gets or initializes the runtime module specifier imported by generated source.
    /// </summary>
    public string? RuntimeImportSpecifier { get; init; }
}
