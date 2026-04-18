// Copyright (c) Bret Johnson. All rights reserved.
// Licensed under the MIT license. See LICENSE file in the project root for full license information.

using System;
using System.Text.Json;
using System.Text.Json.Serialization;

namespace NxLang.Nx.Serialization;

/// <summary>
/// Serializes an NX enum as its authored member string.
/// </summary>
/// <typeparam name="TEnum">The CLR enum type.</typeparam>
/// <typeparam name="TWire">The explicit wire-format mapping type.</typeparam>
public sealed class NxEnumJsonConverter<TEnum, TWire> : JsonConverter<TEnum>
    where TEnum : struct, Enum
    where TWire : INxEnumWireFormat<TEnum>
{
    /// <inheritdoc />
    public override TEnum Read(ref Utf8JsonReader reader, Type typeToConvert, JsonSerializerOptions options)
    {
        if (reader.TokenType is not JsonTokenType.String)
        {
            throw new JsonException("Expected NX enum to be encoded as a JSON string.");
        }

        string value = reader.GetString()!;

        try
        {
            return TWire.Parse(value);
        }
        catch (FormatException e)
        {
            throw new JsonException(e.Message, e);
        }
    }

    /// <inheritdoc />
    public override void Write(Utf8JsonWriter writer, TEnum value, JsonSerializerOptions options)
    {
        writer.WriteStringValue(TWire.Format(value));
    }
}
