// Copyright (c) The NX Authors.
// Licensed under the MIT license. See LICENSE file in the project root for full license information.

using System;
using System.Collections.Generic;
using System.Diagnostics.CodeAnalysis;

namespace NxLang.Nx;

/// <summary>
/// The schema of a generated update record: its <c>$type</c> discriminator and its fields in declared order.
/// </summary>
/// <remarks>
/// Typegen emits one schema per update companion, and it is everything serialization needs: the discriminator
/// to write first, each field's wire name and value type, and the ordinal key order the wire uses. The declared
/// order is kept separately for <see cref="NxUpdateRecord.ChangedNames"/>.
/// </remarks>
public sealed class NxUpdateSchema
{
    private readonly Dictionary<string, NxField> _fieldsByName;

    /// <summary>
    /// Initializes a new instance of the <see cref="NxUpdateSchema"/> class.
    /// </summary>
    /// <param name="nxType">The <c>$type</c> discriminator, <c>&lt;Target&gt;.Update</c>.</param>
    /// <param name="fields">The fields in declared order.</param>
    /// <exception cref="ArgumentException">Thrown when two fields share a name.</exception>
    public NxUpdateSchema(string nxType, params NxField[] fields)
    {
        NxType = nxType ?? throw new ArgumentNullException(nameof(nxType));
        ArgumentNullException.ThrowIfNull(fields);

        _fieldsByName = new Dictionary<string, NxField>(fields.Length, StringComparer.Ordinal);
        foreach (NxField field in fields)
        {
            if (!_fieldsByName.TryAdd(field.Name, field))
            {
                throw new ArgumentException($"'{nxType}' declares the field '{field.Name}' twice.", nameof(fields));
            }
        }

        NxField[] wireOrder = (NxField[])fields.Clone();
        Array.Sort(wireOrder, static (left, right) => string.CompareOrdinal(left.Name, right.Name));
        Fields = fields;
        FieldsInWireOrder = wireOrder;
    }

    /// <summary>
    /// Gets the <c>$type</c> discriminator written first in every serialized patch.
    /// </summary>
    public string NxType { get; }

    /// <summary>
    /// Gets the fields in declared order.
    /// </summary>
    public IReadOnlyList<NxField> Fields { get; }

    /// <summary>
    /// Gets the fields in the order their keys take on the wire, which is ordinal by name.
    /// </summary>
    public IReadOnlyList<NxField> FieldsInWireOrder { get; }

    /// <summary>
    /// Looks up a field by wire name.
    /// </summary>
    /// <param name="name">The wire name.</param>
    /// <param name="field">The field, when the schema declares one by that name.</param>
    /// <returns><see langword="true"/> when the schema declares the field.</returns>
    public bool TryGetField(string name, [MaybeNullWhen(false)] out NxField field) =>
        _fieldsByName.TryGetValue(name, out field);
}
