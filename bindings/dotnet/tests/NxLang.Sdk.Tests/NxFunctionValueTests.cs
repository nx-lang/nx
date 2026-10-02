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
    public void Evaluate_WithAFunctionValuedProp_ReadsTheDeclarationItNames()
    {
        TemplatedListElement rendered =
            NxRuntime.Evaluate<TemplatedListElement>(TemplateSource, "templates.nx");

        Assert.NotNull(rendered.ItemTemplate);
        Assert.Equal("templates.nx", rendered.ItemTemplate!.Module);
        Assert.Equal("Row", rendered.ItemTemplate.Name);
    }

    [Fact]
    public void EvaluateJson_WithAFunctionValuedProp_RendersTheFunctionRecord()
    {
        JsonElement rendered = NxRuntime.EvaluateJson(TemplateSource, "templates.nx");
        JsonElement template = rendered.GetProperty("ItemTemplate");

        Assert.Equal("Function", template.GetProperty("$type").GetString());
        Assert.Equal("templates.nx", template.GetProperty("module").GetString());
        Assert.Equal("Row", template.GetProperty("name").GetString());
    }

    [Fact]
    public void FunctionRef_RoundTripsThroughBothOutputFormats()
    {
        TemplatedListElement rendered =
            NxRuntime.Evaluate<TemplatedListElement>(TemplateSource, "templates.nx");

        byte[] messagePack = MessagePackSerializer.Serialize(
            rendered,
            cancellationToken: TestContext.Current.CancellationToken);
        TemplatedListElement fromMessagePack = MessagePackSerializer.Deserialize<TemplatedListElement>(
            messagePack,
            cancellationToken: TestContext.Current.CancellationToken);
        Assert.Equal("templates.nx", fromMessagePack.ItemTemplate!.Module);
        Assert.Equal("Row", fromMessagePack.ItemTemplate.Name);

        string json = JsonSerializer.Serialize(rendered);
        TemplatedListElement fromJson = JsonSerializer.Deserialize<TemplatedListElement>(json)!;
        Assert.Equal("templates.nx", fromJson.ItemTemplate!.Module);
        Assert.Equal("Row", fromJson.ItemTemplate.Name);
    }

    [Fact]
    public void FunctionRef_AtAFunctionReferenceMember_RoundTripsThroughBothOutputFormats()
    {
        // A member typed `<function ... />: R` takes a function of any parameters, and what a host reads there is
        // the same record a function-typed member holds.
        const string source = """
            type Tool = { fn: <function ... />: object* fallback?: <function ... />: int }
            let double(n:int): int = {n * 2}
            let root() = <Tool fn={double} />
            """;

        ToolRecord rendered = NxRuntime.Evaluate<ToolRecord>(source, "main.nx");
        Assert.Equal("main.nx", rendered.Fn.Module);
        Assert.Equal("double", rendered.Fn.Name);
        Assert.Null(rendered.Fallback);

        byte[] messagePack = MessagePackSerializer.Serialize(
            rendered,
            cancellationToken: TestContext.Current.CancellationToken);
        ToolRecord fromMessagePack = MessagePackSerializer.Deserialize<ToolRecord>(
            messagePack,
            cancellationToken: TestContext.Current.CancellationToken);
        Assert.Equal("main.nx", fromMessagePack.Fn.Module);
        Assert.Equal("double", fromMessagePack.Fn.Name);

        ToolRecord fromJson = JsonSerializer.Deserialize<ToolRecord>(JsonSerializer.Serialize(rendered))!;
        Assert.Equal("main.nx", fromJson.Fn.Module);
        Assert.Equal("double", fromJson.Fn.Name);
    }

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
