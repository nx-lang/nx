// Copyright (c) Bret Johnson. All rights reserved.
// Licensed under the MIT license. See LICENSE file in the project root for full license information.

using System;

namespace NxLang.Nx;

/// <summary>
/// A typed key naming one field of a generated record, with untyped access to that field on an instance.
/// </summary>
/// <remarks>
/// Typegen emits one key per field in a <c>&lt;T&gt;Properties</c> class beside each generated record, and an
/// <c>Of(&lt;T&gt;_property)</c> method that maps each case of the generated property enum to its key. The key's
/// name comes from the same wire format the enum uses, so the enum stays the single naming of a record's fields.
/// </remarks>
/// <typeparam name="TRecord">The generated record type that declares the field.</typeparam>
public abstract class NxProperty<TRecord> : NxField
{
    /// <summary>
    /// Initializes a new instance of the <see cref="NxProperty{TRecord}"/> class.
    /// </summary>
    /// <param name="name">The field's wire name.</param>
    /// <param name="valueType">The CLR type of the field's values.</param>
    /// <param name="clearable">Whether the field may be cleared; see <see cref="NxField"/>.</param>
    protected NxProperty(string name, Type valueType, bool? clearable = null)
        : base(name, valueType, clearable)
    {
    }

    /// <summary>
    /// Reads the field from <paramref name="record"/> as an untyped value.
    /// </summary>
    /// <param name="record">The record to read.</param>
    /// <returns>The field's value, which may be <see langword="null"/>.</returns>
    public abstract object? GetValue(TRecord record);

    /// <summary>
    /// Writes an untyped value to the field of <paramref name="record"/>.
    /// </summary>
    /// <param name="record">The record to write.</param>
    /// <param name="value">The value, which must be of the field's value type or <see langword="null"/>.</param>
    public abstract void SetValue(TRecord record, object? value);
}

/// <summary>
/// A typed key naming one field of a generated record, with typed access to that field on an instance.
/// </summary>
/// <typeparam name="TRecord">The generated record type that declares the field.</typeparam>
/// <typeparam name="TValue">The field's value type.</typeparam>
public sealed class NxProperty<TRecord, TValue> : NxProperty<TRecord>
{
    private readonly Func<TRecord, TValue> _get;
    private readonly Action<TRecord, TValue> _set;

    /// <summary>
    /// Initializes a new instance of the <see cref="NxProperty{TRecord, TValue}"/> class.
    /// </summary>
    /// <param name="name">The field's wire name.</param>
    /// <param name="get">Reads the field from a record.</param>
    /// <param name="set">Writes the field of a record.</param>
    /// <param name="clearable">Whether the field may be cleared; see <see cref="NxField"/>.</param>
    public NxProperty(
        string name,
        Func<TRecord, TValue> get,
        Action<TRecord, TValue> set,
        bool? clearable = null)
        : base(name, typeof(TValue), clearable)
    {
        _get = get ?? throw new ArgumentNullException(nameof(get));
        _set = set ?? throw new ArgumentNullException(nameof(set));
    }

    /// <summary>
    /// Reads the field from <paramref name="record"/>.
    /// </summary>
    /// <param name="record">The record to read.</param>
    /// <returns>The field's value.</returns>
    public TValue Get(TRecord record) => _get(record);

    /// <summary>
    /// Writes <paramref name="value"/> to the field of <paramref name="record"/>.
    /// </summary>
    /// <param name="record">The record to write.</param>
    /// <param name="value">The new value.</param>
    public void Set(TRecord record, TValue value) => _set(record, value);

    /// <inheritdoc />
    public override object? GetValue(TRecord record) => _get(record);

    /// <inheritdoc />
    public override void SetValue(TRecord record, object? value) => _set(record, (TValue)value!);
}
