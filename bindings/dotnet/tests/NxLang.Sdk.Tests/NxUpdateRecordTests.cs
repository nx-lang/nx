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

/// <summary>
/// A hand-written patch that clears <c>name</c>, which <c>User</c> declares required: the payload the generated
/// companion's non-nullable <c>Name</c> accessor keeps a C# caller from building.
/// </summary>
[MessagePackObject]
public sealed class ClearingUserPatch
{
    [Key("$type")]
    public string Type { get; set; } = "User.Update";

    [Key("name")]
    public string? Name { get; set; }
}

[MessagePackObject]
public sealed class ClearingEditorProps
{
    [Key("patch")]
    public ClearingUserPatch Patch { get; set; } = new();
}

/// <summary>
/// A hand-written patch that clears a component's <c>count</c>, a value-typed field the schema knows cannot be
/// cleared.
/// </summary>
[MessagePackObject]
public sealed class ClearingCounterPatch
{
    [Key("$type")]
    public string Type { get; set; } = "Counter.Update";

    [Key("count")]
    public long? Count { get; set; }
}

[MessagePackObject]
public sealed class BookEditorProps
{
    [Key("patch")]
    public Book_update Patch { get; set; } = new();
}

/// <summary>
/// A hand-written <c>Person</c> that carries its <c>$type</c>, which a record the host sends to a record-typed
/// site needs and the generated <c>Person</c> does not write.
/// </summary>
[MessagePackObject]
public sealed class PersonRecord
{
    [Key("$type")]
    public string Type { get; set; } = "Person";

    [Key("name")]
    public string Name { get; set; } = "X";
}

/// <summary>
/// A hand-written <c>Book</c> that carries its <c>$type</c> and writes its optional fields as they are set: a
/// <c>null</c> is nil on the wire, an empty array is an empty array.
/// </summary>
[MessagePackObject]
public sealed class BookRecord
{
    [Key("$type")]
    public string Type { get; set; } = "Book";

    [Key("title")]
    public string Title { get; set; } = "A";

    [Key("author")]
    public PersonRecord? Author { get; set; }

    [Key("tags")]
    public string[]? Tags { get; set; }

    [Key("authors")]
    public PersonRecord[] Authors { get; set; } = new[] { new PersonRecord() };
}

[MessagePackObject]
public sealed class BookViewerProps
{
    [Key("book")]
    public BookRecord Book { get; set; } = new();
}

/// <summary>
/// A hand-written <c>Book</c> with no <c>tags</c> or <c>author</c> key at all.
/// </summary>
[MessagePackObject]
public sealed class BookWithoutOptionalKeys
{
    [Key("$type")]
    public string Type { get; set; } = "Book";

    [Key("title")]
    public string Title { get; set; } = "A";

    [Key("authors")]
    public PersonRecord[] Authors { get; set; } = new[] { new PersonRecord() };
}

[MessagePackObject]
public sealed class BareBookViewerProps
{
    [Key("book")]
    public BookWithoutOptionalKeys Book { get; set; } = new();
}

/// <summary>
/// The rendered <c>Panel</c> a <c>BookEditor</c> renders, with its patch read back through the generated
/// companion.
/// </summary>
[MessagePackObject]
public sealed class BookPanelElement
{
    [Key("patch")]
    public Book_update Patch { get; set; } = new();
}

/// <summary>
/// A hand-written patch whose keys fit <c>User</c> but whose discriminator names another record.
/// </summary>
[MessagePackObject]
public sealed class MislabeledUserPatch
{
    [Key("$type")]
    public string Type { get; set; } = "Form.Update";

    [Key("name")]
    public string Name { get; set; } = "Ada";
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
    public void UpdateRecord_Json_OmitsUnsetAndWritesClearedAsNull()
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
    public void UpdateRecord_Json_WritesTypeThenFieldsInOrdinalKeyOrder()
    {
        User_update update = new() { Name = "Ada", Email = "x@y" };

        string json = JsonSerializer.Serialize(update);
        using JsonDocument document = JsonDocument.Parse(json);
        string[] keys = document.RootElement.EnumerateObject().Select(property => property.Name).ToArray();
        Assert.Equal(new[] { "$type", "email", "name" }, keys);
    }

