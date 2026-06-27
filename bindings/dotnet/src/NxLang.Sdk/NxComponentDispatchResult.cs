// Copyright (c) Bret Johnson. All rights reserved.
// Licensed under the MIT license. See LICENSE file in the project root for full license information.

using System;
using System.Text.Json.Serialization;
using MessagePack;

namespace NxLang.Nx;

/// <summary>
/// Represents the result of dispatching actions against a component instance.
/// </summary>
/// <typeparam name="TEffect">The managed type used for effect action payloads.</typeparam>
[MessagePackObject]
public sealed class NxComponentDispatchResult<TEffect>
{
    /// <summary>
    /// Gets or sets the effect actions returned in dispatch order.
    /// </summary>
    [Key("effects")]
    [JsonPropertyName("effects")]
    public TEffect[] Effects { get; set; } = Array.Empty<TEffect>();

    /// <summary>
    /// Gets or sets the opaque host-owned component state snapshot.
    /// </summary>
    [Key("state_snapshot")]
    [JsonPropertyName("state_snapshot")]
    public byte[] StateSnapshot { get; set; } = Array.Empty<byte>();
}
