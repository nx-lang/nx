// Copyright (c) Bret Johnson. All rights reserved.
// Licensed under the MIT license. See LICENSE file in the project root for full license information.

using System;
using System.Text.Json;
using System.Text.Json.Serialization;

namespace NxLang.Nx.Serialization;

internal sealed class NxSeverityJsonConverter : JsonConverter<NxSeverity>
{
    public override NxSeverity Read(ref Utf8JsonReader reader, Type typeToConvert, JsonSerializerOptions options)
    {
        if (reader.TokenType is not JsonTokenType.String)
        {
            throw new JsonException("Expected NX severity to be encoded as a string.");
        }

        string value = reader.GetString()!;

        try
        {
            return NxSeverityWireFormat.Parse(value);
        }
        catch (FormatException e)
        {
            throw new JsonException(e.Message, e);
        }
    }

    public override void Write(Utf8JsonWriter writer, NxSeverity value, JsonSerializerOptions options)
    {
        writer.WriteStringValue(NxSeverityWireFormat.Format(value));
    }
}