    /// <summary>
    /// The bytes are those the reflection-based formatter wrote before the map-backed storage landed, captured
    /// from it for the same patches: <c>$type</c> first, then the set fields in ordinal key order.
    /// </summary>
    [Fact]
    public void UpdateRecord_MessagePack_WritesTypeThenFieldsInOrdinalKeyOrder()
    {
        User_update user = new() { Name = "Ada", Email = "x@y" };
        Form_update form = new()
        {
            Pending = null,
            Drafts = new[] { new User_update { Name = "Ada" } },
            SortBy = User_property.Email,
        };

        byte[] userBytes = MessagePackSerializer.Serialize(user, cancellationToken: TestContext.Current.CancellationToken);
        byte[] formBytes = MessagePackSerializer.Serialize(form, cancellationToken: TestContext.Current.CancellationToken);

        Assert.Equal(
            "83A52474797065AB557365722E557064617465A5656D61696CA3784079A46E616D65A3416461",
            Convert.ToHexString(userBytes));
        Assert.Equal(
            "84A52474797065AB466F726D2E557064617465A66472616674739182A52474797065AB557365722E557064617465"
            + "A46E616D65A3416461A770656E64696E67C0A6736F72744279A5656D61696C",
            Convert.ToHexString(formBytes));
        using JsonDocument document = JsonDocument.Parse(
            MessagePackSerializer.ConvertToJson(formBytes, cancellationToken: TestContext.Current.CancellationToken));
        string[] keys = document.RootElement.EnumerateObject().Select(property => property.Name).ToArray();
        Assert.Equal(new[] { "$type", "drafts", "pending", "sortBy" }, keys);
    }

    [Fact]
    public void UpdateRecord_MessagePack_OmitsUnsetAndWritesClearedAsNil()
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
    public void RawEffect_CarryingAnUpdateRecord_PreservesAbsence()
    {
        string source = """
            type User = { name:string email?:string }
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
            type User = { name:string email?:string }
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
            type User = { name:string email?:string }
            component <Editor patch:User.Update /> = { <Panel patch={patch} /> }
            """;

        NxEvaluationException error = Assert.Throws<NxEvaluationException>(
            () => NxRuntime.InitializeComponentJson(source, "Editor", new DriftedEditorProps()));

        Assert.Contains(
            error.Diagnostics,
            diagnostic => diagnostic.Message.Contains("nick", StringComparison.Ordinal));
    }

    /// <summary>
    /// A handler bound at the root belongs to no component, so a bare update record it returns is the host's to
    /// apply: it arrives as an effect with only the discriminator and the fields that were set.
    /// </summary>
    [Fact]
    public void BareUpdateRecord_FromARootBoundHandler_ReachesTheHostAsAnEffect()
    {
        string source = """
            type User = { name:string email?:string }
            external component <Button emits { Tapped { } } />
            let saveButton() = <Button onTapped=<User.Update name="Ada" /> />
            component <Form /> = { <Panel>{saveButton()}</Panel> }
            """;

        NxComponentInitResult<JsonElement> init = NxRuntime.InitializeComponentJson(source, "Form");
        string token = Assert.Single(HandlerTokens(init.Rendered));

        NxComponentDispatchResult<JsonElement, JsonElement> dispatched =
            NxRuntime.DispatchComponentActionsJson(
                source,
                init.StateSnapshot,
                new[] { new NxHandlerInvocation<ButtonTapped>(token, new ButtonTapped()) });

        JsonElement effect = Assert.Single(dispatched.Effects);
        string[] keys = effect.EnumerateObject().Select(property => property.Name).ToArray();
        Assert.Equal(new[] { "$type", "name" }, keys);
        Assert.Equal("User.Update", effect.GetProperty("$type").GetString());
        Assert.Equal("Ada", effect.GetProperty("name").GetString());
    }

    [Fact]
    public void UpdateRecord_Fields_EnumerateOnlyTheSetFields()
    {
        User_update update = new() { Name = "Ada" };

        KeyValuePair<string, object?> field = Assert.Single(update.Fields);
        Assert.Equal("name", field.Key);
        Assert.Equal("Ada", field.Value);
        Assert.True(update.IsSet("name"));
        Assert.False(update.IsSet("email"));
        Assert.False(update.IsSet(User_property.Email));
    }

    [Fact]
    public void UpdateRecord_ClearedField_IsCarried()
    {
        User_update update = new() { Email = null };

        KeyValuePair<string, object?> field = Assert.Single(update.Fields);
        Assert.Equal("email", field.Key);
        Assert.Null(field.Value);
        Assert.True(update.IsSet("email"));
        Assert.True(update.IsSet(User_property.Email));
    }

