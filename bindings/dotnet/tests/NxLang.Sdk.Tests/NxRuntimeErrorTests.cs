// Copyright (c) The NX Authors.
// Licensed under the MIT license. See LICENSE file in the project root for full license information.

using System.Linq;
using NxLang.Nx;
using Xunit;

namespace NxLang.Nx.Tests;

public class NxRuntimeErrorTests
{
















    [Fact]
    public void NxRuntime_DoesNotExposeLegacyJsonConverterHelpers()
    {
        Assert.Null(typeof(NxRuntime).GetMethod("ValueBytesToJson"));
        Assert.Null(typeof(NxRuntime).GetMethod("DiagnosticsBytesToJson"));
        Assert.Null(typeof(NxRuntime).GetMethod("ComponentInitResultBytesToJson"));
        Assert.Null(typeof(NxRuntime).GetMethod("ComponentDispatchResultBytesToJson"));
    }
}
