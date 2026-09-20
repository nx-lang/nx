// Copyright (c) Bret Johnson. All rights reserved.
// Licensed under the MIT license. See LICENSE file in the project root for full license information.

using System;
using System.Collections.Generic;
using System.Text.Json.Serialization;
using MessagePack;

namespace NxLang.Nx;

/// <summary>
/// The bounds of a range: the NX prelude's <c>Range</c> record, which <c>a..b</c> and <c>a..=b</c> construct.
/// </summary>
/// <remarks>
/// <para>
/// The prelude is the module every NX module sees without an import, so its declarations are not generated into a host's
/// namespace: two generated libraries sharing one namespace would each declare a copy. It is named <c>NxRange</c>
/// rather than <c>Range</c> because generated files open with <c>using System;</c>, where <c>Range</c> is
/// <see cref="System.Range"/>.
/// </para>
/// <para>
/// The wire shape is exactly what the C# emitter would generate for a user-declared record with these fields and the
/// NX name <c>Range</c>, through both <c>System.Text.Json</c> and MessagePack, so a range crosses the host boundary
/// like any other record.
/// </para>
/// </remarks>
/// <typeparam name="T">The type of the bounds, which NX writes as the type argument of <c>&lt;Range T=…/&gt;</c>.</typeparam>
[MessagePackObject]
public sealed class NxRange<T> : IEquatable<NxRange<T>>
{
    /// <summary>
    /// Initializes a new instance of the <see cref="NxRange{T}"/> class with default bounds, as a deserializer does.
    /// </summary>
    public NxRange()
    {
    }

    /// <summary>
    /// Initializes a new instance of the <see cref="NxRange{T}"/> class from its bounds.
    /// </summary>
    /// <param name="start">Where the range starts.</param>
    /// <param name="end">Where the range stops.</param>
    /// <param name="endInclusive"><see langword="true"/> when <paramref name="end"/> is part of the range.</param>
    public NxRange(T start, T end, bool endInclusive)
    {
        Start = start;
        End = end;
        EndInclusive = endInclusive;
    }

    /// <summary>
    /// Gets or sets where the range starts, which is part of the range.
    /// </summary>
    [Key("start")]
    [JsonPropertyName("start")]
    public T Start { get; set; } = default!;

    /// <summary>
    /// Gets or sets where the range stops, which is part of the range only when
    /// <see cref="EndInclusive"/> is <see langword="true"/>.
    /// </summary>
    [Key("end")]
    [JsonPropertyName("end")]
    public T End { get; set; } = default!;

    /// <summary>
    /// Gets or sets a value indicating whether <see cref="End"/> is part of the range: <see langword="false"/> for
    /// <c>a..b</c> and <see langword="true"/> for <c>a..=b</c>.
    /// </summary>
    [Key("endInclusive")]
    [JsonPropertyName("endInclusive")]
    public bool EndInclusive { get; set; }

    /// <summary>
    /// Compares two ranges by their bounds.
    /// </summary>
    /// <param name="left">The left range.</param>
    /// <param name="right">The right range.</param>
    /// <returns><see langword="true"/> when the two are equal.</returns>
    public static bool operator ==(NxRange<T>? left, NxRange<T>? right) =>
        left is null ? right is null : left.Equals(right);

    /// <summary>
    /// Compares two ranges by their bounds.
    /// </summary>
    /// <param name="left">The left range.</param>
    /// <param name="right">The right range.</param>
    /// <returns><see langword="true"/> when the two differ.</returns>
    public static bool operator !=(NxRange<T>? left, NxRange<T>? right) => !(left == right);

    /// <inheritdoc/>
    public bool Equals(NxRange<T>? other) =>
        other is not null
        && EqualityComparer<T>.Default.Equals(Start, other.Start)
        && EqualityComparer<T>.Default.Equals(End, other.End)
        && EndInclusive == other.EndInclusive;

    /// <inheritdoc/>
    public override bool Equals(object? obj) => Equals(obj as NxRange<T>);

    /// <inheritdoc/>
    public override int GetHashCode() => HashCode.Combine(Start, End, EndInclusive);

    /// <inheritdoc/>
    public override string ToString() =>
        $"{Start}{(EndInclusive ? "..=" : "..")}{End}";
}
