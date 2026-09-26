// Copyright (c) The NX Authors.
// Licensed under the MIT license. See LICENSE file in the project root for full license information.

using System;
using System.Collections.Generic;

namespace NxLang.Nx;

/// <summary>
/// The base of every generated update record DTO: a patch of one record, stored as a map from wire name to value.
/// </summary>
/// <remarks>
/// <para>A field the map carries is set and means "set this field to the value"; a field it does not carry is
/// unset and means "leave this field unchanged". A set field whose value is <see langword="null"/> is cleared:
/// <see langword="null"/> is the .NET spelling of the NX empty value, and clearing is legal only for a field the
/// target declares optional (<c>name?:T</c>), which the schema records as <see cref="NxField.Clearable"/>. The
/// generated class exposes each field as an <see cref="NxOptional{T}"/> property over <see cref="Get{T}"/> and
/// <see cref="Set{T}"/>, whose value type is nullable only for a clearable field, and this class exposes the map
/// itself through <see cref="Fields"/>, <see cref="IsSet"/>, and <see cref="Unset"/>.</para>
/// <para>Serialization is driven by <see cref="Schema"/> in both formats: <c>$type</c> first, then the set fields
/// in ordinal key order, with a cleared field written as <see langword="null"/>. A key the schema does not
/// declare is rejected on read, and so is a <see langword="null"/> for a field the schema knows cannot be
/// cleared.</para>
/// </remarks>
public abstract class NxUpdateRecord
{
    private readonly Dictionary<string, object?> _fields = new(StringComparer.Ordinal);

    /// <summary>
    /// Initializes a new instance of the <see cref="NxUpdateRecord"/> class with no field set.
    /// </summary>
    /// <param name="schema">The generated schema of the concrete DTO.</param>
    protected NxUpdateRecord(NxUpdateSchema schema)
    {
        Schema = schema ?? throw new ArgumentNullException(nameof(schema));
    }

    /// <summary>
    /// Gets the schema: the <c>$type</c> discriminator and every field the DTO can carry.
    /// </summary>
    public NxUpdateSchema Schema { get; }

    /// <summary>
    /// Gets the fields the patch carries, by wire name. A cleared field is present with a <see langword="null"/>
    /// value; an unset field is absent.
    /// </summary>
    public IReadOnlyDictionary<string, object?> Fields => _fields;

    /// <summary>
    /// Returns whether the patch carries the field named <paramref name="name"/>.
    /// </summary>
    /// <param name="name">The field's wire name.</param>
    public bool IsSet(string name) => _fields.ContainsKey(name);

    /// <summary>
    /// Removes the field named <paramref name="name"/> from the patch, so it reads as unset and is omitted from
    /// the wire. Unsetting a field that is not set does nothing.
    /// </summary>
    /// <param name="name">The field's wire name.</param>
    public void Unset(string name) => _fields.Remove(name);

    /// <summary>
    /// Returns the wire names of the fields the patch carries, in the schema's declared order.
    /// </summary>
    /// <remarks>
    /// This is the untyped form of the NX <c>changed</c> intrinsic. A generated DTO narrows it to its
    /// <c>&lt;T&gt;_property</c> enum through <c>Changed()</c>.
    /// </remarks>
    public string[] ChangedNames()
    {
        List<string> names = new(_fields.Count);
        foreach (NxField field in Schema.Fields)
        {
            if (_fields.ContainsKey(field.Name))
            {
                names.Add(field.Name);
            }
        }

        return names.ToArray();
    }

    /// <summary>
    /// Returns a new patch carrying every field either patch carries, where <paramref name="second"/> wins a field
    /// both carry. Neither input is modified.
    /// </summary>
    /// <remarks>
    /// This is the NX <c>merge</c> intrinsic. It is available on every update DTO, including a component's or an
    /// action's, which have no plain record type.
    /// </remarks>
    /// <typeparam name="TUpdate">The generated update DTO type.</typeparam>
    /// <param name="first">The patch applied first.</param>
    /// <param name="second">The patch applied second, whose fields win.</param>
    public static TUpdate Merge<TUpdate>(TUpdate first, TUpdate second)
        where TUpdate : NxUpdateRecord, new()
    {
        ArgumentNullException.ThrowIfNull(first);
        ArgumentNullException.ThrowIfNull(second);

        TUpdate merged = new();
        foreach (KeyValuePair<string, object?> field in first._fields)
        {
            merged._fields[field.Key] = field.Value;
        }

        foreach (KeyValuePair<string, object?> field in second._fields)
        {
            merged._fields[field.Key] = field.Value;
        }

        return merged;
    }

    /// <summary>
    /// Reads the field named <paramref name="name"/>: unset when the patch does not carry it, otherwise set to its
    /// value.
    /// </summary>
    /// <typeparam name="T">The field's value type.</typeparam>
    /// <param name="name">The field's wire name.</param>
    protected NxOptional<T> Get<T>(string name) =>
        _fields.TryGetValue(name, out object? value) ? new NxOptional<T>((T)value!) : default;

    /// <summary>
    /// Writes the field named <paramref name="name"/>: a set value is stored, where <see langword="null"/> clears
    /// the field, and an unset value removes the field.
    /// </summary>
    /// <typeparam name="T">The field's value type.</typeparam>
    /// <param name="name">The field's wire name.</param>
    /// <param name="value">The value to store, or unset to remove the field.</param>
    /// <exception cref="ArgumentException">Thrown when the schema does not declare the field.</exception>
    /// <exception cref="InvalidOperationException">Thrown when the value is <see langword="null"/> and the schema
    /// knows the field cannot be cleared.</exception>
    protected void Set<T>(string name, NxOptional<T> value)
    {
        if (!Schema.TryGetField(name, out NxField? field))
        {
            throw new ArgumentException($"'{name}' is not a field of '{Schema.NxType}'.", nameof(name));
        }

        if (value.HasValue)
        {
            field.CheckClearable(value.Value, Schema.NxType);
            _fields[name] = value.Value;
        }
        else
        {
            _fields.Remove(name);
        }
    }

    /// <summary>
    /// Stores an untyped value read from the wire or computed by a helper. The caller has already checked the
    /// name against the schema and, for a <see langword="null"/>, that the field is clearable.
    /// </summary>
    internal void SetFieldValue(string name, object? value) => _fields[name] = value;
}
