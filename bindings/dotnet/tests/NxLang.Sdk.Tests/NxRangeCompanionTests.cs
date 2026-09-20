// Copyright (c) Bret Johnson. All rights reserved.
// Licensed under the MIT license. See LICENSE file in the project root for full license information.

using System.Collections.Generic;
using System.Linq;
using System.Text.Json;
using MessagePack;
using NxLang.Sdk.Tests.Generated;
using Xunit;

namespace NxLang.Nx.Tests;

/// <summary>
/// The SDK's hand-written <see cref="NxRange_update{T}"/> and <see cref="NxRange_property"/>, which generated C# names
/// wherever a contract refers to the NX prelude's derived companions.
/// </summary>
/// <remarks>
/// <para>A prelude declaration is an ordinary declaration, so a contract field may be typed
/// <c>&lt;Range.Update T=int/&gt;</c> exactly as it may be typed <c>User.Update</c>. The companions are not generated
/// into a host's namespace for the reason <see cref="NxRange{T}"/> is not, so these cases pin that the hand-written
/// copies carry the wire shape the emitter would have generated, and that a generated contract holding them
/// round-trips.</para>
/// <para><c>Generated/PreludeCompanions.g.cs</c> is the generated contract, checked in and pinned by the nx-cli
/// typegen tests.</para>
/// </remarks>
public class NxRangeCompanionTests
{
    [Fact]
    public void NxRangeUpdate_CarriesOnlyTheFieldsThatWereSet()
    {
        NxRange_update<long> patch = new() { End = 9 };

        Assert.True(patch.IsSet(NxRange_property.End));
        Assert.False(patch.IsSet(NxRange_property.Start));
        Assert.Equal(new[] { NxRange_property.End }, patch.Changed());

        using JsonDocument document = JsonDocument.Parse(JsonSerializer.Serialize(patch));
        List<string> members = document.RootElement
            .EnumerateObject()
            .Select(property => property.Name)
            .ToList();
        Assert.Equal(new[] { "$type", "end" }, members);
        Assert.Equal("Range.Update", document.RootElement.GetProperty("$type").GetString());
        Assert.Equal(9, document.RootElement.GetProperty("end").GetInt64());
    }

    [Fact]
    public void NxRangeUpdate_DiffsTwoRanges()
    {
        NxRange_update<long> patch = NxRange_update<long>.Diff(
            new NxRange<long>(1, 5, false),
            new NxRange<long>(1, 9, false));

        Assert.Equal(new[] { NxRange_property.End }, patch.Changed());
        Assert.Equal(9, patch.End.Value);

        patch.Unset(NxRange_property.End);
        Assert.Empty(patch.Changed());
    }

    [Fact]
    public void NxRangeProperties_ReadAndWriteTheFieldTheyName()
    {
        NxRange<long> range = new(1, 5, false);

        Assert.Equal(1L, NxRangeProperties<long>.Of(NxRange_property.Start).GetValue(range));
        NxRangeProperties<long>.Of(NxRange_property.End).SetValue(range, 9L);
        Assert.Equal(9, range.End);
    }

    /// <summary>
    /// A generated contract whose fields are typed by the prelude's companions round-trips in both formats.
    /// </summary>
    [Fact]
    public void GeneratedContract_RoundTripsThePreludeCompanions()
    {
        Patch patch = new()
        {
            Span = new NxRange<long>(0, 4, false),
            Change = new NxRange_update<long> { End = 9 },
            Which = NxRange_property.EndInclusive,
            Wide = new NxRange_update<double> { Start = 0.5 },
        };

        string json = JsonSerializer.Serialize(patch);
        Patch? fromJson = JsonSerializer.Deserialize<Patch>(json);
        Assert.NotNull(fromJson);
        Assert.Equal(patch.Span, fromJson.Span);
        Assert.Equal(9, fromJson.Change.End.Value);
        Assert.False(fromJson.Change.IsSet(NxRange_property.Start));
        Assert.Equal(NxRange_property.EndInclusive, fromJson.Which);
        Assert.Equal(0.5, fromJson.Wide.Start.Value);
        Assert.False(fromJson.Wide.IsSet(NxRange_property.End));

        byte[] bytes = MessagePackSerializer.Serialize(
            patch,
            cancellationToken: TestContext.Current.CancellationToken);
        Patch fromMessagePack = MessagePackSerializer.Deserialize<Patch>(
            bytes,
            cancellationToken: TestContext.Current.CancellationToken);
        Assert.Equal(patch.Span, fromMessagePack.Span);
        Assert.Equal(9, fromMessagePack.Change.End.Value);
        Assert.Equal(NxRange_property.EndInclusive, fromMessagePack.Which);

        // `Change` and `Wide` are two instantiations of one companion, each with a closed formatter
        // of its own. MessagePack's MsgPack009 reads those as two formatters for `NxRange_update<T>`
        // and the generated header suppresses it; these two assertions are what say the suppression
        // is sound — the resolver picked the formatter of each member's own closed type, so the
        // `long` patch did not come back through the `double` one or the other way about.
        Assert.Equal(0.5, fromMessagePack.Wide.Start.Value);
        Assert.False(fromMessagePack.Wide.IsSet(NxRange_property.End));
        Assert.False(fromMessagePack.Change.IsSet(NxRange_property.Start));
    }

    /// <summary>
    /// The companions and the prelude's declaration must not drift apart.
    /// </summary>
    /// <remarks>
    /// The patch NX evaluates carries the prelude's declared field under its declared name, so comparing what
    /// <c>diff</c> produces with what this type serializes fails as soon as the declaration renames a field.
    /// </remarks>
    [Fact]
    public void NxRangeUpdate_TracksThePreludeDeclaration()
    {
        JsonElement evaluated = NxRuntime.EvaluateJson(
            "let root() = { diff(1..5, 1..9) }",
            "ranges.nx");
        List<string> declared = evaluated
            .EnumerateObject()
            .Select(property => property.Name)
            .OrderBy(name => name, System.StringComparer.Ordinal)
            .ToList();

        using JsonDocument sdk = JsonDocument.Parse(
            JsonSerializer.Serialize(new NxRange_update<long> { End = 9 }));
        List<string> members = sdk.RootElement
            .EnumerateObject()
            .Select(property => property.Name)
            .OrderBy(name => name, System.StringComparer.Ordinal)
            .ToList();

        Assert.Equal(declared, members);
    }
}
