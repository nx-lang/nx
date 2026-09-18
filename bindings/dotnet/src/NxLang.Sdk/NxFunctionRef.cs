// Copyright (c) Bret Johnson. All rights reserved.
// Licensed under the MIT license. See LICENSE file in the project root for full license information.

using System.Text.Json.Serialization;
using MessagePack;

namespace NxLang.Nx;

/// <summary>
/// Identifies a function a rendered value carries, such as a template bound to a list.
/// </summary>
/// <remarks>
/// A function value in NX captures nothing: it names a declaration, and rendered output encodes it as a
/// <c>Function</c> record of the declaring module's identity and the function's name. Type a function-typed property of
/// a rendered element DTO with this class to read which function it was handed. Calling it from .NET is not supported;
/// a function value is invoked by the NX program that received it, or by a host runtime that can call one.
/// </remarks>
[MessagePackObject]
public sealed class NxFunctionRef
{
    /// <summary>
    /// Gets or sets the identity of the module that declares the function, such as <c>app/main.nx</c>.
    /// </summary>
    [Key("module")]
    [JsonPropertyName("module")]
    public string Module { get; set; } = string.Empty;

    /// <summary>
    /// Gets or sets the function's declared name, such as <c>ContactRow</c>.
    /// </summary>
    [Key("name")]
    [JsonPropertyName("name")]
    public string Name { get; set; } = string.Empty;
}
