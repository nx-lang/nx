// Copyright (c) The NX Authors.
// Licensed under the MIT license. See LICENSE file in the project root for full license information.

using System;
using System.Text.Json;
using System.Text.Json.Serialization;

namespace NxLang.Nx.Serialization;

/// <summary>
/// Builds the <see cref="NxUpdateRecordJsonConverter{TRecord}"/> for a generated update record DTO whose type is
/// generic, which cannot name its own converter in an attribute.
/// </summary>
/// <remarks>
/// <para>A generic record's companion is generic too — <c>Range_update&lt;T&gt;</c> patches
/// <c>Range&lt;T&gt;</c> — and <c>[JsonConverter(typeof(NxUpdateRecordJsonConverter&lt;Range_update&lt;T&gt;&gt;))]</c>
/// does not compile: an attribute argument cannot use type parameters (CS0416). Typegen names this factory
/// instead, and it closes the converter over whichever instantiation is being serialized.</para>
/// <para>A non-generic companion names its converter directly and never reaches this class.</para>
/// </remarks>
public sealed class NxUpdateRecordJsonConverterFactory : JsonConverterFactory
{
    /// <inheritdoc />
    public override bool CanConvert(Type typeToConvert)
    {
        ArgumentNullException.ThrowIfNull(typeToConvert);
        return typeof(NxUpdateRecord).IsAssignableFrom(typeToConvert)
            && !typeToConvert.IsAbstract
            && typeToConvert.GetConstructor(Type.EmptyTypes) is not null;
    }

    /// <inheritdoc />
    public override JsonConverter? CreateConverter(Type typeToConvert, JsonSerializerOptions options)
    {
        ArgumentNullException.ThrowIfNull(typeToConvert);
        if (!CanConvert(typeToConvert))
        {
            return null;
        }

        Type converterType = typeof(NxUpdateRecordJsonConverter<>).MakeGenericType(typeToConvert);
        return (JsonConverter?)Activator.CreateInstance(converterType);
    }
}
