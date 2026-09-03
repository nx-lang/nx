// Copyright (c) Bret Johnson. All rights reserved.
// Licensed under the MIT license. See LICENSE file in the project root for full license information.

using System;
using System.Collections.Generic;
using System.Reflection;
using System.Text.Json;
using System.Text.Json.Serialization;

namespace NxLang.Nx.Serialization;

/// <summary>
/// Serializes an NX discriminated union whose cases do not all share one wire shape.
/// </summary>
/// <remarks>
/// <para>A constant case — one that declares no fields in a union that declares no base — carries
/// nothing beyond its own name, so it is written and read as that bare JSON string. Every other
/// case keeps the <c>$type</c> object that <see cref="JsonPolymorphicAttribute"/> produces, and is
/// delegated to the default polymorphic handling.</para>
/// <para>This converter is emitted only for a union that actually has a constant case alongside a
/// payload case. A union whose cases are all constant generates a CLR <c>enum</c> instead, and one
/// with no constant case needs nothing beyond <see cref="JsonPolymorphicAttribute"/>.</para>
/// </remarks>
/// <typeparam name="TBase">The abstract union root type.</typeparam>
public sealed class NxPolymorphicJsonConverter<TBase> : JsonConverter<TBase>
    where TBase : class
{
    private const string DiscriminatorName = "$type";

    private static readonly IReadOnlyDictionary<string, Type> DiscriminatorToType = BuildDiscriminatorMap();
    private static readonly IReadOnlyDictionary<Type, string> TypeToDiscriminator = BuildTypeMap();
    private static readonly IReadOnlyDictionary<string, Type> ConstantCaseNameToType = BuildConstantCaseNameMap();
    private static readonly IReadOnlyDictionary<Type, string> TypeToConstantCaseName = BuildConstantCaseTypeMap();

    /// <inheritdoc />
    public override TBase? Read(ref Utf8JsonReader reader, Type typeToConvert, JsonSerializerOptions options)
    {
        if (reader.TokenType is JsonTokenType.Null)
        {
            return null;
        }

        // A constant case is the bare case name.
        if (reader.TokenType is JsonTokenType.String)
        {
            string caseName = reader.GetString()!;
            if (!ConstantCaseNameToType.TryGetValue(caseName, out Type? constantType))
            {
                throw new JsonException(
                    $"Unknown constant union case '{caseName}' for base type '{typeof(TBase).FullName}'.");
            }

            return (TBase)ConstantCaseInstance(constantType);
        }

        if (reader.TokenType is not JsonTokenType.StartObject)
        {
            throw new JsonException(
                $"Expected union '{typeof(TBase).FullName}' to be a JSON object or a constant case name.");
        }

        // Every other case is a `$type` object. The discriminator is read from a copy so the
        // concrete type can then be deserialized from the whole object.
        using JsonDocument document = JsonDocument.ParseValue(ref reader);
        if (!document.RootElement.TryGetProperty(DiscriminatorName, out JsonElement discriminatorElement)
            || discriminatorElement.ValueKind is not JsonValueKind.String)
        {
            throw new JsonException(
                $"Expected union '{typeof(TBase).FullName}' to carry a string '{DiscriminatorName}' discriminator.");
        }

        string discriminator = discriminatorElement.GetString()!;
        if (!DiscriminatorToType.TryGetValue(discriminator, out Type? caseType))
        {
            throw new JsonException(
                $"Unknown '{DiscriminatorName}' discriminator '{discriminator}' for base type '{typeof(TBase).FullName}'.");
        }

        return (TBase?)document.RootElement.Deserialize(caseType, options);
    }

    /// <inheritdoc />
    public override void Write(Utf8JsonWriter writer, TBase value, JsonSerializerOptions options)
    {
        if (value is null)
        {
            writer.WriteNullValue();
            return;
        }

        Type runtimeType = value.GetType();
        if (TypeToConstantCaseName.TryGetValue(runtimeType, out string? caseName))
        {
            writer.WriteStringValue(caseName);
            return;
        }

        if (!TypeToDiscriminator.TryGetValue(runtimeType, out string? discriminator))
        {
            throw new JsonException(
                $"No case registration was found for '{runtimeType.FullName}' under '{typeof(TBase).FullName}'.");
        }

        // The discriminator is written first, then the case's own properties, so the object
        // matches what `[JsonPolymorphic]` produces for a union with one wire shape.
        writer.WriteStartObject();
        writer.WriteString(DiscriminatorName, discriminator);

        using JsonDocument document = JsonSerializer.SerializeToDocument(value, runtimeType, options);
        foreach (JsonProperty property in document.RootElement.EnumerateObject())
        {
            if (property.NameEquals(DiscriminatorName))
            {
                continue;
            }

            property.WriteTo(writer);
        }

        writer.WriteEndObject();
    }

    /// <summary>Returns the single instance of a constant case type.</summary>
    private static object ConstantCaseInstance(Type caseType)
    {
        FieldInfo? instance = caseType.GetField("Instance", BindingFlags.Public | BindingFlags.Static);
        return instance?.GetValue(null)
            ?? Activator.CreateInstance(caseType)
            ?? throw new JsonException(
                $"Could not obtain an instance of constant union case '{caseType.FullName}'.");
    }

    private static IReadOnlyDictionary<string, Type> BuildDiscriminatorMap()
    {
        Dictionary<string, Type> map = new(StringComparer.Ordinal);
        foreach (NxUnionCaseAttribute attribute in typeof(TBase).GetCustomAttributes<NxUnionCaseAttribute>())
        {
            map[attribute.Discriminator] = attribute.CaseType;
        }

        if (map.Count == 0)
        {
            throw new InvalidOperationException(
                $"No NxUnionCase registrations were found for union base type '{typeof(TBase).FullName}'.");
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

    private static IReadOnlyDictionary<string, Type> BuildConstantCaseNameMap()
    {
        Dictionary<string, Type> map = new(StringComparer.Ordinal);
        foreach (Type caseType in DiscriminatorToType.Values)
        {
            NxConstantCaseAttribute? constant = caseType.GetCustomAttribute<NxConstantCaseAttribute>();
            if (constant is not null)
            {
                map[constant.WireName] = caseType;
            }
        }

        return map;
    }

    private static IReadOnlyDictionary<Type, string> BuildConstantCaseTypeMap()
    {
        Dictionary<Type, string> map = new();
        foreach (KeyValuePair<string, Type> pair in ConstantCaseNameToType)
        {
            map[pair.Value] = pair.Key;
        }

        return map;
    }
}
