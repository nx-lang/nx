// Copyright (c) Bret Johnson. All rights reserved.
// Licensed under the MIT license. See LICENSE file in the project root for full license information.

using System;
using System.Text.Json.Serialization;
using MessagePack;

namespace NxLang.Nx;

/// <summary>
/// Represents the result of component initialization.
/// </summary>
/// <typeparam name="TElement">The managed type used for the rendered element payload.</typeparam>
[MessagePackObject]
public sealed class NxComponentInitResult<TElement>
{
    /// <summary>
    /// Gets or sets the rendered component body.
    /// </summary>
    [Key("rendered")]
    [JsonPropertyName("rendered")]
    public TElement Rendered { get; set; } = default!;

    /// <summary>
    /// Gets or sets the opaque host-owned component state snapshot.
    /// </summary>
    [Key("state_snapshot")]
    [JsonPropertyName("state_snapshot")]
    public byte[] StateSnapshot { get; set; } = Array.Empty<byte>();
}
