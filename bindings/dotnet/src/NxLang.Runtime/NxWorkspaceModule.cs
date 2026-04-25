// Copyright (c) Bret Johnson. All rights reserved.
// Licensed under the MIT license. See LICENSE file in the project root for full license information.

using System;
using System.Text;

namespace NxLang.Nx;

/// <summary>
/// Represents one source-backed module in a logical NX workspace.
/// </summary>
public sealed class NxWorkspaceModule
{
    /// <summary>
    /// Creates a workspace module from a logical identity and UTF-8 source bytes.
    /// </summary>
    public NxWorkspaceModule(string identity, ReadOnlyMemory<byte> sourceUtf8)
    {
        ArgumentNullException.ThrowIfNull(identity);
        if (identity.Length == 0)
        {
            throw new ArgumentException("Workspace module identity must not be empty.", nameof(identity));
        }

        Identity = identity;
        SourceUtf8 = sourceUtf8;
    }

    /// <summary>
    /// Gets the logical workspace identity.
    /// </summary>
    public string Identity { get; }

    /// <summary>
    /// Gets the UTF-8 source byte payload.
    /// </summary>
    public ReadOnlyMemory<byte> SourceUtf8 { get; }

    /// <summary>
    /// Creates a workspace module by encoding source text as UTF-8.
    /// </summary>
    public static NxWorkspaceModule FromSourceText(string identity, string source)
    {
        ArgumentNullException.ThrowIfNull(source);
        return new NxWorkspaceModule(identity, Encoding.UTF8.GetBytes(source));
    }
}
