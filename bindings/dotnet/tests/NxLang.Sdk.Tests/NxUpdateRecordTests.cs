// Copyright (c) Bret Johnson. All rights reserved.
// Licensed under the MIT license. See LICENSE file in the project root for full license information.

using System;
using System.Collections.Generic;
using System.Linq;
using System.Text.Json;
using System.Text.Json.Serialization;
using MessagePack;
using NxLang.Nx;
using NxLang.Sdk.Tests.Generated;
using Xunit;

/// <summary>
/// The rendered <c>Button</c> a <c>Counter</c> renders, with its handler read as a dispatchable reference.
/// </summary>
[MessagePackObject]
public sealed class CounterButtonElement
{
    [Key("value")]
    [JsonPropertyName("value")]
    public int Value { get; set; }

    [Key("onTapped")]
    [JsonPropertyName("onTapped")]
    public NxActionHandlerRef OnTapped { get; set; } = new();
}

[MessagePackObject]
public sealed class ButtonTapped
{
    [Key("$type")]
    public string Type { get; set; } = "Button.Tapped";
}

[MessagePackObject]
public sealed class SavedAction
{
    [Key("$type")]
    public string Type { get; set; } = "Saved";
}

[MessagePackObject]
public sealed class EditorProps
{
    [Key("patch")]
    public User_update Patch { get; set; } = new();
}

/// <summary>
/// A hand-written patch that names a field <c>User</c> does not declare, standing in for a host DTO that has
/// drifted from the NX declaration.
/// </summary>
[MessagePackObject]
public sealed class DriftedUserPatch
{
    [Key("$type")]
    public string Type { get; set; } = "User.Update";

    [Key("nick")]
    public string Nick { get; set; } = "ada";
}

[MessagePackObject]
public sealed class DriftedEditorProps
{
    [Key("patch")]
    public DriftedUserPatch Patch { get; set; } = new();
}

/// <remarks>
/// <c>User_update</c> is the typegen output checked in under <c>Generated/</c>, so these tests exercise the DTO
/// shape typegen actually emits.
/// </remarks>
public class NxUpdateRecordTests
{
    private const string CounterSource = """
        external component <Button value:int = 0 emits { Tapped { } } />
        component <Counter /> = {
          state { count:int = 0 }
          <Button value={count} onTapped=<Update count={count + 1} /> />
        }
        """;

    [Fact]
    public void DispatchComponentActions_WithHandlerInvocation_PatchesStateAndReRenders()
    {
        NxComponentInitResult<CounterButtonElement> init =
            NxRuntime.InitializeComponent<CounterButtonElement>(CounterSource, "Counter");

        Assert.Equal(0, init.Rendered.Value);
        Assert.Equal("Button.Tapped", init.Rendered.OnTapped.Action);
        Assert.False(string.IsNullOrEmpty(init.Rendered.OnTapped.Token));

        NxComponentDispatchResult<CounterButtonElement, object> dispatched =
            NxRuntime.DispatchComponentActions<NxHandlerInvocation<ButtonTapped>[], CounterButtonElement, object>(
                CounterSource,
                init.StateSnapshot,
                new[] { init.Rendered.OnTapped.Invoke(new ButtonTapped()) });

        Assert.Equal(1, dispatched.Rendered.Value);
        Assert.Empty(dispatched.Effects);
        Assert.NotEmpty(dispatched.StateSnapshot);

        // The re-rendered handler carries the token for the next dispatch.
        NxComponentDispatchResult<CounterButtonElement, object> again =
            NxRuntime.DispatchComponentActions<NxHandlerInvocation<ButtonTapped>[], CounterButtonElement, object>(
                CounterSource,
                dispatched.StateSnapshot,
                new[] { dispatched.Rendered.OnTapped.Invoke(new ButtonTapped()) });
        Assert.Equal(2, again.Rendered.Value);
    }

    [Fact]
    public void DispatchComponentActionsJson_WithHandlerInvocation_ReturnsRenderedEffectsAndSnapshot()
    {
        NxComponentInitResult<CounterButtonElement> init =
            NxRuntime.InitializeComponent<CounterButtonElement>(CounterSource, "Counter");

        NxComponentDispatchResult<JsonElement, JsonElement> dispatched =
            NxRuntime.DispatchComponentActionsJson(
                CounterSource,
                init.StateSnapshot,
                new[] { init.Rendered.OnTapped.Invoke(new ButtonTapped()) });

        Assert.Equal(1, dispatched.Rendered.GetProperty("value").GetInt32());
        Assert.Equal(
            "ActionHandler",
            dispatched.Rendered.GetProperty("onTapped").GetProperty("$type").GetString());
        Assert.Empty(dispatched.Effects);
        Assert.NotEmpty(dispatched.StateSnapshot);
    }

