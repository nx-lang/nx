// Copyright (c) Bret Johnson. All rights reserved.
// Licensed under the MIT license. See LICENSE file in the project root for full license information.

using System;
using System.Text.Json;
using System.Text.Json.Serialization;
using MessagePack;
using NxLang.Nx;
using NxLang.Nx.Serialization;
using Xunit;

namespace NxLang.Nx.Tests;

[JsonConverter(typeof(NxEnumJsonConverter<ComponentDealStage, ComponentDealStageWireFormat>))]
[MessagePackFormatter(typeof(NxEnumMessagePackFormatter<ComponentDealStage, ComponentDealStageWireFormat>))]
public enum ComponentDealStage
{
    Draft,
    PendingReview,
    ClosedWon,
}

internal sealed class ComponentDealStageWireFormat : INxEnumWireFormat<ComponentDealStage>
{
    public static string Format(ComponentDealStage value)
    {
        return value switch
        {
            ComponentDealStage.Draft => "draft",
            ComponentDealStage.PendingReview => "pending_review",
            ComponentDealStage.ClosedWon => "closed_won",
            _ => throw new FormatException("Unknown NX enum value."),
        };
    }

    public static ComponentDealStage Parse(string value)
    {
        return value switch
        {
            "draft" => ComponentDealStage.Draft,
            "pending_review" => ComponentDealStage.PendingReview,
            "closed_won" => ComponentDealStage.ClosedWon,
            _ => throw new FormatException("Unknown NX enum member."),
        };
    }
}

[JsonConverter(typeof(NxEnumJsonConverter<RestrictedDealStage, RestrictedDealStageWireFormat>))]
[MessagePackFormatter(typeof(NxEnumMessagePackFormatter<RestrictedDealStage, RestrictedDealStageWireFormat>))]
public enum RestrictedDealStage
{
    Draft,
    ClosedWon,
}

internal sealed class RestrictedDealStageWireFormat : INxEnumWireFormat<RestrictedDealStage>
{
    public static string Format(RestrictedDealStage value)
    {
        return value switch
        {
            RestrictedDealStage.Draft => "draft",
            RestrictedDealStage.ClosedWon => "closed_won",
            _ => throw new FormatException("Unknown NX enum value."),
        };
    }

    public static RestrictedDealStage Parse(string value)
    {
        return value switch
        {
            "draft" => RestrictedDealStage.Draft,
            "closed_won" => RestrictedDealStage.ClosedWon,
            _ => throw new FormatException("Unknown NX enum member."),
        };
    }
}

[MessagePackObject]
public sealed class SearchBoxProps
{
    [Key("placeholder")]
    [JsonPropertyName("placeholder")]
    public string Placeholder { get; set; } = string.Empty;
}

[MessagePackObject]
public sealed class TextInputElement
{
    [Key("value")]
    [JsonPropertyName("value")]
    public string Value { get; set; } = string.Empty;

    [Key("placeholder")]
    [JsonPropertyName("placeholder")]
    public string Placeholder { get; set; } = string.Empty;
}

[MessagePackObject]
public sealed class SearchSubmittedAction
{
    [Key("$type")]
    [JsonPropertyName("$type")]
    public string Type { get; set; } = "SearchSubmitted";

    [Key("searchString")]
    [JsonPropertyName("searchString")]
    public string SearchString { get; set; } = string.Empty;
}

[MessagePackObject]
public sealed class ThemeModeProps
{
    [Key("theme")]
    [JsonPropertyName("theme")]
    public string Theme { get; set; } = string.Empty;
}

[MessagePackObject]
public sealed class ThemeModeElement
{
    [Key("theme")]
    [JsonPropertyName("theme")]
    public string Theme { get; set; } = string.Empty;
}

[MessagePackObject]
public sealed class DealStageProps
{
    [Key("stage")]
    [JsonPropertyName("stage")]
    public ComponentDealStage Stage { get; set; } = ComponentDealStage.Draft;
}

[MessagePackObject]
public sealed class DealStageElement
{
    [Key("stage")]
    [JsonPropertyName("stage")]
    public ComponentDealStage Stage { get; set; } = ComponentDealStage.Draft;
}

