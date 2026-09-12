// Copyright (c) Bret Johnson. All rights reserved.
// Licensed under the MIT license. See LICENSE file in the project root for full license information.

using System;
using System.Text.Json.Serialization;
using MessagePack;

namespace NxLang.Nx;

/// <summary>
/// Identifies a handler bound in rendered component output.
/// </summary>
/// <remarks>
/// Rendered output encodes every bound handler as an <c>ActionHandler</c> record. Type a handler-bound property of a
/// rendered element DTO with this class to read it. The token is present only in output returned by component
/// initialization and dispatch, and is valid only with the state snapshot returned by the same call.
/// </remarks>
[MessagePackObject]
public sealed class NxActionHandlerRef
{
    /// <summary>
    /// Gets or sets the public name of the action the handler accepts, such as <c>Button.Tapped</c>.
    /// </summary>
    [Key("action")]
    [JsonPropertyName("action")]
    public string Action { get; set; } = string.Empty;

    /// <summary>
    /// Gets or sets the dispatch token, or <see langword="null"/> when the output came from pure evaluation.
    /// </summary>
    [Key("token")]
    [JsonPropertyName("token")]
    public string? Token { get; set; }

    /// <summary>
    /// Builds a dispatch batch entry that runs this handler with <paramref name="action"/>.
    /// </summary>
    /// <typeparam name="TAction">The managed action type, which must serialize with its <c>$type</c>.</typeparam>
    /// <param name="action">The action to feed the handler.</param>
    /// <returns>A handler invocation for this handler's token.</returns>
    /// <exception cref="InvalidOperationException">Thrown when this reference carries no token.</exception>
    public NxHandlerInvocation<TAction> Invoke<TAction>(TAction action)
    {
        if (string.IsNullOrEmpty(Token))
        {
            throw new InvalidOperationException(
                $"The handler for '{Action}' carries no token; "
                + "only initialization and dispatch output can be dispatched.");
        }

        return new NxHandlerInvocation<TAction>(Token, action);
    }
}
