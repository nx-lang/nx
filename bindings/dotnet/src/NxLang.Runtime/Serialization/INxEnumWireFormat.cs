// Copyright (c) Bret Johnson. All rights reserved.
// Licensed under the MIT license. See LICENSE file in the project root for full license information.

using System;

namespace NxLang.Nx.Serialization;

/// <summary>
/// Defines the explicit authored wire-name mapping for an NX enum.
/// </summary>
/// <typeparam name="TEnum">The CLR enum type.</typeparam>
public interface INxEnumWireFormat<TEnum>
    where TEnum : struct, Enum
{
    /// <summary>
    /// Converts the CLR enum value to its authored NX wire representation.
    /// </summary>
    /// <param name="value">The enum value to encode.</param>
    /// <returns>The authored NX member string.</returns>
    static abstract string Format(TEnum value);

    /// <summary>
    /// Parses an authored NX wire representation into the CLR enum value.
    /// </summary>
    /// <param name="value">The authored NX member string.</param>
    /// <returns>The CLR enum value.</returns>
    static abstract TEnum Parse(string value);
}