[MessagePackObject]
public sealed class RestrictedDealStageElement
{
    [Key("stage")]
    [JsonPropertyName("stage")]
    public RestrictedDealStage Stage { get; set; } = RestrictedDealStage.Draft;
}

public class NxRuntimeComponentTests
{
    [Fact]
    public void InitializeComponent_WithTypedProps_ReturnsRenderedElementAndStateSnapshot()
    {
        string source = """
            action SearchSubmitted = { searchString:string }

            component <SearchBox placeholder:string emits { SearchSubmitted } /> = {
              state { query:string = {placeholder} }
              <TextInput value={query} placeholder={placeholder} />
            }
            """;

        NxComponentInitResult<TextInputElement> result =
            NxRuntime.InitializeComponent<SearchBoxProps, TextInputElement>(
                source,
                "SearchBox",
                new SearchBoxProps { Placeholder = "Find docs" });

        Assert.Equal("Find docs", result.Rendered.Value);
        Assert.Equal("Find docs", result.Rendered.Placeholder);
        Assert.NotEmpty(result.StateSnapshot);
    }

    [Fact]
    public void DispatchComponentActions_WithPersistedStateSnapshot_SucceedsAcrossCalls()
    {
        string source = """
            action SearchSubmitted = { searchString:string }

            component <SearchBox placeholder:string emits { SearchSubmitted } /> = {
              state { query:string = {placeholder} }
              <TextInput value={query} placeholder={placeholder} />
            }
            """;

        NxComponentInitResult<TextInputElement> initResult =
            NxRuntime.InitializeComponent<SearchBoxProps, TextInputElement>(
                source,
                "SearchBox",
                new SearchBoxProps { Placeholder = "Find docs" });

        byte[] persistedSnapshot = initResult.StateSnapshot.ToArray();
        NxComponentDispatchResult<SearchSubmittedAction> dispatchResult =
            NxRuntime.DispatchComponentActions<SearchSubmittedAction[], SearchSubmittedAction>(
                source,
                persistedSnapshot,
                new[]
                {
                    new SearchSubmittedAction
                    {
                        SearchString = "docs"
                    }
                });

        Assert.Empty(dispatchResult.Effects);
        Assert.NotEmpty(dispatchResult.StateSnapshot);
    }

    [Fact]
    public void ComponentJsonWorkflows_ReturnExpectedJson()
    {
        string source = """
            action SearchSubmitted = { searchString:string }

            component <SearchBox placeholder:string emits { SearchSubmitted } /> = {
              state { query:string = {placeholder} }
              <TextInput value={query} placeholder={placeholder} />
            }
            """;

        NxComponentInitResult<JsonElement> initResult =
            NxRuntime.InitializeComponentJson(
                source,
                "SearchBox",
                new SearchBoxProps { Placeholder = "Find docs" });

        Assert.Equal("Find docs", initResult.Rendered.GetProperty("value").GetString());
        Assert.Equal("Find docs", initResult.Rendered.GetProperty("placeholder").GetString());
        Assert.NotEmpty(initResult.StateSnapshot);

        NxComponentDispatchResult<JsonElement> dispatchResult =
            NxRuntime.DispatchComponentActionsJson(
                source,
                initResult.StateSnapshot,
                new[]
                {
                    new SearchSubmittedAction
                    {
                        SearchString = "docs"
                    }
                });

        Assert.Empty(dispatchResult.Effects);
        Assert.NotEmpty(dispatchResult.StateSnapshot);
    }

