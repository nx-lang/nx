// Copyright (c) Bret Johnson. All rights reserved.
// Licensed under the MIT license. See LICENSE file in the project root for full license information.

using System;
using System.Collections.Generic;
using System.Linq;

namespace NxLang.Nx;

/// <summary>
/// Represents in-memory NX source modules submitted together for workspace validation or program artifact builds.
/// </summary>
public sealed class NxWorkspace
{
    /// <summary>
    /// Creates a workspace from source-backed modules.
    /// </summary>
    /// <param name="modules">Source modules to include in the workspace.</param>
    /// <exception cref="ArgumentNullException">
    /// Thrown when <paramref name="modules"/> or one of its items is <see langword="null"/>.
    /// </exception>
    public NxWorkspace(IEnumerable<NxWorkspaceModule> modules)
    {
        ArgumentNullException.ThrowIfNull(modules);
        Modules = modules.Select(module =>
        {
            ArgumentNullException.ThrowIfNull(module);
            return module;
        }).ToArray();
    }

    /// <summary>
    /// Gets the workspace modules in the order submitted by the caller.
    /// </summary>
    public IReadOnlyList<NxWorkspaceModule> Modules { get; }
}
