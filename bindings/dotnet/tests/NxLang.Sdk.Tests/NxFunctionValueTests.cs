// Copyright (c) The NX Authors.
// Licensed under the MIT license. See LICENSE file in the project root for full license information.

using System.Text.Json;
using System.Text.Json.Serialization;
using MessagePack;
using Xunit;

namespace NxLang.Nx.Tests;

/// <summary>
/// A rendered templated list, with its template read as a function reference.
/// </summary>
[MessagePackObject]
public sealed class TemplatedListElement
{
    [Key("ItemTemplate")]
    [JsonPropertyName("ItemTemplate")]
    public NxFunctionRef? ItemTemplate { get; set; }
}

/// <summary>
/// A record with a member of a function reference type, as a generated contract declares it.
/// </summary>
[MessagePackObject]
public sealed class ToolRecord
{
    [Key("fn")]
    [JsonPropertyName("fn")]
    public NxFunctionRef Fn { get; set; } = null!;

    [Key("fallback")]
    [JsonPropertyName("fallback")]
    public NxFunctionRef? Fallback { get; set; }
}

/// <summary>
/// Reading a function value from rendered output in .NET.
/// </summary>
/// <remarks>
/// A function value is a name, so a host can see which template it was handed and carry the record around; it cannot
/// call it. What these cases pin is that the member a generated contract declares — `NxFunctionRef`, what
/// `crates/nx-cli` emits for a function type — actually round-trips in both output formats the SDK supports.
/// </remarks>
public class NxFunctionValueTests
{
    private const string TemplateSource = """
        external component <List ItemTemplate?:<function Item:object Index:int />: string />
        let <Row Item:object Index:int />: string = "r"
        let root() = <List ItemTemplate={Row} />
        """;









    [Fact]
    public void FunctionRef_SerializesWhenTheMemberIsNull()
    {
        // MessagePack resolves a formatter for every member of the containing type, so an unserializable member
        // type breaks the whole contract even when nothing is bound to it. This is the case `System.Delegate`
        // failed.
        TemplatedListElement empty = new();

        byte[] messagePack = MessagePackSerializer.Serialize(
            empty,
            cancellationToken: TestContext.Current.CancellationToken);
        Assert.Null(
            MessagePackSerializer.Deserialize<TemplatedListElement>(
                messagePack,
                cancellationToken: TestContext.Current.CancellationToken).ItemTemplate);
        Assert.Null(JsonSerializer.Deserialize<TemplatedListElement>(JsonSerializer.Serialize(empty))!.ItemTemplate);
    }
}
