// Copyright (c) Bret Johnson. All rights reserved.
// Licensed under the MIT license. See LICENSE file in the project root for full license information.

using System;
using System.Text.Json;
using System.Text.Json.Serialization;
using MessagePack;
using NxLang.Nx;
using Xunit;

namespace NxLang.Nx.Tests;

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
    public void ComponentResultBytesToJson_DebugConvertersReturnExpectedJson()
    {
        string source = """
            action SearchSubmitted = { searchString:string }

            component <SearchBox placeholder:string emits { SearchSubmitted } /> = {
              state { query:string = {placeholder} }
              <TextInput value={query} placeholder={placeholder} />
            }
            """;

        byte[] propsBytes = MessagePackSerializer.Serialize(
            new SearchBoxProps { Placeholder = "Find docs" },
            cancellationToken: TestContext.Current.CancellationToken);
        byte[] initBytes = NxRuntime.InitializeComponentBytes(source, "SearchBox", propsBytes);
        string initJson = NxRuntime.ComponentInitResultBytesToJson(initBytes);

        using JsonDocument initDocument = JsonDocument.Parse(initJson);
        JsonElement initRoot = initDocument.RootElement;
        Assert.Equal("Find docs", initRoot.GetProperty("rendered").GetProperty("value").GetString());
        Assert.Equal("Find docs", initRoot.GetProperty("rendered").GetProperty("placeholder").GetString());

        string? stateSnapshot = initRoot.GetProperty("state_snapshot").GetString();
        Assert.False(string.IsNullOrWhiteSpace(stateSnapshot));

        byte[] actionsBytes = MessagePackSerializer.Serialize(
            new[]
            {
                new SearchSubmittedAction
                {
                    SearchString = "docs"
                }
            },
            cancellationToken: TestContext.Current.CancellationToken);
        byte[] dispatchBytes = NxRuntime.DispatchComponentActionsBytes(
            source,
            Convert.FromBase64String(stateSnapshot!),
            actionsBytes);
        string dispatchJson = NxRuntime.ComponentDispatchResultBytesToJson(dispatchBytes);

        using JsonDocument dispatchDocument = JsonDocument.Parse(dispatchJson);
        JsonElement dispatchRoot = dispatchDocument.RootElement;
        Assert.Equal(0, dispatchRoot.GetProperty("effects").GetArrayLength());
        Assert.False(string.IsNullOrWhiteSpace(dispatchRoot.GetProperty("state_snapshot").GetString()));
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
                action SearchSubmitted = { searchString:string }

                component <SearchBox placeholder:string = "Find docs" emits { SearchSubmitted } /> = {
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
                action SearchSubmitted = { searchString:string }

                component <SearchBox placeholder:string = "Find docs" emits { SearchSubmitted } /> = {
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
                action SearchSubmitted = { searchString:string }

                component <SearchBox placeholder:string = "Find docs" emits { SearchSubmitted } /> = {
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
                action SearchSubmitted = { searchString:string }

                component <SearchBox placeholder:string = "Find docs" emits { SearchSubmitted } /> = {
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
