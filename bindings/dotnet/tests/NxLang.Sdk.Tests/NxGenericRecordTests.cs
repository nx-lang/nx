// Copyright (c) Bret Johnson. All rights reserved.
// Licensed under the MIT license. See LICENSE file in the project root for full license information.

using System.Collections.Generic;
using System.Text.Json;
using MessagePack;
using NxLang.Sdk.Tests.Generated;
using Xunit;

namespace NxLang.Nx.Tests;

/// <summary>
/// A generated generic record is an ordinary closed instantiation at the host boundary: both
/// serializers round-trip it with the reflection-based resolvers, and the type argument is
/// nowhere on the wire.
/// </summary>
public class NxGenericRecordTests
{
    [Fact]
    public void GenericRecord_RoundTripsThroughSystemTextJson()
    {
        Range<long> range = new()
        {
            Start = 1,
            End = 5,
            EndInclusive = true,
        };

        string json = JsonSerializer.Serialize(range);
        using (JsonDocument document = JsonDocument.Parse(json))
        {
            Assert.Equal(1, document.RootElement.GetProperty("start").GetInt64());
            Assert.Equal(5, document.RootElement.GetProperty("end").GetInt64());
            Assert.True(document.RootElement.GetProperty("endInclusive").GetBoolean());
            Assert.False(document.RootElement.TryGetProperty("T", out _));
        }

        Range<long>? read = JsonSerializer.Deserialize<Range<long>>(json);
        Assert.NotNull(read);
        Assert.Equal(1, read!.Start);
        Assert.Equal(5, read.End);
        Assert.True(read.EndInclusive);
    }

    [Fact]
    public void GenericRecord_RoundTripsThroughMessagePack()
    {
        Range<double> range = new()
        {
            Start = 0.5,
            End = 1.5,
            EndInclusive = false,
        };

        byte[] bytes = MessagePackSerializer.Serialize(
            range,
            cancellationToken: TestContext.Current.CancellationToken);

        Dictionary<string, object?> payload =
            MessagePackSerializer.Deserialize<Dictionary<string, object?>>(
                bytes,
                cancellationToken: TestContext.Current.CancellationToken);
        Assert.Equal(0.5, Assert.IsType<double>(payload["start"]));
        Assert.Equal(1.5, Assert.IsType<double>(payload["end"]));
        Assert.False(payload.ContainsKey("T"));

        Range<double> read = MessagePackSerializer.Deserialize<Range<double>>(
            bytes,
            cancellationToken: TestContext.Current.CancellationToken);
        Assert.Equal(0.5, read.Start);
        Assert.Equal(1.5, read.End);
        Assert.False(read.EndInclusive);
    }

    /// <summary>
    /// A record holding two different instantiations keeps them apart on the wire and back.
    /// </summary>
    [Fact]
    public void AppliedTypeFields_RoundTripAsTheirInstantiations()
    {
        Schedule schedule = new()
        {
            Week = new Range<long> { Start = 1, End = 7, EndInclusive = true },
            Spans = [new Range<double> { Start = 0.0, End = 0.5, EndInclusive = false }],
        };

        string json = JsonSerializer.Serialize(schedule);
        Schedule? read = JsonSerializer.Deserialize<Schedule>(json);
        Assert.NotNull(read);
        Assert.Equal(7, read!.Week.End);
        Assert.NotNull(read.Spans);
        Assert.Equal(0.5, read.Spans![0].End);

        byte[] bytes = MessagePackSerializer.Serialize(
            schedule,
            cancellationToken: TestContext.Current.CancellationToken);
        Schedule fromBytes = MessagePackSerializer.Deserialize<Schedule>(
            bytes,
            cancellationToken: TestContext.Current.CancellationToken);
        Assert.Equal(7, fromBytes.Week.End);
        Assert.Equal(0.5, fromBytes.Spans![0].End);
    }

    /// <summary>
    /// A generic record's update companion is generic too, so it is usable at the instantiation a
    /// program actually produces: diff two <c>Range&lt;long&gt;</c>, read the patch typed, and
    /// apply it back.
    /// </summary>
    [Fact]
    public void GenericUpdateCompanion_DiffsAndAppliesAtAnInstantiation()
    {
        Range<long> before = new() { Start = 1, End = 5, EndInclusive = false };
        Range<long> after = new() { Start = 1, End = 9, EndInclusive = true };

        Range_update<long> patch = Range_update<long>.Diff(before, after);

        Assert.Equal([Range_property.End, Range_property.EndInclusive], patch.Changed());
        Assert.False(patch.Start.HasValue);
        Assert.Equal(9, patch.End.Value);

        Range<long> applied = patch.Apply(before);
        Assert.Equal(1, applied.Start);
        Assert.Equal(9, applied.End);
        Assert.True(applied.EndInclusive);
    }

    /// <summary>
    /// A patch that came off the wire applies to the instantiation it patches.
    /// </summary>
    /// <remarks>
    /// This is the case an erased companion cannot serve. With the fields typed <c>object</c>, JSON
    /// deserializes <c>end</c> as a <see cref="JsonElement"/> and applying it to a
    /// <c>Range&lt;long&gt;</c> throws, because the record's field is <c>long</c>. The companion
    /// carrying the record's parameter is what makes the schema say <c>long</c> and the round trip
    /// close.
    /// </remarks>
    [Fact]
    public void GenericUpdateCompanion_RoundTripsThroughBothSerializersAndStillApplies()
    {
        Range<long> before = new() { Start = 1, End = 5, EndInclusive = false };
        Range<long> after = new() { Start = 1, End = 9, EndInclusive = true };
        Range_update<long> patch = Range_update<long>.Diff(before, after);

        string json = JsonSerializer.Serialize(patch);
        using (JsonDocument document = JsonDocument.Parse(json))
        {
            JsonElement root = document.RootElement;
            Assert.Equal("Range.Update", root.GetProperty("$type").GetString());
            Assert.Equal(9, root.GetProperty("end").GetInt64());
            Assert.False(root.TryGetProperty("start", out _));
            Assert.False(root.TryGetProperty("T", out _));
        }

        Range_update<long>? fromJson = JsonSerializer.Deserialize<Range_update<long>>(json);
        Assert.NotNull(fromJson);
        Assert.Equal(9, fromJson!.End.Value);
        Assert.Equal(9, fromJson.Apply(before).End);

        byte[] bytes = MessagePackSerializer.Serialize(
            patch,
            cancellationToken: TestContext.Current.CancellationToken);
        Range_update<long> fromBytes = MessagePackSerializer.Deserialize<Range_update<long>>(
            bytes,
            cancellationToken: TestContext.Current.CancellationToken);
        Assert.Equal(9, fromBytes.End.Value);
        Assert.Equal(9, fromBytes.Apply(before).End);
    }

    /// <summary>
    /// One companion type serves every instantiation independently, and the wire carries no
    /// type argument either way.
    /// </summary>
    [Fact]
    public void GenericUpdateCompanion_KeepsInstantiationsApart()
    {
        Range_update<double> doubles = Range_update<double>.Diff(
            new Range<double> { Start = 0.5, End = 1.5 },
            new Range<double> { Start = 0.5, End = 2.5 });
        Assert.Equal(2.5, doubles.End.Value);

        Range_update<string> strings = Range_update<string>.Diff(
            new Range<string> { Start = "a", End = "b" },
            new Range<string> { Start = "a", End = "c" });
        Assert.Equal("c", strings.End.Value);

        Assert.Equal(
            "Range.Update",
            JsonDocument.Parse(JsonSerializer.Serialize(doubles))
                .RootElement.GetProperty("$type")
                .GetString());
    }
}
