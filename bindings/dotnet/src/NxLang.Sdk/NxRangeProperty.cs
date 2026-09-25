// Copyright (c) Bret Johnson. All rights reserved.
// Licensed under the MIT license. See LICENSE file in the project root for full license information.

using System;
using System.Text.Json.Serialization;
using MessagePack;
using NxLang.Nx.Serialization;

namespace NxLang.Nx;

/// <summary>
/// One field of a <see cref="NxRange{T}"/> by name: the NX prelude's derived <c>Range.Property</c> union.
/// </summary>
/// <remarks>
/// Hand-written here rather than generated into a host's namespace, for the reason <see cref="NxRange{T}"/> gives, and
/// named the way typegen names a prelude companion: the <c>Nx</c> prefix over <c>&lt;Target&gt;_property</c>.
/// </remarks>
[JsonConverter(typeof(NxEnumJsonConverter<NxRange_property, NxRange_propertyWireFormat>))]
[MessagePackFormatter(typeof(NxEnumMessagePackFormatter<NxRange_property, NxRange_propertyWireFormat>))]
[System.Diagnostics.CodeAnalysis.SuppressMessage(
    "StyleCop.CSharp.NamingRules",
    "SA1300:Element should begin with upper-case letter",
    Justification = "The name is the one typegen renders for a prelude companion.")]
public enum NxRange_property
{
    /// <summary>Where the range starts.</summary>
    Start,

    /// <summary>Where the range stops.</summary>
    End,

    /// <summary>Whether the end is part of the range.</summary>
    EndInclusive,
}

/// <summary>
/// The wire names of <see cref="NxRange_property"/>, which are <see cref="NxRange{T}"/>'s NX field names.
/// </summary>
[System.Diagnostics.CodeAnalysis.SuppressMessage(
    "StyleCop.CSharp.NamingRules",
    "SA1300:Element should begin with upper-case letter",
    Justification = "The name is the one typegen renders for a prelude companion.")]
internal sealed class NxRange_propertyWireFormat : INxEnumWireFormat<NxRange_property>
{
    /// <summary>
    /// Returns the wire name of <paramref name="value"/>.
    /// </summary>
    /// <param name="value">The member to format.</param>
    /// <returns>The NX field name.</returns>
    public static string Format(NxRange_property value) =>
        value switch
        {
            NxRange_property.Start => "start",
            NxRange_property.End => "end",
            NxRange_property.EndInclusive => "endInclusive",
            _ => throw new FormatException("Unknown NX enum value."),
        };

    /// <summary>
    /// Returns the member <paramref name="value"/> names.
    /// </summary>
    /// <param name="value">The NX field name.</param>
    /// <returns>The member it names.</returns>
    public static NxRange_property Parse(string value) =>
        value switch
        {
            "start" => NxRange_property.Start,
            "end" => NxRange_property.End,
            "endInclusive" => NxRange_property.EndInclusive,
            _ => throw new FormatException("Unknown NX enum member."),
        };
}

/// <summary>
/// The typed field keys of <see cref="NxRange{T}"/>, which <see cref="NxRange_update{T}"/>'s schema is built from.
/// None of the prelude's range fields is optional, so none can be cleared.
/// </summary>
/// <typeparam name="T">The type of the bounds, as on <see cref="NxRange{T}"/>.</typeparam>
public static class NxRangeProperties<T>
{
    /// <summary>The key of <see cref="NxRange{T}.Start"/>.</summary>
    public static readonly NxProperty<NxRange<T>, T> Start = new(
        NxRange_propertyWireFormat.Format(NxRange_property.Start),
        record => record.Start,
        (record, value) => record.Start = value,
        clearable: false);

    /// <summary>The key of <see cref="NxRange{T}.End"/>.</summary>
    public static readonly NxProperty<NxRange<T>, T> End = new(
        NxRange_propertyWireFormat.Format(NxRange_property.End),
        record => record.End,
        (record, value) => record.End = value,
        clearable: false);

    /// <summary>The key of <see cref="NxRange{T}.EndInclusive"/>.</summary>
    public static readonly NxProperty<NxRange<T>, bool> EndInclusive = new(
        NxRange_propertyWireFormat.Format(NxRange_property.EndInclusive),
        record => record.EndInclusive,
        (record, value) => record.EndInclusive = value,
        clearable: false);

    /// <summary>
    /// Returns the key <paramref name="property"/> names.
    /// </summary>
    /// <param name="property">The field to look up.</param>
    /// <returns>The typed key of that field.</returns>
    public static NxProperty<NxRange<T>> Of(NxRange_property property)
    {
        switch (property)
        {
            case NxRange_property.Start:
                return Start;
            case NxRange_property.End:
                return End;
            case NxRange_property.EndInclusive:
                return EndInclusive;
            default:
                throw new ArgumentOutOfRangeException(nameof(property));
        }
    }
}
