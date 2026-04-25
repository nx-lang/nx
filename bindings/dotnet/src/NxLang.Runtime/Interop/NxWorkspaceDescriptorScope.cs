// Copyright (c) Bret Johnson. All rights reserved.
// Licensed under the MIT license. See LICENSE file in the project root for full license information.

using System;
using System.Runtime.InteropServices;
using System.Text;
using NxLang.Nx;

namespace NxLang.Nx.Interop;

internal sealed class NxWorkspaceDescriptorScope : IDisposable
{
    private readonly GCHandle[] _identityHandles;
    private readonly GCHandle[] _sourceHandles;
    private GCHandle _descriptorHandle;
    private bool _disposed;

    internal NxWorkspaceDescriptorScope(NxWorkspace workspace)
    {
        ArgumentNullException.ThrowIfNull(workspace);
        ValidateWorkspace(workspace);

        NxWorkspaceModuleDescriptor[] descriptors = new NxWorkspaceModuleDescriptor[workspace.Modules.Count];
        _identityHandles = new GCHandle[workspace.Modules.Count];
        _sourceHandles = new GCHandle[workspace.Modules.Count];

        try
        {
            for (int index = 0; index < workspace.Modules.Count; index++)
            {
                NxWorkspaceModule module = workspace.Modules[index];
                byte[] identityBytes = Encoding.UTF8.GetBytes(module.Identity);
                _identityHandles[index] = GCHandle.Alloc(identityBytes, GCHandleType.Pinned);
                _sourceHandles[index] = PinSource(module.SourceUtf8, out IntPtr sourcePointer);

                descriptors[index] = new NxWorkspaceModuleDescriptor
                {
                    IdentityPtr = _identityHandles[index].AddrOfPinnedObject(),
                    IdentityLen = (nuint)identityBytes.Length,
                    SourceUtf8Ptr = sourcePointer,
                    SourceUtf8Len = (nuint)module.SourceUtf8.Length,
                };
            }

            _descriptorHandle = GCHandle.Alloc(descriptors, GCHandleType.Pinned);
            Pointer = descriptors.Length == 0 ? IntPtr.Zero : _descriptorHandle.AddrOfPinnedObject();
            Count = (nuint)descriptors.Length;
        }
        catch
        {
            Dispose();
            throw;
        }
    }

    internal IntPtr Pointer { get; }

    internal nuint Count { get; }

    private static void ValidateWorkspace(NxWorkspace workspace)
    {
        for (int index = 0; index < workspace.Modules.Count; index++)
        {
            NxWorkspaceModule module = workspace.Modules[index];
            ArgumentNullException.ThrowIfNull(module);
            if (module.Identity.Length == 0)
            {
                throw new ArgumentException("Workspace module identity must not be empty.", nameof(workspace));
            }
        }
    }

    private static GCHandle PinSource(ReadOnlyMemory<byte> sourceUtf8, out IntPtr pointer)
    {
        if (sourceUtf8.Length == 0)
        {
            pointer = IntPtr.Zero;
            return default;
        }

        if (MemoryMarshal.TryGetArray(sourceUtf8, out ArraySegment<byte> segment) && segment.Array is not null)
        {
            GCHandle handle = GCHandle.Alloc(segment.Array, GCHandleType.Pinned);
            pointer = IntPtr.Add(handle.AddrOfPinnedObject(), segment.Offset);
            return handle;
        }

        byte[] sourceBytes = sourceUtf8.ToArray();
        GCHandle copiedHandle = GCHandle.Alloc(sourceBytes, GCHandleType.Pinned);
        pointer = copiedHandle.AddrOfPinnedObject();
        return copiedHandle;
    }

    public void Dispose()
    {
        if (_disposed)
        {
            return;
        }

        if (_descriptorHandle.IsAllocated)
        {
            _descriptorHandle.Free();
        }

        for (int index = 0; index < _identityHandles.Length; index++)
        {
            if (_identityHandles[index].IsAllocated)
            {
                _identityHandles[index].Free();
            }

            if (_sourceHandles[index].IsAllocated)
            {
                _sourceHandles[index].Free();
            }
        }

        _disposed = true;
    }
}
