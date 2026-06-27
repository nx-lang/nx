// Copyright (c) Bret Johnson. All rights reserved.
// Licensed under the MIT license. See LICENSE file in the project root for full license information.

using System;
using System.Collections.Generic;
using System.Linq;
using System.Reflection;
using System.Text.Json.Serialization;
using MessagePack;
using MessagePack.Formatters;

namespace NxLang.Nx.Serialization;

/// <summary>
/// MessagePack formatter for a concrete or intermediate derived type that shares the polymorphic root contract
/// implemented by <see cref="NxPolymorphicMessagePackFormatter{TBase}"/>.
/// </summary>
/// <typeparam name="TBase">The abstract polymorphic root type.</typeparam>
/// <typeparam name="TDerived">A type in the hierarchy under <typeparamref name="TBase"/>.</typeparam>
[CLSCompliant(false)]
public sealed class NxPolymorphicConcreteMessagePackFormatter<TBase, TDerived> : IMessagePackFormatter<TDerived>
    where TBase : class
    where TDerived : class, TBase
{
    private static readonly NxPolymorphicMessagePackFormatter<TBase> Inner = new();

    /// <inheritdoc />
    public void Serialize(ref MessagePackWriter writer, TDerived value, MessagePackSerializerOptions options)
    {
        Inner.Serialize(ref writer, value, options);
    }

    /// <inheritdoc />
    public TDerived Deserialize(ref MessagePackReader reader, MessagePackSerializerOptions options)
    {
        TBase? deserialized = Inner.Deserialize(ref reader, options);
        return (TDerived)deserialized!;
    }
}

