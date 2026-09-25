// Copyright (c) Bret Johnson. All rights reserved.
// Licensed under the MIT license. See LICENSE file in the project root for full license information.

using System;

namespace NxLang.Nx;

/// <summary>
/// One field of an update record's schema: its wire name paired with the CLR type its values have, and whether
/// the field can be cleared.
/// </summary>
/// <remarks>
/// <para>A generated update record whose target has a plain generated type describes its fields with
/// <see cref="NxProperty{TRecord, TValue}"/>, which adds read and write access to the field on that type. A
/// companion with no plain type, such as a component's state, describes them with this class alone.</para>
/// <para>A field is clearable when its target declares it optional (<c>name?:T</c>): only such a field may be
/// set to <see langword="null"/>, the .NET spelling of the NX empty value. When the schema does not say, the
/// field counts as clearable unless its CLR type cannot hold <see langword="null"/> at all, and a
/// <see langword="null"/> for a non-clearable field that gets past the schema is rejected by the NX runtime when
/// the update record is constructed.</para>
/// </remarks>
public class NxField
{
    /// <summary>
    /// Initializes a new instance of the <see cref="NxField"/> class.
    /// </summary>
    /// <param name="name">The field's wire name, which is the NX field name.</param>
    /// <param name="valueType">The CLR type of the field's values, without nullable reference annotations.</param>
    /// <param name="clearable">
    /// Whether the field may be set to <see langword="null"/>, which is whether the target declares it optional.
    /// <see langword="null"/> lets the CLR type decide: a non-nullable value type cannot be cleared, and any
    /// other type can, as far as the schema knows.
    /// </param>
    public NxField(string name, Type valueType, bool? clearable = null)
    {
        Name = name ?? throw new ArgumentNullException(nameof(name));
        ValueType = valueType ?? throw new ArgumentNullException(nameof(valueType));
        Clearable = clearable ?? (!valueType.IsValueType || Nullable.GetUnderlyingType(valueType) is not null);
    }

    /// <summary>
    /// Gets the field's wire name.
    /// </summary>
    public string Name { get; }

    /// <summary>
    /// Gets the CLR type of the field's values, which is what serialization reads and writes them as.
    /// </summary>
    public Type ValueType { get; }

    /// <summary>
    /// Gets a value indicating whether the field may be cleared, that is, carried with a <see langword="null"/>
    /// value. Only a field the target declares optional (<c>name?:T</c>) can be.
    /// </summary>
    public bool Clearable { get; }

    /// <summary>
    /// Throws when <paramref name="value"/> would clear a field that cannot be cleared.
    /// </summary>
    /// <param name="value">The value about to be stored for the field.</param>
    /// <param name="nxType">The <c>$type</c> of the update record, for the message.</param>
    /// <exception cref="InvalidOperationException">Thrown when the value is <see langword="null"/> and the field
    /// is not clearable.</exception>
    internal void CheckClearable(object? value, string nxType)
    {
        if (value is null && !Clearable)
        {
            throw new InvalidOperationException(CannotClearMessage(nxType));
        }
    }

    /// <summary>
    /// Returns the message that reports an attempt to clear this field when it cannot be cleared.
    /// </summary>
    /// <param name="nxType">The <c>$type</c> of the update record.</param>
    /// <param name="recordTypeName">The name of the CLR type the update record targets, when the message should
    /// name it too, as serialization's does.</param>
    internal string CannotClearMessage(string nxType, string? recordTypeName = null)
    {
        string target = recordTypeName is null ? $"'{nxType}'" : $"{recordTypeName} ('{nxType}')";
        return $"'{Name}' cannot be cleared: it is not optional in the target of {target}.";
    }

    /// <inheritdoc />
    public override string ToString() => Name;
}
