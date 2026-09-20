// Copyright (c) Bret Johnson. All rights reserved.
// Licensed under the MIT license. See LICENSE file in the project root for full license information.

using System.Collections.Generic;
using System.Linq;
using System.Text.Json;
using MessagePack;
using Xunit;

namespace NxLang.Nx.Tests;

/// <summary>
/// The SDK's hand-written <see cref="NxRange{T}"/>, which generated C# names wherever a contract refers to the NX
/// prelude's <c>Range</c>.
/// </summary>
/// <remarks>
/// The prelude's declarations are not generated into a host's namespace, so what makes them usable from .NET is that
/// this type's wire shape is exactly what the emitter would have generated. These cases pin that shape in both output
/// formats, pin the value equality a host compares ranges with, and fail if the prelude's declaration and this type
/// stop matching.
/// </remarks>
public class NxRangeTests
{
    [Fact]
    public void NxRange_RoundTripsThroughSystemTextJson()
    {
        NxRange<long> range = new(1, 5, false);

        string json = JsonSerializer.Serialize(range);
        using (JsonDocument document = JsonDocument.Parse(json))
        {
            Assert.Equal(1, document.RootElement.GetProperty("start").GetInt64());
            Assert.Equal(5, document.RootElement.GetProperty("end").GetInt64());
            Assert.False(document.RootElement.GetProperty("endInclusive").GetBoolean());
        }

        NxRange<long>? read = JsonSerializer.Deserialize<NxRange<long>>(json);
        Assert.NotNull(read);
        Assert.Equal(range, read);
    }

    [Fact]
    public void NxRange_RoundTripsThroughMessagePack()
    {
        NxRange<long> range = new(1, 5, false);

        byte[] bytes = MessagePackSerializer.Serialize(
            range,
            cancellationToken: TestContext.Current.CancellationToken);
        Dictionary<string, object?> payload =
            MessagePackSerializer.Deserialize<Dictionary<string, object?>>(
                bytes,
                cancellationToken: TestContext.Current.CancellationToken);
        // MessagePack encodes a small integer in its most compact form, so the value is compared
        // numerically rather than by its boxed carrier.
        Assert.Equal(1L, System.Convert.ToInt64(payload["start"], System.Globalization.CultureInfo.InvariantCulture));
        Assert.Equal(5L, System.Convert.ToInt64(payload["end"], System.Globalization.CultureInfo.InvariantCulture));
        Assert.False(Assert.IsType<bool>(payload["endInclusive"]));

        NxRange<long> read = MessagePackSerializer.Deserialize<NxRange<long>>(
            bytes,
            cancellationToken: TestContext.Current.CancellationToken);
        Assert.Equal(range, read);
    }

    [Fact]
    public void NxRange_ComparesByItsBounds()
    {
        Assert.Equal(new NxRange<long>(1, 5, false), new NxRange<long>(1, 5, false));
        Assert.True(new NxRange<long>(1, 5, false) == new NxRange<long>(1, 5, false));
        Assert.NotEqual(new NxRange<long>(1, 5, false), new NxRange<long>(1, 5, true));
        Assert.NotEqual(new NxRange<long>(1, 5, false), new NxRange<long>(1, 4, false));
        Assert.True(new NxRange<long>(1, 5, false) != new NxRange<long>(2, 5, false));
        Assert.Equal(
            new NxRange<long>(1, 5, false).GetHashCode(),
            new NxRange<long>(1, 5, false).GetHashCode());
    }

    [Fact]
    public void EvaluatedRange_DeserializesIntoTheSdkType()
    {
        NxRange<long> range = NxRuntime.Evaluate<NxRange<long>>("let root() = { 1..=5 }", "ranges.nx");

        Assert.Equal(1, range.Start);
        Assert.Equal(5, range.End);
        Assert.True(range.EndInclusive);
    }

    /// <summary>
    /// The prelude's <c>Range</c> and this type must not drift apart.
    /// </summary>
    /// <remarks>
    /// <para>The range NX evaluates carries the prelude's declared fields, and a field with a default is materialized
    /// into the value, so comparing its canonical JSON members with this type's fails as soon as the declaration gains,
    /// loses or renames one.</para>
    /// <para>The member names alone would not catch a field whose <em>type</em> changed — <c>endInclusive:boolean</c>
    /// becoming an <c>int</c> keeps the name — so the JSON value kinds are compared too. A change in the shape the
    /// emitter would generate is caught on the compiler side instead, by the checked-in
    /// <c>Generated/PreludeCompanions.g.cs</c> fixture.</para>
    /// </remarks>
    [Fact]
    public void NxRange_TracksThePreludeDeclaration()
    {
        JsonElement evaluated = NxRuntime.EvaluateJson("let root() = { 1..=5 }", "ranges.nx");
        List<KeyValuePair<string, JsonValueKind>> declared = evaluated
            .EnumerateObject()
            .Where(property => property.Name != "$type")
            .Select(property => new KeyValuePair<string, JsonValueKind>(property.Name, Kind(property.Value)))
            .OrderBy(member => member.Key, System.StringComparer.Ordinal)
            .ToList();

        using JsonDocument sdk = JsonDocument.Parse(JsonSerializer.Serialize(new NxRange<long>(1, 5, true)));
        List<KeyValuePair<string, JsonValueKind>> members = sdk.RootElement
            .EnumerateObject()
            .Select(property => new KeyValuePair<string, JsonValueKind>(property.Name, Kind(property.Value)))
            .OrderBy(member => member.Key, System.StringComparer.Ordinal)
            .ToList();

        Assert.Equal(declared, members);

        // The comparison is only as strong as the kinds it distinguishes, so assert what they are rather than trusting
        // two lists that would also match if every field serialized the same way.
        Assert.Equal(
            new List<KeyValuePair<string, JsonValueKind>>
            {
                new("end", JsonValueKind.Number),
                new("endInclusive", JsonValueKind.True),
                new("start", JsonValueKind.Number),
            },
            declared);
    }

    /// <summary>
    /// A JSON value's kind, with <see cref="JsonValueKind.False"/> folded into <see cref="JsonValueKind.True"/> so a
    /// boolean compares as a boolean rather than as the value it happens to hold.
    /// </summary>
    private static JsonValueKind Kind(JsonElement value)
    {
        return value.ValueKind == JsonValueKind.False ? JsonValueKind.True : value.ValueKind;
    }
}