    [Fact]
    public void UpdateRecord_Unset_RemovesTheFieldInMemoryAndOnTheWire()
    {
        User_update update = new() { Name = "Ada", Email = null };

        update.Unset("name");

        Assert.False(update.Fields.ContainsKey("name"));
        Assert.False(update.Name.HasValue);
        string json = JsonSerializer.Serialize(update);
        using JsonDocument document = JsonDocument.Parse(json);
        string[] keys = document.RootElement.EnumerateObject().Select(property => property.Name).ToArray();
        Assert.Equal(new[] { "$type", "email" }, keys);

        update.Unset(User_property.Email);
        Assert.Empty(update.Fields);
        Assert.Equal("{\"$type\":\"User.Update\"}", JsonSerializer.Serialize(update));
    }

    [Fact]
    public void UpdateRecord_Json_WithAnUnknownKey_ThrowsNamingTheKey()
    {
        const string json = """{"$type":"User.Update","nick":"ada"}""";

        JsonException error = Assert.Throws<JsonException>(() => JsonSerializer.Deserialize<User_update>(json));

        Assert.Contains("nick", error.Message, StringComparison.Ordinal);
        Assert.Contains(nameof(User_update), error.Message, StringComparison.Ordinal);
    }

    [Fact]
    public void UpdateRecord_MessagePack_WithAnUnknownKey_ThrowsNamingTheKey()
    {
        byte[] bytes = MessagePackSerializer.Serialize(
            new DriftedUserPatch(),
            cancellationToken: TestContext.Current.CancellationToken);

        MessagePackSerializationException error = Assert.Throws<MessagePackSerializationException>(
            () => MessagePackSerializer.Deserialize<User_update>(
                bytes,
                cancellationToken: TestContext.Current.CancellationToken));

        Assert.Contains("nick", error.ToString(), StringComparison.Ordinal);
        Assert.Contains(nameof(User_update), error.ToString(), StringComparison.Ordinal);
    }

    [Fact]
    public void UpdateRecord_Json_WithAnotherRecordsDiscriminator_Throws()
    {
        const string json = """{"$type":"Form.Update","name":"Ada"}""";

        JsonException error = Assert.Throws<JsonException>(() => JsonSerializer.Deserialize<User_update>(json));

        Assert.Contains("Form.Update", error.Message, StringComparison.Ordinal);
        Assert.Contains("User.Update", error.Message, StringComparison.Ordinal);
    }

    [Fact]
    public void UpdateRecord_MessagePack_WithAnotherRecordsDiscriminator_Throws()
    {
        byte[] bytes = MessagePackSerializer.Serialize(
            new MislabeledUserPatch(),
            cancellationToken: TestContext.Current.CancellationToken);

        MessagePackSerializationException error = Assert.Throws<MessagePackSerializationException>(
            () => MessagePackSerializer.Deserialize<User_update>(
                bytes,
                cancellationToken: TestContext.Current.CancellationToken));

        Assert.Contains("Form.Update", error.ToString(), StringComparison.Ordinal);
        Assert.Contains("User.Update", error.ToString(), StringComparison.Ordinal);
    }

    /// <summary>
    /// A field named after a companion or base-class member keeps its accessor name, and the companion's own
    /// members step around it, as the discriminator already did; the base surface stays reachable through the
    /// base type.
    /// </summary>
    [Fact]
    public void UpdateRecord_WithFieldsNamedAfterItsMembers_KeepsBothReachable()
    {
        Clash_update update = new() { Changed = true, Fields = "f", Diff = "d" };

        Assert.Equal("Clash.Update", update.NxType_);
        Assert.Equal(new[] { Clash_property.Changed, Clash_property.Diff, Clash_property.Fields }, update.Changed_());
        Assert.True(update.IsSet_(Clash_property.Fields));
        Assert.Equal("f", update.Fields.Value);
        NxUpdateRecord asBase = update;
        Assert.Equal(3, asBase.Fields.Count);
        Assert.Equal("Clash.Update", asBase.Schema.NxType);

        update.Unset_(Clash_property.Changed);
        Assert.False(update.Changed.HasValue);

        Clash before = new() { Diff = "a" };
        Clash after = new() { Diff = "b" };
        Assert.Equal(new[] { "diff" }, Clash_update.Diff_(before, after).ChangedNames());
        Assert.Equal("d", update.Apply(after).Diff);
    }

