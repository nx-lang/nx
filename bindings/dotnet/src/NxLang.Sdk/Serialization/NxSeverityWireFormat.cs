// Copyright (c) Bret Johnson. All rights reserved.
// Licensed under the MIT license. See LICENSE file in the project root for full license information.

using System;

namespace NxLang.Nx.Serialization;

internal sealed class NxSeverityWireFormat : INxEnumWireFormat<NxSeverity>
{
    public static string Format(NxSeverity value) =>
        value switch
        {
            NxSeverity.Error => "error",
            NxSeverity.Warning => "warning",
            NxSeverity.Info => "info",
            NxSeverity.Hint => "hint",
            _ => throw new FormatException("Unknown NX severity value."),
        };

    public static NxSeverity Parse(string value) =>
        value switch
        {
            "error" => NxSeverity.Error,
            "warning" => NxSeverity.Warning,
            "info" => NxSeverity.Info,
            "hint" => NxSeverity.Hint,
            _ => throw new FormatException("Unknown NX severity value."),
        };
}
