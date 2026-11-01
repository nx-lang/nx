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






}
