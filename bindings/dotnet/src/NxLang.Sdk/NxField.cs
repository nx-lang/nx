// Copyright (c) Bret Johnson. All rights reserved.
// Licensed under the MIT license. See LICENSE file in the project root for full license information.

using System;

namespace NxLang.Nx;

/// <summary>
/// One field of an update record's schema: its wire name paired with the CLR type its values have.
/// </summary>
/// <remarks>
/// A generated update record whose target has a plain generated type describes its fields with
/// <see cref="NxProperty{TRecord, TValue}"/>, which adds read and write access to the field on that type. A
/// companion with no plain type, such as a component's state, describes them with this class alone.
/// </remarks>
public class NxField
{
    /// <summary>
    /// Initializes a new instance of the <see cref="NxField"/> class.
    /// </summary>
    /// <param name="name">The field's wire name, which is the NX field name.</param>
    /// <param name="valueType">The CLR type of the field's values, without nullable reference annotations.</param>
    public NxField(string name, Type valueType)
    {
        Name = name ?? throw new ArgumentNullException(nameof(name));
        ValueType = valueType ?? throw new ArgumentNullException(nameof(valueType));
    }

    /// <summary>
    /// Gets the field's wire name.
    /// </summary>
    public string Name { get; }

    /// <summary>
    /// Gets the CLR type of the field's values, which is what serialization reads and writes them as.
    /// </summary>
    public Type ValueType { get; }

    /// <inheritdoc />
    public override string ToString() => Name;
}
