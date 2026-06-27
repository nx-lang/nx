// Copyright (c) Bret Johnson. All rights reserved.
// Licensed under the MIT license. See LICENSE file in the project root for full license information.

using System;
using System.Text;

namespace NxLang.Nx;

/// <summary>
/// Represents one source-backed module in an in-memory NX workspace.
/// </summary>
public sealed class NxWorkspaceModule
{
    /// <summary>
    /// Creates a workspace module from a logical identity and UTF-8 source bytes.
    /// </summary>
    /// <param name="identity">
    /// Logical module identity used for imports, diagnostics, and workspace entry selection.
    /// </param>
    /// <param name="sourceUtf8">UTF-8 encoded NX source bytes for the module.</param>
    /// <exception cref="ArgumentNullException">
    /// Thrown when <paramref name="identity"/> is <see langword="null"/>.
    /// </exception>
    /// <exception cref="ArgumentException">Thrown when <paramref name="identity"/> is empty.</exception>
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
    /// Gets the logical workspace identity used by NX imports, diagnostics, and entry selection.
    /// </summary>
    public string Identity { get; }

    /// <summary>
    /// Gets the UTF-8 source byte payload.
    /// </summary>
    public ReadOnlyMemory<byte> SourceUtf8 { get; }

    /// <summary>
    /// Creates a workspace module by encoding source text as UTF-8.
    /// </summary>
    /// <param name="identity">
    /// Logical module identity used for imports, diagnostics, and workspace entry selection.
    /// </param>
    /// <param name="source">NX source text for the module.</param>
    /// <returns>A workspace module containing UTF-8 encoded source bytes.</returns>
    /// <exception cref="ArgumentNullException">
    /// Thrown when <paramref name="identity"/> or <paramref name="source"/> is <see langword="null"/>.
    /// </exception>
    /// <exception cref="ArgumentException">Thrown when <paramref name="identity"/> is empty.</exception>
    public static NxWorkspaceModule FromSourceText(string identity, string source)
    {
        ArgumentNullException.ThrowIfNull(source);
        return new NxWorkspaceModule(identity, Encoding.UTF8.GetBytes(source));
    }
}
