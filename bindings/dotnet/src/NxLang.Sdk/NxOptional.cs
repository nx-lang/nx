// Copyright (c) Bret Johnson. All rights reserved.
// Licensed under the MIT license. See LICENSE file in the project root for full license information.

using System;
using System.Collections.Generic;

namespace NxLang.Nx;

/// <summary>
/// A value that may be unset, which is different from being set to <see langword="null"/>.
/// </summary>
/// <remarks>
/// <para>Generated update record DTOs expose every field through this struct. In an update record an unset field
/// means "leave this field unchanged" and a field set to <see langword="null"/> means "set this field to null", so
/// the two stay distinct: the DTO stores only its set fields, and serialization writes only those.</para>
/// <para><c>default</c> is unset. The struct is an in-memory value only and is never serialized itself; a DTO's
/// fields travel through the map <see cref="NxUpdateRecord"/> keeps.</para>
/// </remarks>
/// <typeparam name="T">The type of the value when set.</typeparam>
public readonly struct NxOptional<T> : IEquatable<NxOptional<T>>
{
    private readonly T _value;

    /// <summary>
    /// Initializes a new instance of the <see cref="NxOptional{T}"/> struct that is set to <paramref name="value"/>.
    /// </summary>
    /// <param name="value">The value, which may be <see langword="null"/>.</param>
    public NxOptional(T value)
    {
        _value = value;
        HasValue = true;
    }

    /// <summary>
    /// Gets an unset value.
    /// </summary>
    public static NxOptional<T> Unset => default;

    /// <summary>
    /// Gets a value indicating whether the value is set.
    /// </summary>
    public bool HasValue { get; }

    /// <summary>
    /// Gets the value.
    /// </summary>
    /// <exception cref="InvalidOperationException">Thrown when the value is unset.</exception>
    public T Value => HasValue
        ? _value
        : throw new InvalidOperationException("The optional value is unset.");

    /// <summary>
    /// Converts a value to a set <see cref="NxOptional{T}"/>.
    /// </summary>
    /// <param name="value">The value, which may be <see langword="null"/>.</param>
    public static implicit operator NxOptional<T>(T value) => new(value);

    /// <summary>
    /// Determines whether two optional values are equal: both unset, or both set to equal values.
    /// </summary>
    public static bool operator ==(NxOptional<T> left, NxOptional<T> right) => left.Equals(right);

    /// <summary>
    /// Determines whether two optional values differ.
    /// </summary>
    public static bool operator !=(NxOptional<T> left, NxOptional<T> right) => !left.Equals(right);

    /// <summary>
    /// Returns the value when set, otherwise <paramref name="fallback"/>.
    /// </summary>
    /// <param name="fallback">The value to return when unset.</param>
    /// <returns>The value or the fallback.</returns>
    public T GetValueOrDefault(T fallback) => HasValue ? _value : fallback;

    /// <inheritdoc />
    public bool Equals(NxOptional<T> other)
    {
        return HasValue == other.HasValue
            && (!HasValue || EqualityComparer<T>.Default.Equals(_value, other._value));
    }

    /// <inheritdoc />
    public override bool Equals(object? obj) => obj is NxOptional<T> other && Equals(other);

    /// <inheritdoc />
    public override int GetHashCode() => HasValue ? HashCode.Combine(true, _value) : 0;

    /// <inheritdoc />
    public override string ToString() => HasValue ? _value?.ToString() ?? "null" : "(unset)";
}
