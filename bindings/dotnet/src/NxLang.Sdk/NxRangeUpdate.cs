// Copyright (c) Bret Johnson. All rights reserved.
// Licensed under the MIT license. See LICENSE file in the project root for full license information.

// MsgPack006 reads the open generic MessagePack documents for a generic type's formatter as a type
// that implements no formatter interface, because an unbound generic implements none until it is
// closed. Generated C# disables it for the same reason.
#pragma warning disable MsgPack006

using System;
using System.Text.Json.Serialization;
using MessagePack;
using MessagePack.Formatters;
using NxLang.Nx.Serialization;

namespace NxLang.Nx;

/// <summary>
/// A patch of a <see cref="NxRange{T}"/>: the NX prelude's derived <c>Range.Update</c> companion, which <c>apply</c>
/// and <c>diff</c> carry.
/// </summary>
/// <remarks>
/// <para>
/// The prelude's declarations are hand-written here rather than generated into a host's namespace, for the reason
/// <see cref="NxRange{T}"/> gives: two generated libraries sharing one namespace would each declare a copy. The name
/// keeps the <c>Nx</c> prefix the emitter puts on every prelude type, over the <c>&lt;Target&gt;_update</c> spelling it
/// gives every companion, so a field typed <c>&lt;Range.Update T=int/&gt;</c> renders as
/// <c>NxLang.Nx.NxRange_update&lt;long&gt;</c>.
/// </para>
/// <para>
/// The wire shape is exactly what the C# emitter generates for a user-declared record with <see cref="NxRange{T}"/>'s
/// fields. <c>NxRangeCompanionTests</c> holds it to that, against a generated contract whose fields are typed by the
/// prelude's companions and against what NX's own <c>diff</c> produces.
/// </para>
/// </remarks>
/// <typeparam name="T">The type of the bounds, as on <see cref="NxRange{T}"/>.</typeparam>
[JsonConverter(typeof(NxUpdateRecordJsonConverterFactory))]
[MessagePackFormatter(typeof(NxRange_updateFormatter<>))]
[System.Diagnostics.CodeAnalysis.SuppressMessage(
    "StyleCop.CSharp.NamingRules",
    "SA1300:Element should begin with upper-case letter",
    Justification = "The name is the one typegen renders for a prelude companion.")]
public sealed class NxRange_update<T> : NxUpdate<NxRange<T>>
{
    private static readonly NxUpdateSchema FieldSchema = new(
        "Range.Update",
        NxRangeProperties<T>.Start,
        NxRangeProperties<T>.End,
        NxRangeProperties<T>.EndInclusive);

    /// <summary>
    /// Initializes a new instance of the <see cref="NxRange_update{T}"/> class with no field set.
    /// </summary>
    public NxRange_update()
        : base(FieldSchema)
    {
    }

    /// <summary>
    /// Gets the <c>$type</c> discriminator this patch carries on the wire.
    /// </summary>
    public string NxType => "Range.Update";

    /// <summary>
    /// Gets or sets the patch of <see cref="NxRange{T}.Start"/>.
    /// </summary>
    public NxOptional<T> Start
    {
        get => base.Get<T>("start");
        set => base.Set("start", value);
    }

    /// <summary>
    /// Gets or sets the patch of <see cref="NxRange{T}.End"/>.
    /// </summary>
    public NxOptional<T> End
    {
        get => base.Get<T>("end");
        set => base.Set("end", value);
    }

    /// <summary>
    /// Gets or sets the patch of <see cref="NxRange{T}.EndInclusive"/>.
    /// </summary>
    public NxOptional<bool> EndInclusive
    {
        get => base.Get<bool>("endInclusive");
        set => base.Set("endInclusive", value);
    }

    /// <summary>
    /// Returns the patch from <paramref name="before"/> to <paramref name="after"/>, as NX's <c>diff</c> does.
    /// </summary>
    /// <param name="before">The range the patch applies to.</param>
    /// <param name="after">The range the patch produces.</param>
    /// <returns>A patch carrying every field the two differ in, and no other.</returns>
    public static NxRange_update<T> Diff(NxRange<T> before, NxRange<T> after) =>
        NxUpdate<NxRange<T>>.Diff<NxRange_update<T>>(before, after);

    /// <summary>
    /// Returns whether the patch carries <paramref name="property"/>.
    /// </summary>
    /// <param name="property">The field to test.</param>
    /// <returns><see langword="true"/> when the field is set.</returns>
    public bool IsSet(NxRange_property property) => base.IsSet(NxRange_propertyWireFormat.Format(property));

    /// <summary>
    /// Removes <paramref name="property"/> from the patch, so it reads as unchanged.
    /// </summary>
    /// <param name="property">The field to unset.</param>
    public void Unset(NxRange_property property) => base.Unset(NxRange_propertyWireFormat.Format(property));

    /// <summary>
    /// Returns the fields the patch carries, in the schema's declared order.
    /// </summary>
    /// <returns>One entry per set field.</returns>
    public NxRange_property[] Changed() => Array.ConvertAll(base.ChangedNames(), NxRange_propertyWireFormat.Parse);
}

/// <summary>
/// The MessagePack formatter of <see cref="NxRange_update{T}"/>, which a generic companion declares of its own because
/// the attribute takes an open generic type.
/// </summary>
/// <typeparam name="T">The type of the bounds, as on <see cref="NxRange{T}"/>.</typeparam>
[CLSCompliant(false)]
[System.Diagnostics.CodeAnalysis.SuppressMessage(
    "StyleCop.CSharp.NamingRules",
    "SA1300:Element should begin with upper-case letter",
    Justification = "The name is the one typegen renders for a prelude companion.")]
public sealed class NxRange_updateFormatter<T> : IMessagePackFormatter<NxRange_update<T>?>
{
    private static readonly NxUpdateRecordMessagePackFormatter<NxRange_update<T>> Inner = new();

    /// <inheritdoc/>
    public void Serialize(ref MessagePackWriter writer, NxRange_update<T>? value, MessagePackSerializerOptions options) =>
        Inner.Serialize(ref writer, value!, options);

    /// <inheritdoc/>
    public NxRange_update<T>? Deserialize(ref MessagePackReader reader, MessagePackSerializerOptions options) =>
        Inner.Deserialize(ref reader, options);
}
