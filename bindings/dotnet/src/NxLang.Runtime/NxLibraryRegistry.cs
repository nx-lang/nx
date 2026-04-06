// Copyright (c) Bret Johnson. All rights reserved.
// Licensed under the MIT license. See LICENSE file in the project root for full license information.

using System;
using System.IO;
using System.Text;
using NxLang.Nx.Interop;

namespace NxLang.Nx;

/// <summary>
/// Represents a reusable registry of analyzed NX library snapshots.
/// </summary>
public sealed class NxLibraryRegistry : IDisposable
{
    private readonly NxLibraryRegistrySafeHandle _handle;

    /// <summary>
    /// Creates an empty reusable library registry.
    /// </summary>
    public NxLibraryRegistry()
    {
        NxNativeLibrary.EnsureLoaded();

        NxEvalStatus status = NxNativeMethods.nx_create_library_registry(out IntPtr handle);
        _handle = status switch
        {
            NxEvalStatus.Ok when handle != IntPtr.Zero => new NxLibraryRegistrySafeHandle(handle),
            NxEvalStatus.Ok => throw new InvalidOperationException(
                "NX native runtime returned success without a library registry handle."),
            _ => throw NxRuntime.CreateInteropStatusException(status),
        };
    }

    /// <summary>
    /// Loads and analyzes a local NX library root into this registry.
    /// </summary>
    /// <param name="rootPath">The directory containing one NX library root.</param>
    /// <exception cref="ArgumentNullException">Thrown when <paramref name="rootPath"/> is null.</exception>
    /// <exception cref="ArgumentException">Thrown when <paramref name="rootPath"/> is empty or whitespace.</exception>
    /// <exception cref="NxEvaluationException">Thrown when loading the library reports NX diagnostics.</exception>
    /// <exception cref="InvalidOperationException">Thrown when the native runtime cannot load the library.</exception>
    public void LoadFromDirectory(string rootPath)
    {
        ArgumentNullException.ThrowIfNull(rootPath);
        if (string.IsNullOrWhiteSpace(rootPath))
        {
            throw new ArgumentException("Library root path cannot be empty.", nameof(rootPath));
        }

        NxNativeLibrary.EnsureLoaded();

        string normalizedRootPath = Path.GetFullPath(rootPath);
        byte[] rootPathBytes = Encoding.UTF8.GetBytes(normalizedRootPath);

        NxEvalStatus status = NxNativeMethods.nx_load_library_into_registry(
            SafeHandle,
            rootPathBytes,
            (nuint)rootPathBytes.Length,
            out NxBuffer buffer);

        byte[] payload = NxRuntime.CopyAndFreeBuffer(buffer);
        switch (status)
        {
            case NxEvalStatus.Ok:
                return;
            case NxEvalStatus.Error:
                throw NxRuntime.CreateEvaluationExceptionFromMessagePack(payload);
            default:
                throw NxRuntime.CreateInteropStatusException(status);
        }
    }

    /// <summary>
    /// Creates a reusable program build context backed by this registry.
    /// </summary>
    public NxProgramBuildContext CreateBuildContext()
    {
        return NxProgramBuildContext.Create(this);
    }

    /// <summary>
    /// Releases the native library-registry handle.
    /// </summary>
    public void Dispose()
    {
        _handle.Dispose();
    }

    internal NxLibraryRegistrySafeHandle SafeHandle
    {
        get
        {
            ObjectDisposedException.ThrowIf(_handle.IsClosed || _handle.IsInvalid, this);
            return _handle;
        }
    }
}
