// Copyright (c) Bret Johnson. All rights reserved.
// Licensed under the MIT license. See LICENSE file in the project root for full license information.

namespace NxLang.Nx;

/// <summary>
/// Selects the output format returned by native NX runtime calls.
/// </summary>
public enum NxOutputFormat
{
    /// <summary>
    /// Return the canonical NX MessagePack payload.
    /// </summary>
    MessagePack = 0,

    /// <summary>
    /// Return a UTF-8 JSON payload.
    /// </summary>
    Json = 1,
}