    [Fact]
    public void DispatchComponentActions_WithUndeclaredAction_ThrowsEvaluationException()
    {
        string source = """
            action SearchSubmitted = { searchString:string }
            action ValueChanged = { value:string }

            component <SearchBox placeholder:string emits { SearchSubmitted } /> = {
              state { query:string = {placeholder} }
              <TextInput value={query} placeholder={placeholder} />
            }
            """;

        NxComponentInitResult<TextInputElement> initResult =
            NxRuntime.InitializeComponent<SearchBoxProps, TextInputElement>(
                source,
                "SearchBox",
                new SearchBoxProps { Placeholder = "Find docs" });

        NxEvaluationException error = Assert.Throws<NxEvaluationException>(
            () => NxRuntime.DispatchComponentActions<SearchSubmittedAction[], SearchSubmittedAction>(
                source,
                initResult.StateSnapshot,
                new[]
                {
                    new SearchSubmittedAction
                    {
                        Type = "ValueChanged",
                        SearchString = "docs"
                    }
                }));

        Assert.Contains(
            error.Diagnostics,
            diagnostic => diagnostic.Message.Contains("does not declare emitted action 'ValueChanged'"));
    }

    [Fact]
    public void InitializeComponent_WithBuildContext_ResolvesImportedComponentDefinition()
    {
        string tempPath = Path.Combine(Path.GetTempPath(), $"nx-component-context-{Guid.NewGuid():N}");
        Directory.CreateDirectory(tempPath);

        try
        {
            string appRoot = Path.Combine(tempPath, "app");
            string libraryRoot = Path.Combine(tempPath, "question-flow");
            Directory.CreateDirectory(appRoot);
            Directory.CreateDirectory(libraryRoot);
            File.WriteAllText(
                Path.Combine(libraryRoot, "QuestionFlow.nx"),
                """
                export action SearchSubmitted = { searchString:string }

                export component <SearchBox placeholder:string = "Find docs" emits { SearchSubmitted } /> = {
                  state { query:string = {placeholder} }
                  <TextInput value={query} placeholder={placeholder} />
                }
                """);

            string source = """
                import "../question-flow"
                let root() = { 0 }
                """;
            string mainPath = Path.Combine(appRoot, "main.nx");
            File.WriteAllText(mainPath, source);

            using NxLibraryRegistry registry = new();
            registry.LoadFromDirectory(libraryRoot);
            using NxProgramBuildContext buildContext = registry.CreateBuildContext();

            NxComponentInitResult<TextInputElement> result =
                NxRuntime.InitializeComponent<SearchBoxProps, TextInputElement>(
                    source,
                    "SearchBox",
                    buildContext,
                    new SearchBoxProps { Placeholder = "From library" },
                    mainPath);

            Assert.Equal("From library", result.Rendered.Value);
            Assert.Equal("From library", result.Rendered.Placeholder);
            Assert.NotEmpty(result.StateSnapshot);
        }
        finally
        {
            Directory.Delete(tempPath, recursive: true);
        }
    }

    [Fact]
    public void DispatchComponentActions_WithBuildContext_ReusesImportedComponentDefinition()
    {
        string tempPath = Path.Combine(Path.GetTempPath(), $"nx-component-context-{Guid.NewGuid():N}");
        Directory.CreateDirectory(tempPath);

        try
        {
            string appRoot = Path.Combine(tempPath, "app");
            string libraryRoot = Path.Combine(tempPath, "question-flow");
            Directory.CreateDirectory(appRoot);
            Directory.CreateDirectory(libraryRoot);
            File.WriteAllText(
                Path.Combine(libraryRoot, "QuestionFlow.nx"),
                """
                export action SearchSubmitted = { searchString:string }

                export component <SearchBox placeholder:string = "Find docs" emits { SearchSubmitted } /> = {
                  state { query:string = {placeholder} }
                  <TextInput value={query} placeholder={placeholder} />
                }
                """);

            string source = """
                import "../question-flow"
                let root() = { 0 }
                """;
            string mainPath = Path.Combine(appRoot, "main.nx");
            File.WriteAllText(mainPath, source);

            using NxLibraryRegistry registry = new();
            registry.LoadFromDirectory(libraryRoot);
            using NxProgramBuildContext buildContext = registry.CreateBuildContext();

            NxComponentInitResult<TextInputElement> initResult =
                NxRuntime.InitializeComponent<TextInputElement>(
                    source,
                    "SearchBox",
                    buildContext,
                    mainPath);

            NxComponentDispatchResult<SearchSubmittedAction> dispatchResult =
                NxRuntime.DispatchComponentActions<SearchSubmittedAction[], SearchSubmittedAction>(
                    source,
                    initResult.StateSnapshot,
                    buildContext,
                    new[]
                    {
                        new SearchSubmittedAction
                        {
                            SearchString = "docs"
                        }
                    },
                    mainPath);

            Assert.Empty(dispatchResult.Effects);
            Assert.NotEmpty(dispatchResult.StateSnapshot);
        }
        finally
        {
            Directory.Delete(tempPath, recursive: true);
        }
    }