    [Fact]
    public void DispatchComponentActions_MixedBatch_RunsInvocationsAndActionsTogether()
    {
        string source = """
            action Saved = { }
            external component <Button value:int = 0 emits { Tapped { } } />
            component <Counter emits { Saved } /> = {
              state { count:int = 0 }
              <Button value={count} onTapped=<Update count={count + 1} /> />
            }
            """;
        NxComponentInitResult<CounterButtonElement> init =
            NxRuntime.InitializeComponent<CounterButtonElement>(source, "Counter");

        // No parent bound `onSaved`, so the emitted action runs nothing; the batch still succeeds.
        NxComponentDispatchResult<CounterButtonElement, object> dispatched =
            NxRuntime.DispatchComponentActions<object[], CounterButtonElement, object>(
                source,
                init.StateSnapshot,
                new object[] { init.Rendered.OnTapped.Invoke(new ButtonTapped()), new SavedAction() });

        Assert.Equal(1, dispatched.Rendered.Value);
        Assert.Empty(dispatched.Effects);
    }

    [Fact]
    public void DispatchComponentActions_WithStaleToken_ThrowsWithoutAResult()
    {
        NxComponentInitResult<CounterButtonElement> init =
            NxRuntime.InitializeComponent<CounterButtonElement>(CounterSource, "Counter");
        NxHandlerInvocation<ButtonTapped> stale = init.Rendered.OnTapped.Invoke(new ButtonTapped());
        NxComponentDispatchResult<CounterButtonElement, object> dispatched =
            NxRuntime.DispatchComponentActions<NxHandlerInvocation<ButtonTapped>[], CounterButtonElement, object>(
                CounterSource,
                init.StateSnapshot,
                new[] { stale });

        NxEvaluationException error = Assert.Throws<NxEvaluationException>(
            () => NxRuntime.DispatchComponentActions<NxHandlerInvocation<ButtonTapped>[], CounterButtonElement, object>(
                CounterSource,
                dispatched.StateSnapshot,
                new[] { stale }));

        Assert.Contains(
            error.Diagnostics,
            diagnostic => diagnostic.Message.Contains(stale.Token, StringComparison.Ordinal));
    }

    [Fact]
    public void EvaluateComponent_HandlerReference_HasNoToken()
    {
        CounterButtonElement rendered =
            NxRuntime.EvaluateComponent<Dictionary<string, object>, CounterState, CounterButtonElement>(
                CounterSource,
                "Counter",
                new Dictionary<string, object>(),
                new CounterState { Count = 3 });

        Assert.Equal(3, rendered.Value);
        Assert.Equal("Button.Tapped", rendered.OnTapped.Action);
        Assert.Null(rendered.OnTapped.Token);
        Assert.Throws<InvalidOperationException>(() => rendered.OnTapped.Invoke(new ButtonTapped()));
    }

    [Fact]
    public void UpdateRecord_Json_OmitsUnsetAndKeepsNull()
    {
        User_update update = new() { Email = null };

        string json = JsonSerializer.Serialize(update);
        using JsonDocument document = JsonDocument.Parse(json);
        string[] keys = document.RootElement.EnumerateObject().Select(property => property.Name).ToArray();
        Assert.Equal(new[] { "$type", "email" }, keys);
        Assert.Equal("User.Update", document.RootElement.GetProperty("$type").GetString());
        Assert.Equal(JsonValueKind.Null, document.RootElement.GetProperty("email").ValueKind);

        User_update read = JsonSerializer.Deserialize<User_update>(json)!;
        Assert.False(read.Name.HasValue);
        Assert.True(read.Email.HasValue);
        Assert.Null(read.Email.Value);
    }

    [Fact]
    public void UpdateRecord_MessagePack_OmitsUnsetAndKeepsNull()
    {
        User_update update = new() { Email = null };

        byte[] bytes = MessagePackSerializer.Serialize(
            update,
            cancellationToken: TestContext.Current.CancellationToken);
        string json = MessagePackSerializer.ConvertToJson(
            bytes,
            cancellationToken: TestContext.Current.CancellationToken);
        using JsonDocument document = JsonDocument.Parse(json);
        string[] keys = document.RootElement.EnumerateObject().Select(property => property.Name).ToArray();
        Assert.Equal(new[] { "$type", "email" }, keys);

        User_update read = MessagePackSerializer.Deserialize<User_update>(
            bytes,
            cancellationToken: TestContext.Current.CancellationToken);
        Assert.False(read.Name.HasValue);
        Assert.True(read.Email.HasValue);
        Assert.Null(read.Email.Value);
    }

