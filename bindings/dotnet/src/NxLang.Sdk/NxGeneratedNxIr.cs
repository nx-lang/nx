// Copyright (c) Bret Johnson. All rights reserved.
// Licensed under the MIT license. See LICENSE file in the project root for full license information.

namespace NxLang.Nx;

/// <summary>
/// One NX IR artifact generated from an NX program artifact: its image and its metadata.
/// </summary>
public sealed class NxGeneratedNxIr
{
    /// <summary>
    /// Gets the workspace identity of the module the artifact carries.
    /// </summary>
    public string Identity { get; init; } = string.Empty;

    /// <summary>
    /// Gets the artifact as an NX IR image, byte for byte what the Node and wasm SDKs and the CLI emit
    /// for the same input.
    /// </summary>
    public byte[] Bytes { get; init; } = Array.Empty<byte>();

    /// <summary>
    /// Gets structured metadata for the generated artifact.
    /// </summary>
    public NxIrMetadata Metadata { get; init; } = new();
}
