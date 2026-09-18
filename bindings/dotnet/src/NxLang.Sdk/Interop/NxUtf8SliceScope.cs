// Copyright (c) Bret Johnson. All rights reserved.
// Licensed under the MIT license. See LICENSE file in the project root for full license information.

using System;
using System.Collections.Generic;
using System.Runtime.InteropServices;
using System.Text;

namespace NxLang.Nx.Interop;

/// <summary>
/// Pins a list of strings as UTF-8 slices for the duration of one native call, such as the
/// implicit-import identities of a workspace build.
/// </summary>
internal sealed class NxUtf8SliceScope : IDisposable
{
    private readonly GCHandle[] _stringHandles;
    private GCHandle _sliceHandle;
    private bool _disposed;

    internal NxUtf8SliceScope(IReadOnlyList<string>? values)
    {
        values ??= Array.Empty<string>();
        NxUtf8Slice[] slices = new NxUtf8Slice[values.Count];
        _stringHandles = new GCHandle[values.Count];

        try
        {
            for (int index = 0; index < values.Count; index++)
            {
                string value = values[index];
                ArgumentNullException.ThrowIfNull(value);
                byte[] bytes = Encoding.UTF8.GetBytes(value);
                _stringHandles[index] = GCHandle.Alloc(bytes, GCHandleType.Pinned);
                slices[index] = new NxUtf8Slice
                {
                    Ptr = bytes.Length == 0 ? IntPtr.Zero : _stringHandles[index].AddrOfPinnedObject(),
                    Len = (nuint)bytes.Length,
                };
            }

            _sliceHandle = GCHandle.Alloc(slices, GCHandleType.Pinned);
            Pointer = slices.Length == 0 ? IntPtr.Zero : _sliceHandle.AddrOfPinnedObject();
            Count = (nuint)slices.Length;
        }
        catch
        {
            Dispose();
            throw;
        }
    }

    internal IntPtr Pointer { get; }

    internal nuint Count { get; }

    public void Dispose()
    {
        if (_disposed)
        {
            return;
        }

        if (_sliceHandle.IsAllocated)
        {
            _sliceHandle.Free();
        }

        for (int index = 0; index < _stringHandles.Length; index++)
        {
            if (_stringHandles[index].IsAllocated)
            {
                _stringHandles[index].Free();
            }
        }

        _disposed = true;
    }
}
