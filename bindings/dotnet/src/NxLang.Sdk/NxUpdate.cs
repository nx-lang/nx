// Copyright (c) The NX Authors.
// Licensed under the MIT license. See LICENSE file in the project root for full license information.

using System;
using System.Collections.Generic;

namespace NxLang.Nx;

/// <summary>
/// The base of a generated update record DTO whose target has a plain generated type: adds typed access by
/// <see cref="NxProperty{TRecord, TValue}"/> key, and the <see cref="Apply"/> and <see cref="Diff{TUpdate}"/>
/// helpers that touch a record.
/// </summary>
/// <remarks>
/// Every field of the schema a derived class passes up must be an <see cref="NxProperty{TRecord}"/>, since the
/// helpers read and write the record through the keys. Typegen builds the schema from the generated
/// <c>&lt;T&gt;Properties</c> table, which guarantees that.
/// </remarks>
/// <typeparam name="TRecord">The generated record type the patch applies to.</typeparam>
public abstract class NxUpdate<TRecord> : NxUpdateRecord
    where TRecord : class, new()
{
    /// <summary>
    /// Initializes a new instance of the <see cref="NxUpdate{TRecord}"/> class with no field set.
    /// </summary>
    /// <param name="schema">The generated schema, whose fields are the record's property keys.</param>
    protected NxUpdate(NxUpdateSchema schema)
        : base(schema)
    {
    }

    /// <summary>
    /// Reads the field <paramref name="property"/> names, with the key's value type.
    /// </summary>
    /// <typeparam name="TValue">The field's value type.</typeparam>
    /// <param name="property">The field's key.</param>
    public NxOptional<TValue> Get<TValue>(NxProperty<TRecord, TValue> property)
    {
        ArgumentNullException.ThrowIfNull(property);
        return Get<TValue>(property.Name);
    }

    /// <summary>
    /// Writes the field <paramref name="property"/> names, with the key's value type. A <see langword="null"/>
    /// clears the field, which only a clearable field allows; an unset value removes the field.
    /// </summary>
    /// <typeparam name="TValue">The field's value type.</typeparam>
    /// <param name="property">The field's key.</param>
    /// <param name="value">The value to store, or unset to remove the field.</param>
    public void Set<TValue>(NxProperty<TRecord, TValue> property, NxOptional<TValue> value)
    {
        ArgumentNullException.ThrowIfNull(property);
        Set(property.Name, value);
    }

    /// <summary>
    /// Returns whether the patch carries the field <paramref name="property"/> names.
    /// </summary>
    /// <param name="property">The field's key.</param>
    public bool IsSet(NxProperty<TRecord> property)
    {
        ArgumentNullException.ThrowIfNull(property);
        return IsSet(property.Name);
    }

    /// <summary>
    /// Removes the field <paramref name="property"/> names from the patch.
    /// </summary>
    /// <param name="property">The field's key.</param>
    public void Unset(NxProperty<TRecord> property)
    {
        ArgumentNullException.ThrowIfNull(property);
        Unset(property.Name);
    }

    /// <summary>
    /// Returns a new record equal to <paramref name="record"/> except for the fields the patch carries, where a
    /// carried <see langword="null"/> clears the field: the record's property becomes <see langword="null"/>, the
    /// .NET reading of the empty value, which serializes as an omitted key. <paramref name="record"/> is not
    /// modified.
    /// </summary>
    /// <remarks>
    /// This is the NX <c>apply</c> intrinsic. The copy is shallow, as it is in the TypeScript runtime.
    /// </remarks>
    /// <param name="record">The record to patch.</param>
    public TRecord Apply(TRecord record)
    {
        ArgumentNullException.ThrowIfNull(record);

        TRecord result = new();
        foreach (NxProperty<TRecord> property in Properties())
        {
            property.SetValue(
                result,
                Fields.TryGetValue(property.Name, out object? value) ? value : property.GetValue(record));
        }

        return result;
    }

    /// <summary>
    /// Returns a patch carrying exactly the fields whose values differ between <paramref name="before"/> and
    /// <paramref name="after"/>, each set to its value in <paramref name="after"/>. A field that is empty in
    /// <paramref name="after"/> and not in <paramref name="before"/> is carried cleared, as
    /// <see langword="null"/>, whether <paramref name="after"/> holds <see langword="null"/> or an empty array.
    /// </summary>
    /// <remarks>
    /// This is the NX <c>diff</c> intrinsic, with the comparison the TypeScript runtime's <c>nxValuesEqual</c>
    /// makes: an empty field reads as the empty value whether it holds <see langword="null"/> or an empty array,
    /// arrays compare element-wise and nested records structurally. A generated DTO wraps it as a non-generic
    /// <c>Diff(before, after)</c>.
    /// </remarks>
    /// <typeparam name="TUpdate">The generated update DTO type to produce.</typeparam>
    /// <param name="before">The earlier record.</param>
    /// <param name="after">The later record.</param>
    /// <exception cref="InvalidOperationException">Thrown when a field that cannot be cleared is empty in
    /// <paramref name="after"/> and not in <paramref name="before"/>, which a valid record never is.</exception>
    public static TUpdate Diff<TUpdate>(TRecord before, TRecord after)
        where TUpdate : NxUpdate<TRecord>, new()
    {
        ArgumentNullException.ThrowIfNull(before);
        ArgumentNullException.ThrowIfNull(after);

        TUpdate diff = new();
        foreach (NxProperty<TRecord> property in diff.Properties())
        {
            object? next = property.GetValue(after);
            if (!NxValueEquality.ValuesEqual(property.GetValue(before), next))
            {
                // An emptied field is carried cleared, as the TypeScript runtime's nxCleared carries it.
                object? carried = NxValueEquality.IsEmpty(next) ? null : next;
                property.CheckClearable(carried, diff.Schema.NxType);
                diff.SetFieldValue(property.Name, carried);
            }
        }

        return diff;
    }

    private IEnumerable<NxProperty<TRecord>> Properties()
    {
        foreach (NxField field in Schema.Fields)
        {
            yield return field as NxProperty<TRecord>
                ?? throw new InvalidOperationException(
                    $"The schema of '{Schema.NxType}' describes '{field.Name}' without a property key over "
                    + $"{typeof(TRecord).Name}.");
        }
    }
}
