// Copyright (c) Bret Johnson. All rights reserved.
// Licensed under the MIT license. See LICENSE file in the project root for full license information.

using System;
using System.Text.Json.Serialization;
using MessagePack;

namespace NxLang.Nx;

/// <summary>
/// A dispatch batch entry that runs a handler from rendered output with an action.
/// </summary>
/// <remarks>
/// Serializes to the canonical <c>{ "$type": "ActionHandlerInvocation", "token": ..., "action": ... }</c> record
/// in both MessagePack and JSON. A batch may mix invocations with emitted actions; pass such a batch as an
/// <see cref="object"/> array.
/// </remarks>
/// <typeparam name="TAction">The managed action type, which must serialize with its <c>$type</c>.</typeparam>
[MessagePackObject]
public sealed class NxHandlerInvocation<TAction>
{
    /// <summary>
    /// The record type name of a handler invocation batch entry.
    /// </summary>
    public const string TypeName = "ActionHandlerInvocation";

    /// <summary>
    /// Initializes a new instance of the <see cref="NxHandlerInvocation{TAction}"/> class.
    /// </summary>
    /// <param name="token">The handler token read from rendered output.</param>
    /// <param name="action">The action to feed the handler.</param>
    public NxHandlerInvocation(string token, TAction action)
    {
        ArgumentException.ThrowIfNullOrEmpty(token);
        Token = token;
        Action = action;
    }

    /// <summary>
    /// Gets the record type name, always <see cref="TypeName"/>.
    /// </summary>
    [Key("$type")]
    [JsonPropertyName("$type")]
    public string Type { get; } = TypeName;

    /// <summary>
    /// Gets the handler token read from rendered output.
    /// </summary>
    [Key("token")]
    [JsonPropertyName("token")]
    public string Token { get; }

    /// <summary>
    /// Gets the action fed to the handler.
    /// </summary>
    [Key("action")]
    [JsonPropertyName("action")]
    public TAction Action { get; }
}