    [Fact]
    public void Diff_ReportsAPolymorphicFieldThatChangesType()
    {
        Doc circle = new() { Shape = new Circle { Id = "1" } };
        Doc square = new() { Shape = new Square { Id = "1" } };

        Assert.Equal(new[] { "shape" }, Doc_update.Diff(circle, square).ChangedNames());
        Assert.IsType<Square>(Doc_update.Diff(circle, square).Shape.Value);
        Assert.Empty(Doc_update.Diff(circle, new Doc { Shape = new Circle { Id = "1" } }).Fields);
    }

    /// <summary>
    /// An external component's state does have a plain generated type, <c>Ticker_state</c>, so its companion
    /// applies to that rather than to the props contract emitted under the component's own name.
    /// </summary>
    [Fact]
    public void ExternalComponentUpdate_AppliesToTheStateRecord()
    {
        Ticker_state state = new() { Count = 1 };

        Ticker_state applied = new Ticker_update { Count = 2 }.Apply(state);

        Assert.Equal(2, applied.Count);
        Assert.Equal(new[] { Ticker_property.Count }, Ticker_update.Diff(state, applied).Changed());
        Assert.Same(Ticker_stateProperties.Count, Ticker_stateProperties.Of(Ticker_property.Count));
        Assert.Equal(typeof(NxUpdate<Ticker_state>), typeof(Ticker_update).BaseType);
    }

    [Fact]
    public void Apply_OverwritesOnlyTheCarriedFields()
    {
        User user = new() { Name = "Ada", Email = "x@y" };
        User_update update = new() { Email = null };

        User applied = update.Apply(user);

        Assert.Equal("Ada", applied.Name);
        Assert.Null(applied.Email);
        Assert.Equal("x@y", user.Email);
    }

    [Fact]
    public void Merge_LetsTheLaterPatchWin()
    {
        User_update merged = NxUpdateRecord.Merge(
            new User_update { Name = "Ada" },
            new User_update { Name = "Grace", Email = null });

        Assert.Equal("Grace", merged.Name.Value);
        Assert.True(merged.Email.HasValue);
        Assert.Null(merged.Email.Value);

        // The TypeScript runtime's own case: a later cleared field replaces an earlier value.
        User_update runtimeCase = User_update.Merge(
            new User_update { Name = "Ada", Email = "x@y" },
            new User_update { Email = null });
        Assert.Equal("Ada", runtimeCase.Name.Value);
        Assert.Null(runtimeCase.Email.Value);
    }

    [Fact]
    public void Diff_CarriesOnlyTheDifferingFields()
    {
        User user = new() { Name = "Ada", Email = "x@y" };
        User partial = new() { Name = "Ada" };

        User_update forward = User_update.Diff(user, partial);
        KeyValuePair<string, object?> field = Assert.Single(forward.Fields);
        Assert.Equal("email", field.Key);
        Assert.Null(field.Value);

        User_update backward = User_update.Diff(partial, user);
        Assert.Equal(new[] { "email" }, backward.ChangedNames());
        Assert.Equal("x@y", backward.Email.Value);

        Assert.Empty(User_update.Diff(user, new User { Name = "Ada", Email = "x@y" }).Fields);
    }

    /// <summary>
    /// Arrays compare element-wise and nested patches by the fields they carry, as <c>nxValuesEqual</c> does, so
    /// two records built separately from the same values diff to nothing.
    /// </summary>
    [Fact]
    public void Diff_ComparesArraysAndNestedRecordsStructurally()
    {
        static Form MakeForm(NxOptional<string?> pendingEmail) => new()
        {
            Pending = new User_update { Email = pendingEmail },
            Drafts = new[] { new User_update { Name = "Ada" } },
            SortBy = User_property.Name,
        };

        Assert.Empty(Form_update.Diff(MakeForm(null), MakeForm(null)).Fields);

        // An unset and a cleared field are different patches, so the nested record differs.
        Form_update pending = Form_update.Diff(MakeForm(null), MakeForm(NxOptional<string?>.Unset));
        Assert.Equal(new[] { "pending" }, pending.ChangedNames());

        Form longer = MakeForm(null);
        longer.Drafts = new[] { new User_update { Name = "Ada" }, new User_update() };
        Form_update drafts = Form_update.Diff(MakeForm(null), longer);
        Assert.Equal(new[] { "drafts" }, drafts.ChangedNames());
        Assert.Equal(2, drafts.Drafts.Value.Length);
    }

