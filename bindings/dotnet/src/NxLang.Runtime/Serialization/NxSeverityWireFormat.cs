// Copyright (c) Bret Johnson. All rights reserved.
// Licensed under the MIT license. See LICENSE file in the project root for full license information.

using System;

namespace NxLang.Nx.Serialization;

internal static class NxSeverityWireFormat
{
    internal static string Format(NxSeverity value)
    {
        return value switch
        {
            NxSeverity.Error => "error",
            NxSeverity.Warning => "warning",
            NxSeverity.Info => "info",
            NxSeverity.Hint => "hint",
            _ => throw new FormatException("Unknown NX severity value."),
        };
    }

    internal static NxSeverity Parse(string value)
    {
        return value switch
        {
            "error" => NxSeverity.Error,
            "warning" => NxSeverity.Warning,
            "info" => NxSeverity.Info,
            "hint" => NxSeverity.Hint,
            _ => throw new FormatException("Unknown NX severity value."),
        };
    }
}
