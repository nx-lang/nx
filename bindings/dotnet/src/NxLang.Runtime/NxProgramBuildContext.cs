// Copyright (c) Bret Johnson. All rights reserved.
// Licensed under the MIT license. See LICENSE file in the project root for full license information.

using System;
using NxLang.Nx.Interop;

namespace NxLang.Nx;

/// <summary>
/// Represents a registry-backed build scope used to create transient program artifacts.
/// </summary>
public sealed class NxProgramBuildContext : IDisposable
{
    private readonly NxProgramBuildContextSafeHandle _handle;
    private readonly NxLibraryRegistry _registry;

    private NxProgramBuildContext(IntPtr handle, NxLibraryRegistry registry)
    {
        _handle = new NxProgramBuildContextSafeHandle(handle);
        _registry = registry;
    }

    internal static NxProgramBuildContext Create(NxLibraryRegistry registry)
    {
        ArgumentNullException.ThrowIfNull(registry);

        NxNativeLibrary.EnsureLoaded();

        NxEvalStatus status = NxNativeMethods.nx_create_program_build_context(
            registry.SafeHandle,
            out IntPtr handle);

        return status switch
        {
            NxEvalStatus.Ok when handle != IntPtr.Zero => new NxProgramBuildContext(handle, registry),
            NxEvalStatus.Ok => throw new InvalidOperationException(
                "NX native runtime returned success without a program build context handle."),
            _ => throw NxRuntime.CreateInteropStatusException(status),
        };
    }

    /// <summary>
    /// Releases the native program-build-context handle.
    /// </summary>
    public void Dispose()
    {
        _handle.Dispose();
    }

    internal NxProgramBuildContextSafeHandle SafeHandle
    {
        get
        {
            ObjectDisposedException.ThrowIf(_handle.IsClosed || _handle.IsInvalid, this);
            return _handle;
        }
    }
}
