// Copyright (c) Bret Johnson. All rights reserved.
// Licensed under the MIT license. See LICENSE file in the project root for full license information.

using System;
using System.Text.Json;
using System.Text.Json.Serialization;

namespace NxLang.Nx.Serialization;

/// <summary>
/// Serializes a generated update record DTO as a JSON object holding <c>$type</c> and only the fields it carries.
/// </summary>
/// <remarks>
/// <para>Writing follows the DTO's <see cref="NxUpdateSchema"/>: <c>$type</c> first, then every set field in
/// ordinal key order, with a cleared field written as <see langword="null"/>. Reading leaves a missing key unset,
/// reads a present <see langword="null"/> as cleared, and rejects a key the schema does not declare, naming the
/// key and the DTO, as the NX runtime rejects a field a record does not declare. A <see langword="null"/> for a
/// field the schema knows cannot be cleared, and a <c>$type</c> that names a different record, are rejected the
/// same way.</para>
/// <para>Field values go through <see cref="JsonSerializer"/> by the type the schema carries; the DTO's own
/// members are never reflected over.</para>
/// </remarks>
/// <typeparam name="TRecord">The generated update record DTO type.</typeparam>
public sealed class NxUpdateRecordJsonConverter<TRecord> : JsonConverter<TRecord>
    where TRecord : NxUpdateRecord, new()
{
    /// <inheritdoc />
    public override TRecord Read(ref Utf8JsonReader reader, Type typeToConvert, JsonSerializerOptions options)
    {
        if (reader.TokenType != JsonTokenType.StartObject)
        {
            throw new JsonException($"Expected a JSON object for {typeof(TRecord).Name}.");
        }

        TRecord record = new();
        NxUpdateSchema schema = record.Schema;
        while (reader.Read())
        {
            if (reader.TokenType == JsonTokenType.EndObject)
            {
                return record;
            }

            if (reader.TokenType != JsonTokenType.PropertyName)
            {
                throw new JsonException($"Expected a property name in {typeof(TRecord).Name}.");
            }

            string key = reader.GetString()!;
            if (!reader.Read())
            {
                break;
            }

            if (key == "$type")
            {
                string? type = reader.TokenType == JsonTokenType.String ? reader.GetString() : null;
                if (type != schema.NxType)
                {
                    throw new JsonException(
                        $"Expected '$type' to be '{schema.NxType}' for {typeof(TRecord).Name} but read '{type}'.");
                }

                continue;
            }

            if (!schema.TryGetField(key, out NxField? field))
            {
                throw new JsonException(
                    $"'{key}' is not a field of {typeof(TRecord).Name} ('{schema.NxType}').");
            }

            if (reader.TokenType == JsonTokenType.Null && !field.Clearable)
            {
                throw new JsonException(field.CannotClearMessage(schema.NxType, typeof(TRecord).Name));
            }

            record.SetFieldValue(key, JsonSerializer.Deserialize(ref reader, field.ValueType, options));
        }

        throw new JsonException($"Unexpected end of JSON while reading {typeof(TRecord).Name}.");
    }

    /// <inheritdoc />
    public override void Write(Utf8JsonWriter writer, TRecord value, JsonSerializerOptions options)
    {
        writer.WriteStartObject();
        writer.WriteString("$type", value.Schema.NxType);
        foreach (NxField field in value.Schema.FieldsInWireOrder)
        {
            if (!value.Fields.TryGetValue(field.Name, out object? fieldValue))
            {
                continue;
            }

            writer.WritePropertyName(field.Name);
            JsonSerializer.Serialize(writer, fieldValue, field.ValueType, options);
        }

        writer.WriteEndObject();
    }
}
