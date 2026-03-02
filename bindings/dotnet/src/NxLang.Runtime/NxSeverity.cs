// Copyright (c) Bret Johnson. All rights reserved.
// Licensed under the MIT license. See LICENSE file in the project root for full license information.

using System.Text.Json.Serialization;
using MessagePack;
using NxLang.Nx.Serialization;

namespace NxLang.Nx;

/// <summary>
/// Defines the severity level of a diagnostic emitted by the NX runtime.
/// </summary>
[JsonConverter(typeof(NxSeverityJsonConverter))]
[MessagePackFormatter(typeof(NxSeverityMessagePackFormatter))]
public enum NxSeverity
{
    Error,
    Warning,
    Info,
    Hint,
}