    [Fact]
    public void UpdateTypedFields_GenerateAsTheCompanionAndRoundTrip()
    {
        Form form = new() { Drafts = new[] { new User_update { Name = "Ada" } } };

        string json = JsonSerializer.Serialize(form);
        using JsonDocument document = JsonDocument.Parse(json);
        Assert.Equal(JsonValueKind.Null, document.RootElement.GetProperty("pending").ValueKind);
        JsonElement draft = document.RootElement.GetProperty("drafts")[0];
        string[] keys = draft.EnumerateObject().Select(property => property.Name).ToArray();
        Assert.Equal(new[] { "$type", "name" }, keys);

        Form read = MessagePackSerializer.Deserialize<Form>(
            MessagePackSerializer.Serialize(form, cancellationToken: TestContext.Current.CancellationToken),
            cancellationToken: TestContext.Current.CancellationToken);
        Assert.Null(read.Pending);
        Assert.Single(read.Drafts);
        Assert.Equal("Ada", read.Drafts[0].Name.Value);
        Assert.False(read.Drafts[0].Email.HasValue);
    }

    [Fact]
    public void HandlerInvocation_Json_UsesTheCanonicalKeys()
    {
        NxHandlerInvocation<ButtonTapped> invocation = new("h1-1", new ButtonTapped());

        string json = JsonSerializer.Serialize(invocation);
        using JsonDocument document = JsonDocument.Parse(json);
        string[] keys = document.RootElement.EnumerateObject().Select(property => property.Name).ToArray();
        Assert.Equal(new[] { "$type", "token", "action" }, keys);
        Assert.Equal("ActionHandlerInvocation", document.RootElement.GetProperty("$type").GetString());
        Assert.Equal("h1-1", document.RootElement.GetProperty("token").GetString());
    }

    [Fact]
    public void UnsetOptional_OutsideAnUpdateRecord_ThrowsInsteadOfWritingNull()
    {
        NxOptional<string>[] values = { NxOptional<string>.Unset };

        Assert.Throws<InvalidOperationException>(() => JsonSerializer.Serialize(values));
        MessagePackSerializationException error = Assert.Throws<MessagePackSerializationException>(
            () => MessagePackSerializer.Serialize(values, cancellationToken: TestContext.Current.CancellationToken));
        Assert.IsType<InvalidOperationException>(error.InnerException);
    }

    [Fact]
    public void RawEffect_CarryingAnUpdateRecord_PreservesAbsence()
    {
        string source = """
            type User = { name:string email:string? }
            action Apply = { patch:User.Update }
            external component <Button value:int = 0 emits { Tapped { } } />
            component <Form emits { Apply } /> = {
              state { count:int = 0 }
              <Button value={count} onTapped=<Apply patch=<User.Update name="Ada" /> /> />
            }
            """;

        NxComponentInitResult<CounterButtonElement> init =
            NxRuntime.InitializeComponent<CounterButtonElement>(source, "Form");

        NxComponentDispatchResult<JsonElement, JsonElement> dispatched =
            NxRuntime.DispatchComponentActionsJson(
                source,
                init.StateSnapshot,
                new[] { init.Rendered.OnTapped.Invoke(new ButtonTapped()) });

        JsonElement effect = Assert.Single(dispatched.Effects);
        Assert.Equal("Apply", effect.GetProperty("$type").GetString());
        JsonElement patch = effect.GetProperty("patch");
        string[] keys = patch.EnumerateObject().Select(property => property.Name).ToArray();
        Assert.Equal(new[] { "$type", "name" }, keys);
        Assert.Equal("User.Update", patch.GetProperty("$type").GetString());
    }

    [Fact]
    public void UpdateRecord_PassedAsAProp_DecodesNativelyWithOnlyThePresentField()
    {
        string source = """
            type User = { name:string email:string? }
            component <Editor patch:User.Update /> = { <Panel patch={patch} /> }
            """;

        NxComponentInitResult<JsonElement> init = NxRuntime.InitializeComponentJson(
            source,
            "Editor",
            new EditorProps { Patch = new User_update { Name = "Ada" } });

        JsonElement patch = init.Rendered.GetProperty("patch");
        string[] keys = patch.EnumerateObject().Select(property => property.Name).ToArray();
        Assert.Equal(new[] { "$type", "name" }, keys);
        Assert.Equal("User.Update", patch.GetProperty("$type").GetString());
        Assert.Equal("Ada", patch.GetProperty("name").GetString());
    }

    [Fact]
    public void UpdateRecord_PassedAsAProp_WithAnUnknownField_IsRejectedAtInitialization()
    {
        string source = """
            type User = { name:string email:string? }
            component <Editor patch:User.Update /> = { <Panel patch={patch} /> }
            """;

        NxEvaluationException error = Assert.Throws<NxEvaluationException>(
            () => NxRuntime.InitializeComponentJson(source, "Editor", new DriftedEditorProps()));

        Assert.Contains(
            error.Diagnostics,
            diagnostic => diagnostic.Message.Contains("nick", StringComparison.Ordinal));
    }
}

[MessagePackObject]
public sealed class CounterState
{
    [Key("count")]
    public int Count { get; set; }
}