    [Fact]
    public void Changed_ReportsTheCarriedFieldsInDeclaredOrder()
    {
        User_update update = new() { Email = null, Name = "Ada" };

        Assert.Equal(new[] { User_property.Name, User_property.Email }, update.Changed());
        Assert.Equal(new[] { "name", "email" }, update.ChangedNames());
        Assert.Empty(new User_update().Changed());
    }

    /// <summary>
    /// A component's state has no plain generated type, so its companion derives from the untyped base; merge
    /// and changed still work there, and the patch still round-trips.
    /// </summary>
    [Fact]
    public void ComponentUpdate_WithNoPlainType_MergesAndReportsChanges()
    {
        Counter_update merged = NxUpdateRecord.Merge(
            new Counter_update { Count = 1 },
            new Counter_update { Count = 2 });

        Assert.Equal(typeof(NxUpdateRecord), typeof(Counter_update).BaseType);
        Assert.Equal(2, merged.Count.Value);
        Assert.Equal(new[] { Counter_property.Count }, merged.Changed());
        Assert.True(merged.IsSet(Counter_property.Count));

        string json = JsonSerializer.Serialize(merged);
        Assert.Equal("{\"$type\":\"Counter.Update\",\"count\":2}", json);
        Counter_update read = MessagePackSerializer.Deserialize<Counter_update>(
            MessagePackSerializer.Serialize(merged, cancellationToken: TestContext.Current.CancellationToken),
            cancellationToken: TestContext.Current.CancellationToken);
        Assert.Equal(2, read.Count.Value);
    }

    private const string BookSource = """
        type Person = { name:string }
        type Book = { title:string author?:Person tags?:string+ authors:Person+ }
        component <BookEditor patch:Book.Update /> = { <Panel patch={patch} /> }
        component <BookViewer book:Book /> = { <Panel book={book} /> }
        """;

    /// <summary>
    /// <c>null</c> is the .NET spelling of a cleared field: it crosses the boundary as <c>null</c>, the NX runtime
    /// reads it as the empty value for the optional <c>author</c>, and it comes back as a carried <c>null</c>.
    /// </summary>
    [Fact]
    public void ClearedOptionalField_RoundTripsAsNullThroughTheRuntime()
    {
        BookEditorProps props = new() { Patch = new Book_update { Author = null } };

        NxComponentInitResult<JsonElement> asJson =
            NxRuntime.InitializeComponentJson(BookSource, "BookEditor", props);
        JsonElement patch = asJson.Rendered.GetProperty("patch");
        string[] keys = patch.EnumerateObject().Select(property => property.Name).ToArray();
        Assert.Equal(new[] { "$type", "author" }, keys);
        Assert.Equal("Book.Update", patch.GetProperty("$type").GetString());
        Assert.Equal(JsonValueKind.Null, patch.GetProperty("author").ValueKind);

        NxComponentInitResult<BookPanelElement> typed =
            NxRuntime.InitializeComponent<BookEditorProps, BookPanelElement>(BookSource, "BookEditor", props);
        Book_update read = typed.Rendered.Patch;
        Assert.Equal(new[] { Book_property.Author }, read.Changed());
        Assert.True(read.Author.HasValue);
        Assert.Null(read.Author.Value);
        Assert.False(read.Title.HasValue);
    }

    /// <summary>
    /// The generated companion types <c>Name</c> as <c>NxOptional&lt;string&gt;</c>, so clearing it is a nullability
    /// diagnostic in C#; a payload that clears it anyway is refused when the runtime constructs the
    /// <c>User.Update</c>, naming the field, while clearing the optional <c>email</c> is accepted.
    /// </summary>
    [Fact]
    public void ClearingANonClearableField_IsRejectedByTheRuntimeNamingIt()
    {
        string source = """
            type User = { name:string email?:string }
            component <Editor patch:User.Update /> = { <Panel patch={patch} /> }
            """;

        NxEvaluationException error = Assert.Throws<NxEvaluationException>(
            () => NxRuntime.InitializeComponentJson(source, "Editor", new ClearingEditorProps()));
        Assert.Contains("name", DiagnosticText(error), StringComparison.Ordinal);

        NxComponentInitResult<JsonElement> cleared = NxRuntime.InitializeComponentJson(
            source,
            "Editor",
            new EditorProps { Patch = new User_update { Email = null } });
        JsonElement patch = cleared.Rendered.GetProperty("patch");
        Assert.Equal(new[] { "$type", "email" }, patch.EnumerateObject().Select(property => property.Name).ToArray());
        Assert.Equal(JsonValueKind.Null, patch.GetProperty("email").ValueKind);
    }