    [Fact]
    public void InitializeComponent_WithProgramArtifact_ReusesImportedComponentDefinition()
    {
        string tempPath = Path.Combine(Path.GetTempPath(), $"nx-prepared-component-{Guid.NewGuid():N}");
        Directory.CreateDirectory(tempPath);

        try
        {
            string appRoot = Path.Combine(tempPath, "app");
            string libraryRoot = Path.Combine(tempPath, "question-flow");
            Directory.CreateDirectory(appRoot);
            Directory.CreateDirectory(libraryRoot);
            File.WriteAllText(
                Path.Combine(libraryRoot, "QuestionFlow.nx"),
                """
                export action SearchSubmitted = { searchString:string }

                export component <SearchBox placeholder:string = "Find docs" emits { SearchSubmitted } /> = {
                  state { query:string = {placeholder} }
                  <TextInput value={query} placeholder={placeholder} />
                }
                """);

            string source = """
                import "../question-flow"
                let root() = { 0 }
                """;
            string mainPath = Path.Combine(appRoot, "main.nx");
            File.WriteAllText(mainPath, source);
            using NxLibraryRegistry registry = new();
            registry.LoadFromDirectory(libraryRoot);
            using NxProgramBuildContext buildContext = registry.CreateBuildContext();
            using NxProgramArtifact programArtifact = NxProgramArtifact.Build(source, buildContext, mainPath);

            NxComponentInitResult<TextInputElement> result =
                NxRuntime.InitializeComponent<SearchBoxProps, TextInputElement>(
                    programArtifact,
                    "SearchBox",
                    new SearchBoxProps { Placeholder = "From library" });

            Assert.Equal("From library", result.Rendered.Value);
            Assert.Equal("From library", result.Rendered.Placeholder);
            Assert.NotEmpty(result.StateSnapshot);
        }
        finally
        {
            Directory.Delete(tempPath, recursive: true);
        }
    }

    [Fact]
    public void InitializeComponent_WithBareStringEnumProp_ReturnsBareStringInRenderedOutput()
    {
        string source = """
            enum ThemeMode = | light | dark

            external component <ThemePicker theme:ThemeMode />
            """;

        NxComponentInitResult<ThemeModeElement> result =
            NxRuntime.InitializeComponent<ThemeModeProps, ThemeModeElement>(
                source,
                "ThemePicker",
                new ThemeModeProps { Theme = "light" });

        Assert.Equal("light", result.Rendered.Theme);
    }

    [Fact]
    public void InitializeComponent_WithUnknownEnumMember_ThrowsEvaluationException()
    {
        string source = """
            enum ThemeMode = | light | dark

            external component <ThemePicker theme:ThemeMode />
            """;

        NxEvaluationException error = Assert.Throws<NxEvaluationException>(
            () => NxRuntime.InitializeComponent<ThemeModeProps, ThemeModeElement>(
                source,
                "ThemePicker",
                new ThemeModeProps { Theme = "sparkly" }));

        Assert.Contains(
            error.Diagnostics,
            diagnostic => diagnostic.Message.Contains("unknown enum member 'sparkly'"));
    }

    [Fact]
    public void InitializeComponent_WithEnumTypedDto_RoundTripsEnumThroughRuntimeWrapper()
    {
        string source = """
            enum DealStage = | draft | pending_review | closed_won

            external component <Pipeline stage:DealStage />
            """;

        NxComponentInitResult<DealStageElement> result =
            NxRuntime.InitializeComponent<DealStageProps, DealStageElement>(
                source,
                "Pipeline",
                new DealStageProps { Stage = ComponentDealStage.PendingReview });

        Assert.Equal(ComponentDealStage.PendingReview, result.Rendered.Stage);
    }

