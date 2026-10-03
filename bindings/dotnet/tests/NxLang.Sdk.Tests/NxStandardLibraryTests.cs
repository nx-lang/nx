// Copyright (c) The NX Authors.
// Licensed under the MIT license. See LICENSE file in the project root for full license information.

using System.Text.Json;
using Xunit;

namespace NxLang.Nx.Tests;

/// <summary>
/// A standard library, NX source the SDK itself carries, reached from .NET with nothing loaded.
/// </summary>
public class NxStandardLibraryTests
{
    private const string AgentSource = """
        import "@nx/agent"

        /// Finds the plans that fit a team.
        let findPlans(teamSize:int): string* = { "Team" }

        let root() = {
          <Agent name="support" tools={ <FunctionTool function={findPlans} /> }>Be brief.</Agent>
        }
        """;

    [Fact]
    public void EvaluateJson_WithAnImportedStandardLibrary_ReturnsTheAgentRecord()
    {
        JsonElement agent = NxRuntime.EvaluateJson(AgentSource, "main.nx");

        Assert.Equal("Agent", agent.GetProperty("$type").GetString());
        Assert.Equal("support", agent.GetProperty("name").GetString());
        Assert.Equal("Be brief.", agent.GetProperty("instructions").GetString());

        JsonElement tool = agent.GetProperty("tools")[0];
        Assert.Equal("FunctionTool", tool.GetProperty("$type").GetString());
        JsonElement function = tool.GetProperty("function");
        Assert.Equal("Function", function.GetProperty("$type").GetString());
        Assert.Equal("main.nx", function.GetProperty("module").GetString());
        Assert.Equal("findPlans", function.GetProperty("name").GetString());
    }

    [Fact]
    public void EvaluateJson_WithAnUnknownStandardLibrary_ReportsTheLibrariesThatExist()
    {
        NxEvaluationException exception = Assert.Throws<NxEvaluationException>(
            () => NxRuntime.EvaluateJson("import \"@nx/automation\"\nlet root() = { 1 }", "main.nx"));

        NxDiagnostic diagnostic = Assert.Single(
            exception.Diagnostics,
            diagnostic => diagnostic.Code == "unknown-standard-library");
        Assert.Contains("@nx/automation", diagnostic.Message);
        Assert.Contains("@nx/agent", diagnostic.Message);
    }
}
