// Copyright (c) Bret Johnson. All rights reserved.
// Licensed under the MIT license. See LICENSE file in the project root for full license information.

using System;
using System.Collections.Generic;
using System.Linq;
using System.Reflection;
using System.Text.Json;
using System.Text.Json.Serialization;
using MessagePack;
using MessagePack.Formatters;

namespace NxLang.Nx.Serialization;

/// <summary>
/// Creates System.Text.Json converters for <see cref="NxOptional{T}"/>.
/// </summary>
public sealed class NxOptionalJsonConverterFactory : JsonConverterFactory
{
    /// <inheritdoc />
    public override bool CanConvert(Type typeToConvert)
    {
        return typeToConvert.IsGenericType && typeToConvert.GetGenericTypeDefinition() == typeof(NxOptional<>);
    }

    /// <inheritdoc />
    public override JsonConverter? CreateConverter(Type typeToConvert, JsonSerializerOptions options)
    {
        Type valueType = typeToConvert.GetGenericArguments()[0];
        return (JsonConverter?)Activator.CreateInstance(
            typeof(NxOptionalJsonConverter<>).MakeGenericType(valueType));
    }
}

/// <summary>
/// Reads and writes the value of a set <see cref="NxOptional{T}"/>; a present <see langword="null"/> reads as set
/// to null.
/// </summary>
/// <remarks>
/// A converter cannot remove its property from the containing object, so an unset value is omitted by marking the
/// property <c>[JsonIgnore(Condition = JsonIgnoreCondition.WhenWritingDefault)]</c>. A missing key never reaches the
/// converter, which is what leaves the property unset. An unset value that does reach the converter is a misuse
/// and throws, since writing <see langword="null"/> for it would turn "unchanged" into "set to null".
/// </remarks>
/// <typeparam name="T">The type of the value when set.</typeparam>
public sealed class NxOptionalJsonConverter<T> : JsonConverter<NxOptional<T>>
{
    /// <inheritdoc />
    public override bool HandleNull => true;

    /// <inheritdoc />
    public override NxOptional<T> Read(ref Utf8JsonReader reader, Type typeToConvert, JsonSerializerOptions options)
    {
        return new NxOptional<T>(JsonSerializer.Deserialize<T>(ref reader, options)!);
    }

    /// <inheritdoc />
    public override void Write(Utf8JsonWriter writer, NxOptional<T> value, JsonSerializerOptions options)
    {
        if (!value.HasValue)
        {
            throw new InvalidOperationException(
                $"An unset {typeof(NxOptional<T>).Name} cannot be written as JSON; mark the property "
                + "[JsonIgnore(Condition = JsonIgnoreCondition.WhenWritingDefault)] so it is omitted.");
        }

        JsonSerializer.Serialize(writer, value.Value, options);
    }
}

/// <summary>
/// Reads and writes the value of a set <see cref="NxOptional{T}"/> in MessagePack; a present nil reads as set to null.
/// </summary>
/// <remarks>
/// Omitting an unset value is the containing object's job, which
/// <see cref="NxUpdateRecordMessagePackFormatter{TRecord}"/> does. An unset value that reaches this formatter is a
/// misuse and throws, since writing nil for it would turn "unchanged" into "set to null".
/// </remarks>
/// <typeparam name="T">The type of the value when set.</typeparam>
[CLSCompliant(false)]
public sealed class NxOptionalMessagePackFormatter<T> : IMessagePackFormatter<NxOptional<T>>
{
    /// <inheritdoc />
    public void Serialize(ref MessagePackWriter writer, NxOptional<T> value, MessagePackSerializerOptions options)
    {
        if (!value.HasValue)
        {
            throw new InvalidOperationException(
                $"An unset {typeof(NxOptional<T>).Name} cannot be written as MessagePack; give the containing type "
                + "[MessagePackFormatter(typeof(NxUpdateRecordMessagePackFormatter<>))] so it is omitted.");
        }

        MessagePackSerializer.Serialize(ref writer, value.Value, options);
    }

