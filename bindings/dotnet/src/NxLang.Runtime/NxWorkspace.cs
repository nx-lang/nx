// Copyright (c) Bret Johnson. All rights reserved.
// Licensed under the MIT license. See LICENSE file in the project root for full license information.

using System;
using System.Collections.Generic;
using System.Linq;

namespace NxLang.Nx;

/// <summary>
/// Represents a logical set of NX modules submitted together for validation or program builds.
/// </summary>
public sealed class NxWorkspace
{
    /// <summary>
    /// Creates a workspace from source-backed modules.
    /// </summary>
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
    /// Gets the workspace modules.
    /// </summary>
    public IReadOnlyList<NxWorkspaceModule> Modules { get; }
}
