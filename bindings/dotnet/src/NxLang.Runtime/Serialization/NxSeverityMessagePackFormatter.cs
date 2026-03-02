// Copyright (c) Bret Johnson. All rights reserved.
// Licensed under the MIT license. See LICENSE file in the project root for full license information.

using System;
using MessagePack;
using MessagePack.Formatters;

namespace NxLang.Nx.Serialization;

internal sealed class NxSeverityMessagePackFormatter : IMessagePackFormatter<NxSeverity>
{
    public void Serialize(ref MessagePackWriter writer, NxSeverity value, MessagePackSerializerOptions options)
    {
        writer.Write(NxSeverityWireFormat.Format(value));
    }

    public NxSeverity Deserialize(ref MessagePackReader reader, MessagePackSerializerOptions options)
    {
        if (reader.TryReadNil() || reader.NextMessagePackType is not MessagePackType.String)
        {
            throw new MessagePackSerializationException("Expected NX severity to be encoded as a MessagePack string.");
        }

        string value = reader.ReadString()!;

        try
        {
            return NxSeverityWireFormat.Parse(value);
        }
        catch (FormatException e)
        {
            throw new MessagePackSerializationException(e.Message, e);
        }
    }
}
