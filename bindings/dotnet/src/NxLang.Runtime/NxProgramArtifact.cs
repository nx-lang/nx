// Copyright (c) Bret Johnson. All rights reserved.
// Licensed under the MIT license. See LICENSE file in the project root for full license information.

using System;
using System.Text;
using System.Threading;
using NxLang.Nx.Interop;

namespace NxLang.Nx;

/// <summary>
/// Represents a reusable NX program artifact built from source text.
/// </summary>
public sealed class NxProgramArtifact : IDisposable
{
    private IntPtr _handle;

    private NxProgramArtifact(IntPtr handle, string fileName)
    {
        _handle = handle;
        FileName = fileName;
    }

    /// <summary>
    /// Gets the file name identity used to build this program artifact.
    /// </summary>
    public string FileName { get; }

    /// <summary>
    /// Builds a reusable program artifact from NX source text.
    /// </summary>
    /// <param name="source">The NX source code to build.</param>
    /// <param name="fileName">Optional file name used for diagnostics and local import resolution.</param>
    /// <returns>A disposable program artifact handle.</returns>
    /// <exception cref="ArgumentNullException">Thrown when <paramref name="source"/> is null.</exception>
    /// <exception cref="NxEvaluationException">Thrown when building the program reports NX diagnostics.</exception>
    /// <exception cref="InvalidOperationException">Thrown when the native runtime cannot build the program.</exception>
    public static NxProgramArtifact Build(string source, string? fileName = null)
    {
        ArgumentNullException.ThrowIfNull(source);

        NxNativeLibrary.EnsureLoaded();

        byte[] sourceBytes = Encoding.UTF8.GetBytes(source);
        byte[] fileNameBytes = fileName is null ? Array.Empty<byte>() : Encoding.UTF8.GetBytes(fileName);
        IntPtr handle = IntPtr.Zero;

        try
        {
            NxEvalStatus status = NxNativeMethods.nx_build_program_artifact(
                sourceBytes,
                (nuint)sourceBytes.Length,
                fileNameBytes,
                (nuint)fileNameBytes.Length,
                out handle,
                out NxBuffer buffer);

            byte[] payload = NxRuntime.CopyAndFreeBuffer(buffer);
            string normalizedFileName = string.IsNullOrEmpty(fileName) ? "input.nx" : fileName;

            return status switch
            {
                NxEvalStatus.Ok when handle != IntPtr.Zero => new NxProgramArtifact(handle, normalizedFileName),
                NxEvalStatus.Ok => throw new InvalidOperationException(
                    "NX native runtime returned success without a program artifact handle."),
                NxEvalStatus.Error => throw NxRuntime.CreateEvaluationExceptionFromMessagePack(payload),
                _ => throw NxRuntime.CreateInteropStatusException(status),
            };
        }
        catch
        {
            if (handle != IntPtr.Zero)
            {
                NxNativeMethods.nx_free_program_artifact(handle);
            }

            throw;
        }
    }

    /// <summary>
    /// Releases the native program-artifact handle.
    /// </summary>
    public void Dispose()
    {
        ReleaseHandle();
        GC.SuppressFinalize(this);
    }

    internal IntPtr DangerousGetHandle()
    {
        IntPtr handle = Interlocked.CompareExchange(ref _handle, IntPtr.Zero, IntPtr.Zero);
        ObjectDisposedException.ThrowIf(handle == IntPtr.Zero, this);
        return handle;
    }

    ~NxProgramArtifact()
    {
        ReleaseHandle();
    }

    private void ReleaseHandle()
    {
        IntPtr handle = Interlocked.Exchange(ref _handle, IntPtr.Zero);
        if (handle == IntPtr.Zero)
        {
            return;
        }

        NxNativeMethods.nx_free_program_artifact(handle);
    }
}
