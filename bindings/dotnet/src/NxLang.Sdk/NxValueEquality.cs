// Copyright (c) Bret Johnson. All rights reserved.
// Licensed under the MIT license. See LICENSE file in the project root for full license information.

using System;
using System.Collections.Generic;
using System.Text.Json;

namespace NxLang.Nx;

/// <summary>
/// The value comparison <see cref="NxUpdate{TRecord}.Diff{TUpdate}"/> makes, ported from the TypeScript runtime's
/// <c>nxValuesEqual</c>.
/// </summary>
internal static class NxValueEquality
{
    /// <summary>
    /// Compares two field values as NX does: <see langword="null"/> equals only <see langword="null"/>, arrays
    /// compare element-wise, update records compare by the fields they carry, and any other record compares
    /// structurally once its runtime type matches.
    /// </summary>
    /// <remarks>
    /// A generated record class has no structural <c>Equals</c>, and the SDK does not reflect over its members, so
    /// two records compare through their canonical JSON encoding, which is the same declared-order property walk
    /// the serializer already makes for them.
    /// </remarks>
    public static bool ValuesEqual(object? left, object? right)
    {
        if (left is null || right is null)
        {
            return left is null && right is null;
        }

        if (left is Array || right is Array)
        {
            if (left is not Array leftItems || right is not Array rightItems || leftItems.Length != rightItems.Length)
            {
                return false;
            }

            for (int index = 0; index < leftItems.Length; index++)
            {
                if (!ValuesEqual(leftItems.GetValue(index), rightItems.GetValue(index)))
                {
                    return false;
                }
            }

            return true;
        }

        if (left is NxUpdateRecord || right is NxUpdateRecord)
        {
            if (left is not NxUpdateRecord leftUpdate
                || right is not NxUpdateRecord rightUpdate
                || leftUpdate.Schema.NxType != rightUpdate.Schema.NxType
                || leftUpdate.Fields.Count != rightUpdate.Fields.Count)
            {
                return false;
            }

            foreach (KeyValuePair<string, object?> field in leftUpdate.Fields)
            {
                if (!rightUpdate.Fields.TryGetValue(field.Key, out object? other) || !ValuesEqual(field.Value, other))
                {
                    return false;
                }
            }

            return true;
        }

        if (left is IConvertible || right is IConvertible)
        {
            return left.Equals(right);
        }

        // A polymorphic field that switches to a sibling type with the same fields is a change: the
        // TypeScript runtime sees it through the `$type` key, and serializing each side at its own
        // runtime type below would not write one.
        if (left.GetType() != right.GetType())
        {
            return false;
        }

        ReadOnlySpan<byte> leftJson = JsonSerializer.SerializeToUtf8Bytes(left, left.GetType());
        ReadOnlySpan<byte> rightJson = JsonSerializer.SerializeToUtf8Bytes(right, right.GetType());
        return leftJson.SequenceEqual(rightJson);
    }
}