    /// <inheritdoc />
    public NxOptional<T> Deserialize(ref MessagePackReader reader, MessagePackSerializerOptions options)
    {
        return new NxOptional<T>(MessagePackSerializer.Deserialize<T>(ref reader, options));
    }
}

/// <summary>
/// Serializes an NX update record DTO as a MessagePack map holding only the keys whose values are present.
/// </summary>
/// <remarks>
/// Every property with a string <see cref="KeyAttribute"/> is written in ordinal key order, which puts the
/// <c>$type</c> discriminator first, except an <see cref="NxOptional{T}"/> property that is unset, which is
/// omitted. On read, a key present in the map sets its property and a missing key leaves it unset. A property
/// without a setter, such as the discriminator, is written and not read.
/// </remarks>
/// <typeparam name="TRecord">The update record DTO type.</typeparam>
[CLSCompliant(false)]
public sealed class NxUpdateRecordMessagePackFormatter<TRecord> : IMessagePackFormatter<TRecord>
    where TRecord : class, new()
{
    private static readonly IReadOnlyList<KeyedProperty> Properties = typeof(TRecord)
        .GetProperties(BindingFlags.Public | BindingFlags.Instance)
        .Select(property => (property, key: property.GetCustomAttribute<KeyAttribute>()?.StringKey))
        .Where(entry => entry.key is not null)
        .OrderBy(entry => entry.key, StringComparer.Ordinal)
        .Select(entry => new KeyedProperty(entry.property, entry.key!))
        .ToArray();

    private static readonly IReadOnlyDictionary<string, KeyedProperty> PropertiesByKey =
        Properties.ToDictionary(property => property.Key, StringComparer.Ordinal);

    /// <inheritdoc />
    public void Serialize(ref MessagePackWriter writer, TRecord value, MessagePackSerializerOptions options)
    {
        if (value is null)
        {
            writer.WriteNil();
            return;
        }

        List<(KeyedProperty Property, object? Value)> present = new(Properties.Count);
        foreach (KeyedProperty property in Properties)
        {
            object? propertyValue = property.Info.GetValue(value);
            if (property.IsOptional && !property.IsSet(propertyValue))
            {
                continue;
            }

            present.Add((property, propertyValue));
        }

        writer.WriteMapHeader(present.Count);
        foreach ((KeyedProperty property, object? propertyValue) in present)
        {
            writer.Write(property.Key);
            MessagePackSerializer.Serialize(property.Info.PropertyType, ref writer, propertyValue, options);
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
            int count = reader.ReadMapHeader();
            for (int i = 0; i < count; i++)
            {
                string? key = reader.NextMessagePackType == MessagePackType.String ? reader.ReadString() : null;
                if (key is null
                    || !PropertiesByKey.TryGetValue(key, out KeyedProperty? property)
                    || !property.Info.CanWrite)
                {
                    if (key is null)
                    {
                        reader.Skip();
                    }

                    reader.Skip();
                    continue;
                }

                object? propertyValue =
                    MessagePackSerializer.Deserialize(property.Info.PropertyType, ref reader, options);
                property.Info.SetValue(record, propertyValue);
            }

            return record;
        }
        finally
        {
            reader.Depth--;
        }
    }

    private sealed class KeyedProperty
    {
        private readonly PropertyInfo? _hasValue;

        public KeyedProperty(PropertyInfo info, string key)
        {
            Info = info;
            Key = key;
            IsOptional = info.PropertyType.IsGenericType
                && info.PropertyType.GetGenericTypeDefinition() == typeof(NxOptional<>);
            _hasValue = IsOptional ? info.PropertyType.GetProperty(nameof(NxOptional<int>.HasValue)) : null;
        }

        public PropertyInfo Info { get; }

        public string Key { get; }

        public bool IsOptional { get; }

        public bool IsSet(object? optional) => optional is not null && (bool)_hasValue!.GetValue(optional)!;
    }
}
