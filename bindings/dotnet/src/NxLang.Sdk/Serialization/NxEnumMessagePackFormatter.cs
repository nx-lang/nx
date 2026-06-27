// Copyright (c) Bret Johnson. All rights reserved.
// Licensed under the MIT license. See LICENSE file in the project root for full license information.

using System;
using MessagePack;
using MessagePack.Formatters;

namespace NxLang.Nx.Serialization;

/// <summary>
/// Serializes an NX enum as its authored member string.
/// </summary>
/// <typeparam name="TEnum">The CLR enum type.</typeparam>
/// <typeparam name="TWire">The explicit wire-format mapping type.</typeparam>
[CLSCompliant(false)]
public sealed class NxEnumMessagePackFormatter<TEnum, TWire> : IMessagePackFormatter<TEnum>
    where TEnum : struct, Enum
    where TWire : INxEnumWireFormat<TEnum>
{
    /// <inheritdoc />
    public void Serialize(ref MessagePackWriter writer, TEnum value, MessagePackSerializerOptions options)
    {
        writer.Write(TWire.Format(value));
    }

    /// <inheritdoc />
    public TEnum Deserialize(ref MessagePackReader reader, MessagePackSerializerOptions options)
    {
        if (reader.TryReadNil() || reader.NextMessagePackType is not MessagePackType.String)
        {
            throw new MessagePackSerializationException("Expected NX enum to be encoded as a MessagePack string.");
        }

        string value = reader.ReadString()!;

        try
        {
            return TWire.Parse(value);
        }
        catch (FormatException e)
        {
            throw new MessagePackSerializationException(e.Message, e);
        }
    }
}
