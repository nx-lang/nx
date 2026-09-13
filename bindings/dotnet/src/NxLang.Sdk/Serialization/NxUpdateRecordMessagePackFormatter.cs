// Copyright (c) Bret Johnson. All rights reserved.
// Licensed under the MIT license. See LICENSE file in the project root for full license information.

using System;
using MessagePack;
using MessagePack.Formatters;

namespace NxLang.Nx.Serialization;

/// <summary>
/// Serializes a generated update record DTO as a MessagePack map holding <c>$type</c> and only the fields it
/// carries.
/// </summary>
/// <remarks>
/// <para>Writing follows the DTO's <see cref="NxUpdateSchema"/>: <c>$type</c> first, then every set field in
/// ordinal key order, with a field set to <see langword="null"/> written as nil. Reading leaves a missing key
/// unset, reads a present nil as set to null, and rejects a key the schema does not declare, naming the key and
/// the DTO, as the NX runtime rejects a field a record does not declare. A <c>$type</c> that names a different
/// record is rejected the same way.</para>
/// <para>Field values go through <see cref="MessagePackSerializer"/> by the type the schema carries; the DTO's
/// own members are never reflected over.</para>
/// </remarks>
/// <typeparam name="TRecord">The generated update record DTO type.</typeparam>
[CLSCompliant(false)]
public sealed class NxUpdateRecordMessagePackFormatter<TRecord> : IMessagePackFormatter<TRecord>
    where TRecord : NxUpdateRecord, new()
{
    /// <inheritdoc />
    public void Serialize(ref MessagePackWriter writer, TRecord value, MessagePackSerializerOptions options)
    {
        if (value is null)
        {
            writer.WriteNil();
            return;
        }

        int count = 1;
        foreach (NxField field in value.Schema.FieldsInWireOrder)
        {
            if (value.Fields.ContainsKey(field.Name))
            {
                count++;
            }
        }

        writer.WriteMapHeader(count);
        writer.Write("$type");
        writer.Write(value.Schema.NxType);
        foreach (NxField field in value.Schema.FieldsInWireOrder)
        {
            if (!value.Fields.TryGetValue(field.Name, out object? fieldValue))
            {
                continue;
            }

            writer.Write(field.Name);
            MessagePackSerializer.Serialize(field.ValueType, ref writer, fieldValue, options);
        }
    }

    /// <inheritdoc />
    public TRecord Deserialize(ref MessagePackReader reader, MessagePackSerializerOptions options)
    {
        if (reader.TryReadNil())
        {
            return null!;
        }

        options.Security.DepthStep(ref reader);
        try
        {
            TRecord record = new();
            NxUpdateSchema schema = record.Schema;
            int count = reader.ReadMapHeader();
            for (int i = 0; i < count; i++)
            {
                if (reader.NextMessagePackType != MessagePackType.String)
                {
                    throw new MessagePackSerializationException(
                        $"Expected a string key in the map for {typeof(TRecord).Name}.");
                }

                string key = reader.ReadString()!;
                if (key == "$type")
                {
                    string? type = reader.NextMessagePackType == MessagePackType.String
                        ? reader.ReadString()
                        : null;
                    if (type != schema.NxType)
                    {
                        if (type is null)
                        {
                            reader.Skip();
                        }

                        throw new MessagePackSerializationException(
                            $"Expected '$type' to be '{schema.NxType}' for {typeof(TRecord).Name} but read '{type}'.");
                    }

                    continue;
                }

                if (!schema.TryGetField(key, out NxField? field))
                {
                    throw new MessagePackSerializationException(
                        $"'{key}' is not a field of {typeof(TRecord).Name} ('{schema.NxType}').");
                }

                record.SetFieldValue(key, MessagePackSerializer.Deserialize(field.ValueType, ref reader, options));
            }

            return record;
        }
        finally
        {
            reader.Depth--;
        }
    }
}