    /// <summary>
    /// A <c>+</c> field is a plain array in C#, so an empty one compiles; the boundary is where <c>+</c> is
    /// enforced.
    /// </summary>
    [Fact]
    public void EmptyArrayAtAOneOrMoreSite_IsRejectedByTheRuntimeNamingIt()
    {
        BookViewerProps props = new()
        {
            Book = new BookRecord { Authors = Array.Empty<PersonRecord>() },
        };

        NxEvaluationException error = Assert.Throws<NxEvaluationException>(
            () => NxRuntime.InitializeComponentJson(BookSource, "BookViewer", props));

        Assert.Contains("authors", DiagnosticText(error), StringComparison.Ordinal);
    }

    [Fact]
    public void NullAnEmptyArrayAndAMissingKey_AllDecodeAsTheEmptyValue()
    {
        object[] inputs =
        {
            new BookViewerProps { Book = new BookRecord { Tags = null, Author = null } },
            new BookViewerProps { Book = new BookRecord { Tags = Array.Empty<string>() } },
            new BareBookViewerProps(),
        };

        foreach (object props in inputs)
        {
            NxComponentInitResult<JsonElement> init;
            try
            {
                init = NxRuntime.InitializeComponentJson(BookSource, "BookViewer", props);
            }
            catch (NxEvaluationException error)
            {
                Assert.Fail($"{props.GetType().Name}: {DiagnosticText(error)}");
                return;
            }

            JsonElement book = init.Rendered.GetProperty("book");
            string[] keys = book.EnumerateObject().Select(property => property.Name).ToArray();
            Assert.Equal(new[] { "$type", "authors", "title" }, keys);
        }
    }

    /// <summary>
    /// A decoded optional sequence may read as <c>null</c> or as an empty array depending on where it came from;
    /// <c>diff</c> sees one empty value either way, and carries a field that becomes empty as cleared.
    /// </summary>
    [Fact]
    public void Diff_ReadsNullAndAnEmptyArrayAsOneEmptyValue()
    {
        Person[] authors = new[] { new Person { Name = "X" } };
        Book nullTags = new() { Title = "A", Authors = authors, Tags = null };
        Book emptyTags = new() { Title = "A", Authors = authors, Tags = Array.Empty<string>() };
        Book tagged = new() { Title = "A", Authors = authors, Tags = new[] { "t" } };

        Assert.Empty(Book_update.Diff(nullTags, emptyTags).Fields);
        Assert.Empty(Book_update.Diff(emptyTags, nullTags).Fields);

        Book_update cleared = Book_update.Diff(tagged, nullTags);
        Assert.Equal(new[] { Book_property.Tags }, cleared.Changed());
        Assert.True(cleared.Tags.HasValue);
        Assert.Null(cleared.Tags.Value);
        Book_update emptied = Book_update.Diff(tagged, emptyTags);
        Assert.Equal(new[] { Book_property.Tags }, emptied.Changed());
        Assert.True(emptied.Tags.HasValue);
        Assert.Null(emptied.Tags.Value);
    }

    /// <summary>
    /// <c>diff</c> refuses to carry a field that cannot be cleared as cleared, as <c>Set</c> does, rather than
    /// build a patch the runtime would reject; only an invalid record can empty such a field.
    /// </summary>
    [Fact]
    public void Diff_EmptyingANonClearableField_ThrowsNamingTheField()
    {
        Person[] authors = new[] { new Person { Name = "X" } };
        Book before = new() { Title = "A", Authors = authors };

        InvalidOperationException emptiedArray = Assert.Throws<InvalidOperationException>(
            () => Book_update.Diff(before, new Book { Title = "A", Authors = Array.Empty<Person>() }));
        Assert.Contains("'authors'", emptiedArray.Message, StringComparison.Ordinal);
        Assert.Contains("Book.Update", emptiedArray.Message, StringComparison.Ordinal);

        InvalidOperationException nulled = Assert.Throws<InvalidOperationException>(
            () => Book_update.Diff(before, new Book { Title = null!, Authors = authors }));
        Assert.Contains("'title'", nulled.Message, StringComparison.Ordinal);
    }