    [Fact]
    public void InitializeComponentJson_RawEnumResult_CanBeMappedToEnumTypedDto()
    {
        string source = """
            enum DealStage = | draft | pending_review | closed_won

            external component <Pipeline stage:DealStage />
            """;

        NxComponentInitResult<JsonElement> result =
            NxRuntime.InitializeComponentJson(
                source,
                "Pipeline",
                new DealStageProps { Stage = ComponentDealStage.PendingReview });

        DealStageElement? rendered = JsonSerializer.Deserialize<DealStageElement>(result.Rendered.GetRawText());

        Assert.NotNull(rendered);
        Assert.Equal(ComponentDealStage.PendingReview, rendered!.Stage);
    }

    [Fact]
    public void InitializeComponent_WithEnumTypedDtoMismatch_ThrowsWrappedSerializationError()
    {
        string source = """
            enum DealStage = | draft | pending_review | closed_won

            external component <Pipeline stage:DealStage />
            """;

        InvalidOperationException error = Assert.Throws<InvalidOperationException>(
            () => NxRuntime.InitializeComponent<DealStageProps, RestrictedDealStageElement>(
                source,
                "Pipeline",
                new DealStageProps { Stage = ComponentDealStage.PendingReview }));

        Assert.Contains("invalid component initialization MessagePack payload", error.Message, StringComparison.OrdinalIgnoreCase);

        MessagePackSerializationException outer = Assert.IsType<MessagePackSerializationException>(error.InnerException);
        MessagePackSerializationException inner = Assert.IsType<MessagePackSerializationException>(outer.InnerException);

        Assert.Equal("Unknown NX enum member.", inner.Message);
        Assert.IsType<FormatException>(inner.InnerException);
    }

    [Fact]
    public void DispatchComponentActions_WithProgramArtifact_ReusesImportedComponentDefinition()
    {
        string tempPath = Path.Combine(Path.GetTempPath(), $"nx-prepared-component-{Guid.NewGuid():N}");
        Directory.CreateDirectory(tempPath);

        try
        {
            string appRoot = Path.Combine(tempPath, "app");
            string libraryRoot = Path.Combine(tempPath, "question-flow");
            Directory.CreateDirectory(appRoot);
            Directory.CreateDirectory(libraryRoot);
            File.WriteAllText(
                Path.Combine(libraryRoot, "QuestionFlow.nx"),
                """
                export action SearchSubmitted = { searchString:string }

                export component <SearchBox placeholder:string = "Find docs" emits { SearchSubmitted } /> = {
                  state { query:string = {placeholder} }
                  <TextInput value={query} placeholder={placeholder} />
                }
                """);

            string source = """
                import "../question-flow"
                let root() = { 0 }
                """;
            string mainPath = Path.Combine(appRoot, "main.nx");
            File.WriteAllText(mainPath, source);
            using NxLibraryRegistry registry = new();
            registry.LoadFromDirectory(libraryRoot);
            using NxProgramBuildContext buildContext = registry.CreateBuildContext();
            using NxProgramArtifact programArtifact = NxProgramArtifact.Build(source, buildContext, mainPath);
            NxComponentInitResult<TextInputElement> initResult =
                NxRuntime.InitializeComponent<TextInputElement>(
                    programArtifact,
                    "SearchBox");

            NxComponentDispatchResult<SearchSubmittedAction> dispatchResult =
                NxRuntime.DispatchComponentActions<SearchSubmittedAction[], SearchSubmittedAction>(
                    programArtifact,
                    initResult.StateSnapshot,
                    new[]
                    {
                        new SearchSubmittedAction
                        {
                            SearchString = "docs"
                        }
                    });

            Assert.Empty(dispatchResult.Effects);
            Assert.NotEmpty(dispatchResult.StateSnapshot);
        }
        finally
        {
            Directory.Delete(tempPath, recursive: true);
        }
    }
}