/// <summary>
/// Serializes abstract NX record/action roots using a canonical MessagePack map with a <c>$type</c> discriminator.
/// </summary>
/// <remarks>
/// This formatter expects polymorphic descendants to be declared with <see cref="JsonDerivedTypeAttribute"/>
/// using string discriminator values.
/// </remarks>
/// <typeparam name="TBase">The abstract root type.</typeparam>
[CLSCompliant(false)]
public sealed class NxPolymorphicMessagePackFormatter<TBase> : IMessagePackFormatter<TBase>
    where TBase : class
{
    private const string DiscriminatorKey = "$type";

    private static readonly IReadOnlyDictionary<string, Type> DiscriminatorToType = BuildDiscriminatorMap();
    private static readonly IReadOnlyDictionary<Type, string> TypeToDiscriminator = BuildTypeMap();
    private static readonly IReadOnlyDictionary<Type, IReadOnlyList<SerializableProperty>> TypeProperties = BuildTypeProperties();
    private static readonly IReadOnlyDictionary<Type, IReadOnlyDictionary<string, SerializableProperty>> TypePropertiesByWireName = BuildTypePropertiesByWireName();

    /// <inheritdoc />
    public void Serialize(ref MessagePackWriter writer, TBase value, MessagePackSerializerOptions options)
    {
        if (value is null)
        {
            writer.WriteNil();
            return;
        }

        Type runtimeType = value.GetType();
        if (!TypeToDiscriminator.TryGetValue(runtimeType, out string? discriminator))
        {
            throw new MessagePackSerializationException(
                $"No $type discriminator registration was found for polymorphic type '{runtimeType.FullName}'.");
        }

        if (!TypeProperties.TryGetValue(runtimeType, out IReadOnlyList<SerializableProperty>? properties))
        {
            throw new MessagePackSerializationException(
                $"No serializable MessagePack properties were found for polymorphic type '{runtimeType.FullName}'.");
        }

        writer.WriteMapHeader(properties.Count + 1);
        writer.Write(DiscriminatorKey);
        writer.Write(discriminator);

        foreach (SerializableProperty property in properties)
        {
            writer.Write(property.WireName);
            object? propertyValue = property.PropertyInfo.GetValue(value);
            MessagePackSerializer.Serialize(property.PropertyInfo.PropertyType, ref writer, propertyValue, options);
        }
    }

    /// <inheritdoc />
    public TBase Deserialize(ref MessagePackReader reader, MessagePackSerializerOptions options)
    {
        if (reader.TryReadNil())
        {
            return null!;
        }

        if (reader.NextMessagePackType != MessagePackType.Map)
        {
            throw new MessagePackSerializationException(
                $"Expected polymorphic MessagePack payload for '{typeof(TBase).FullName}' to be a map.");
        }

        int entryCount = reader.ReadMapHeader();

        string discriminator = PeekDiscriminator(reader.CreatePeekReader(), entryCount);

        if (!DiscriminatorToType.TryGetValue(discriminator, out Type? concreteType))
        {
            throw new MessagePackSerializationException(
                $"Unknown polymorphic $type discriminator '{discriminator}' for base type '{typeof(TBase).FullName}'.");
        }

        if (!TypePropertiesByWireName.TryGetValue(concreteType, out IReadOnlyDictionary<string, SerializableProperty>? propertiesByWireName))
        {
            throw new MessagePackSerializationException(
                $"No serializable MessagePack properties were found for polymorphic type '{concreteType.FullName}'.");
        }

        object instance = Activator.CreateInstance(concreteType)
            ?? throw new MessagePackSerializationException(
                $"Could not create an instance of polymorphic type '{concreteType.FullName}'.");

        for (int i = 0; i < entryCount; i++)
        {
            if (reader.NextMessagePackType != MessagePackType.String)
            {
                reader.Skip();
                reader.Skip();
                continue;
            }

            string? key = reader.ReadString();
            if (key == DiscriminatorKey)
            {
                reader.Skip();
                continue;
            }

            if (key is not null && propertiesByWireName.TryGetValue(key, out SerializableProperty? property))
            {
                object? value = MessagePackSerializer.Deserialize(
                    property.PropertyInfo.PropertyType, ref reader, options);
                property.PropertyInfo.SetValue(instance, value);
            }
            else
            {
                reader.Skip();
            }
        }

        return (TBase)instance;
    }

    private static string PeekDiscriminator(MessagePackReader peek, int entryCount)
    {
        for (int i = 0; i < entryCount; i++)
        {
            if (peek.NextMessagePackType != MessagePackType.String)
            {
                peek.Skip();
                peek.Skip();
                continue;
            }

            string? key = peek.ReadString();
            if (key != DiscriminatorKey)
            {
                peek.Skip();
                continue;
            }

            if (peek.NextMessagePackType != MessagePackType.String)
            {
                throw new MessagePackSerializationException(
                    $"Expected '$type' discriminator for '{typeof(TBase).FullName}' to be a MessagePack string.");
            }

            string? discriminator = peek.ReadString();
            if (string.IsNullOrWhiteSpace(discriminator))
            {
                throw new MessagePackSerializationException(
                    $"Polymorphic '$type' discriminator for '{typeof(TBase).FullName}' must be a non-empty string.");
            }

            return discriminator;
        }

        throw new MessagePackSerializationException(
            $"Expected polymorphic MessagePack payload for '{typeof(TBase).FullName}' to include a string '$type' key.");
    }

    private static IReadOnlyDictionary<string, Type> BuildDiscriminatorMap()
    {
        Dictionary<string, Type> map = new(StringComparer.Ordinal);
        foreach (JsonDerivedTypeAttribute attribute in typeof(TBase).GetCustomAttributes<JsonDerivedTypeAttribute>())
        {
            if (attribute.DerivedType is null || attribute.TypeDiscriminator is null)
            {
                continue;
            }

            if (attribute.TypeDiscriminator is not string discriminator)
            {
                throw new InvalidOperationException(
                    $"Polymorphic type '{typeof(TBase).FullName}' uses non-string discriminator metadata; MessagePack polymorphic serialization requires string '$type' discriminators.");
            }

            map[discriminator] = attribute.DerivedType;
        }

        if (map.Count == 0)
        {
            throw new InvalidOperationException(
                $"No JsonDerivedType registrations with string discriminators were found for polymorphic base type '{typeof(TBase).FullName}'.");
        }

        return map;
    }

    private static IReadOnlyDictionary<Type, string> BuildTypeMap()
    {
        Dictionary<Type, string> map = new();
        foreach (KeyValuePair<string, Type> pair in DiscriminatorToType)
        {
            map[pair.Value] = pair.Key;
        }

        return map;
    }

    private static IReadOnlyDictionary<Type, IReadOnlyList<SerializableProperty>> BuildTypeProperties()
    {
        Dictionary<Type, IReadOnlyList<SerializableProperty>> map = new();
        foreach (Type type in TypeToDiscriminator.Keys)
        {
            SerializableProperty[] properties = type
                .GetProperties(BindingFlags.Instance | BindingFlags.Public)
                .Where(property => property.CanRead && property.CanWrite)
                .Select(property => new SerializableProperty(property, ResolveWireName(property)))
                .Where(property => !string.IsNullOrEmpty(property.WireName))
                .ToArray();
            map[type] = properties;
        }

        return map;
    }

    private static IReadOnlyDictionary<Type, IReadOnlyDictionary<string, SerializableProperty>> BuildTypePropertiesByWireName()
    {
        Dictionary<Type, IReadOnlyDictionary<string, SerializableProperty>> map = new();
        foreach (KeyValuePair<Type, IReadOnlyList<SerializableProperty>> entry in TypeProperties)
        {
            Dictionary<string, SerializableProperty> lookup = new(StringComparer.Ordinal);
            foreach (SerializableProperty property in entry.Value)
            {
                lookup[property.WireName] = property;
            }

            map[entry.Key] = lookup;
        }

        return map;
    }

    private static string ResolveWireName(PropertyInfo property)
    {
        KeyAttribute? key = property.GetCustomAttribute<KeyAttribute>();
        if (key?.IntKey is not null)
        {
            throw new InvalidOperationException(
                $"Property '{property.DeclaringType?.FullName}.{property.Name}' uses an integer MessagePack key. NxPolymorphicMessagePackFormatter requires string keys that match NX wire field names.");
        }

        return key?.StringKey ?? string.Empty;
    }

    private sealed record SerializableProperty(PropertyInfo PropertyInfo, string WireName);
}