    /// <summary>
    /// The schema knows a field cannot be cleared when the key says so, as every generated key does for a field
    /// its target does not declare optional and the prelude's hand-written range keys do; a field whose key does
    /// not say counts as clearable unless its CLR type cannot hold <c>null</c>, and the runtime is what refuses a
    /// <c>null</c> for it.
    /// </summary>
    [Fact]
    public void Schema_KnowsWhichFieldsCanBeCleared()
    {
        Assert.True(BookProperties.Author.Clearable);
        Assert.True(BookProperties.Tags.Clearable);
        Assert.True(UserProperties.Email.Clearable);
        Assert.False(UserProperties.Name.Clearable);
        Assert.False(BookProperties.Title.Clearable);
        Assert.False(BookProperties.Authors.Clearable);
        Assert.False(Assert.Single(new Counter_update().Schema.Fields).Clearable);
        Assert.False(NxRangeProperties<long>.Start.Clearable);
        Assert.False(NxRangeProperties<string>.Start.Clearable);
        Assert.False(new NxField("n", typeof(long)).Clearable);
        Assert.True(new NxField("n", typeof(long?)).Clearable);
        Assert.False(new NxField("s", typeof(string), clearable: false).Clearable);
    }

    [Fact]
    public void Set_NullForANonClearableField_ThrowsNamingTheField()
    {
        InvalidOperationException error = Assert.Throws<InvalidOperationException>(
            () => new NxRange_update<string> { Start = null! });

        Assert.Contains("start", error.Message, StringComparison.Ordinal);
        Assert.Contains("Range.Update", error.Message, StringComparison.Ordinal);
    }

    [Fact]
    public void UpdateRecord_Json_NullForANonClearableField_ThrowsNamingTheField()
    {
        const string json = """{"$type":"Counter.Update","count":null}""";

        JsonException error = Assert.Throws<JsonException>(() => JsonSerializer.Deserialize<Counter_update>(json));

        Assert.Contains("count", error.Message, StringComparison.Ordinal);
        Assert.Contains(nameof(Counter_update), error.Message, StringComparison.Ordinal);
    }

    [Fact]
    public void UpdateRecord_MessagePack_NilForANonClearableField_ThrowsNamingTheField()
    {
        byte[] bytes = MessagePackSerializer.Serialize(
            new ClearingCounterPatch(),
            cancellationToken: TestContext.Current.CancellationToken);

        MessagePackSerializationException error = Assert.Throws<MessagePackSerializationException>(
            () => MessagePackSerializer.Deserialize<Counter_update>(
                bytes,
                cancellationToken: TestContext.Current.CancellationToken));

        Assert.Contains("count", error.ToString(), StringComparison.Ordinal);
        Assert.Contains(nameof(Counter_update), error.ToString(), StringComparison.Ordinal);
    }

    private static string DiagnosticText(NxEvaluationException error) =>
        string.Join("\n", error.Diagnostics.Select(diagnostic => diagnostic.Message));

    /// <summary>
    /// Collects the token of every action handler reference in a rendered JSON tree.
    /// </summary>
    private static List<string> HandlerTokens(JsonElement element)
    {
        List<string> tokens = new();
        CollectHandlerTokens(element, tokens);
        return tokens;
    }

    private static void CollectHandlerTokens(JsonElement element, List<string> tokens)
    {
        switch (element.ValueKind)
        {
            case JsonValueKind.Object:
                if (element.TryGetProperty("$type", out JsonElement type)
                    && type.ValueKind == JsonValueKind.String
                    && type.GetString() == "ActionHandler"
                    && element.TryGetProperty("token", out JsonElement token)
                    && token.ValueKind == JsonValueKind.String)
                {
                    tokens.Add(token.GetString()!);
                }

                foreach (JsonProperty property in element.EnumerateObject())
                {
                    CollectHandlerTokens(property.Value, tokens);
                }

                break;
            case JsonValueKind.Array:
                foreach (JsonElement item in element.EnumerateArray())
                {
                    CollectHandlerTokens(item, tokens);
                }

                break;
        }
    }
}

[MessagePackObject]
public sealed class CounterState
{
    [Key("count")]
    public int Count { get; set; }
}
