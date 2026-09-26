// Copyright (c) The NX Authors.
// Licensed under the MIT license. See LICENSE file in the project root for full license information.

using System;
using System.Buffers.Binary;
using System.Collections.Generic;
using System.Text;
using System.Text.Json;
using System.Text.Json.Serialization;
using NxLang.Nx.Interop;

namespace NxLang.Nx;

/// <summary>
/// Represents a reusable analyzed NX program artifact.
/// </summary>
/// <remarks>
/// Program artifacts can generate deterministic NX IR and evaluate supported entrypoints after their originating build
/// context has been disposed.
/// </remarks>
public sealed class NxProgramArtifact : IDisposable
{
    private readonly NxProgramArtifactSafeHandle _handle;

    private NxProgramArtifact(IntPtr handle, string fileName)
    {
        _handle = new NxProgramArtifactSafeHandle(handle);
        FileName = fileName;
    }

    /// <summary>
    /// Gets the logical file or workspace identity selected when this artifact was built.
    /// </summary>
    public string FileName { get; }

    /// <summary>
    /// Builds a reusable program artifact from NX source text.
    /// </summary>
    /// <param name="source">The NX source code to build.</param>
    /// <param name="fileName">Optional file name used for diagnostics and local import normalization.</param>
    /// <returns>A disposable program artifact handle.</returns>
    /// <exception cref="ArgumentNullException">Thrown when <paramref name="source"/> is null.</exception>
    /// <exception cref="NxEvaluationException">Thrown when building the program reports NX diagnostics.</exception>
    /// <exception cref="InvalidOperationException">Thrown when the native runtime cannot build the program.</exception>
    public static NxProgramArtifact Build(string source, string? fileName = null)
    {
        using NxLibraryRegistry registry = new();
        using NxProgramBuildContext buildContext = registry.CreateBuildContext();
        return BuildCore(source, buildContext.SafeHandle, fileName);
    }

    /// <summary>
    /// Builds a reusable program artifact from an in-memory workspace.
    /// </summary>
    /// <param name="workspace">Workspace containing source modules to analyze.</param>
    /// <param name="entryIdentity">
    /// Logical identity of the workspace module selected as the program entry module.
    /// </param>
    /// <param name="buildContext">The registry-backed build context used to resolve imported libraries.</param>
    /// <returns>A disposable program artifact handle.</returns>
    /// <exception cref="ArgumentNullException">
    /// Thrown when <paramref name="workspace"/>, <paramref name="entryIdentity"/>, or
    /// <paramref name="buildContext"/> is <see langword="null"/>.
    /// </exception>
    /// <exception cref="ArgumentException">Thrown when <paramref name="entryIdentity"/> is empty.</exception>
    /// <exception cref="NxEvaluationException">
    /// Thrown when building the workspace program reports NX diagnostics, including a missing entry identity.
    /// </exception>
    /// <exception cref="ObjectDisposedException">
    /// Thrown when <paramref name="buildContext"/> has already been disposed.
    /// </exception>
    /// <exception cref="InvalidOperationException">Thrown when the native runtime cannot build the program.</exception>
    public static NxProgramArtifact BuildWorkspace(
        NxWorkspace workspace,
        string entryIdentity,
        NxProgramBuildContext buildContext,
        IReadOnlyList<string>? implicitImports = null)
    {
        ArgumentNullException.ThrowIfNull(buildContext);
        return BuildWorkspaceCore(workspace, entryIdentity, buildContext.SafeHandle, implicitImports);
    }

    /// <summary>
    /// Builds a reusable program artifact from NX source text against a preloaded build context.
    /// </summary>
    /// <param name="source">The NX source code to build.</param>
    /// <param name="buildContext">The registry-backed build context used to resolve imported libraries.</param>
    /// <param name="fileName">Optional file name used for diagnostics and local import normalization.</param>
    /// <returns>A disposable program artifact handle.</returns>
    /// <exception cref="ArgumentNullException">
    /// Thrown when <paramref name="source"/> or <paramref name="buildContext"/> is <see langword="null"/>.
    /// </exception>
    /// <exception cref="NxEvaluationException">Thrown when building the program reports NX diagnostics.</exception>
    /// <exception cref="ObjectDisposedException">
    /// Thrown when <paramref name="buildContext"/> has already been disposed.
    /// </exception>
    /// <exception cref="InvalidOperationException">Thrown when the native runtime cannot build the program.</exception>
    public static NxProgramArtifact Build(string source, NxProgramBuildContext buildContext, string? fileName = null)
    {
        ArgumentNullException.ThrowIfNull(buildContext);
        return BuildCore(source, buildContext.SafeHandle, fileName);
    }

    private static NxProgramArtifact BuildCore(
        string source,
        NxProgramBuildContextSafeHandle? buildContextHandle,
        string? fileName)
    {
        ArgumentNullException.ThrowIfNull(source);

        NxNativeLibrary.EnsureLoaded();

        byte[] sourceBytes = Encoding.UTF8.GetBytes(source);
        byte[] fileNameBytes = fileName is null ? Array.Empty<byte>() : Encoding.UTF8.GetBytes(fileName);
        IntPtr handle = IntPtr.Zero;

        try
        {
            NxEvalStatus status = NxNativeMethods.nx_build_program_artifact(
                buildContextHandle,
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

    private static NxProgramArtifact BuildWorkspaceCore(
        NxWorkspace workspace,
        string entryIdentity,
        NxProgramBuildContextSafeHandle? buildContextHandle,
        IReadOnlyList<string>? implicitImports)
    {
        ArgumentNullException.ThrowIfNull(workspace);
        ArgumentNullException.ThrowIfNull(entryIdentity);
        if (entryIdentity.Length == 0)
        {
            throw new ArgumentException("Workspace entry identity must not be empty.", nameof(entryIdentity));
        }

        NxNativeLibrary.EnsureLoaded();

        byte[] entryIdentityBytes = Encoding.UTF8.GetBytes(entryIdentity);
        IntPtr handle = IntPtr.Zero;

        try
        {
            using NxWorkspaceDescriptorScope descriptors = new(workspace);
            using NxUtf8SliceScope implicitImportSlices = new(implicitImports);
            NxEvalStatus status = NxNativeMethods.nx_build_workspace_program_artifact(
                buildContextHandle,
                descriptors.Pointer,
                descriptors.Count,
                entryIdentityBytes,
                (nuint)entryIdentityBytes.Length,
                implicitImportSlices.Pointer,
                implicitImportSlices.Count,
                out handle,
                out NxBuffer buffer);

            byte[] payload = NxRuntime.CopyAndFreeBuffer(buffer);

            return status switch
            {
                NxEvalStatus.Ok when handle != IntPtr.Zero => new NxProgramArtifact(handle, entryIdentity),
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
    /// Generates a host-neutral JavaScript program module from this reusable program artifact.
    /// </summary>
    /// <param name="options">Optional program-module codegen options.</param>
    /// <returns>Generated JavaScript source text and structured metadata.</returns>
    /// <exception cref="NxEvaluationException">
    /// Thrown when program-module generation reports NX diagnostics.
    /// </exception>
    /// <exception cref="ObjectDisposedException">Thrown when this artifact has already been disposed.</exception>
    /// <exception cref="InvalidOperationException">
    /// Thrown when the native runtime returns an invalid generated program-module payload.
    /// </exception>
    public NxGeneratedJSProgramModule GenerateJSProgramModule(NxJSProgramModuleOptions? options = null)
    {
        NxNativeLibrary.EnsureLoaded();

        string? logicalModuleName = options?.LogicalModuleName;
        string? runtimeImportSpecifier = options?.RuntimeImportSpecifier;
        byte[] logicalModuleNameBytes = string.IsNullOrEmpty(logicalModuleName)
            ? Array.Empty<byte>()
            : Encoding.UTF8.GetBytes(logicalModuleName);
        byte[] runtimeImportSpecifierBytes = string.IsNullOrEmpty(runtimeImportSpecifier)
            ? Array.Empty<byte>()
            : Encoding.UTF8.GetBytes(runtimeImportSpecifier);

        NxEvalStatus status = NxNativeMethods.nx_codegen_js_program_module(
            SafeHandle,
            logicalModuleNameBytes,
            (nuint)logicalModuleNameBytes.Length,
            runtimeImportSpecifierBytes,
            (nuint)runtimeImportSpecifierBytes.Length,
            out NxBuffer buffer);
        byte[] payload = NxRuntime.CopyAndFreeBuffer(buffer);

        return status switch
        {
            NxEvalStatus.Ok => DeserializeGeneratedJSProgramModule(payload),
            NxEvalStatus.Error => throw NxRuntime.CreateEvaluationExceptionFromJson(payload),
            _ => throw NxRuntime.CreateInteropStatusException(status),
        };
    }

    /// <summary>
    /// Generates the NX IR artifact of this program's entry module, compact and without its debug
    /// section.
    /// </summary>
    /// <returns>The NX IR image and structured metadata.</returns>
    /// <exception cref="NxEvaluationException">Thrown when IR generation reports NX diagnostics.</exception>
    /// <exception cref="ObjectDisposedException">Thrown when this artifact has already been disposed.</exception>
    /// <exception cref="InvalidOperationException">
    /// Thrown when the native runtime returns an invalid generated NX IR payload.
    /// </exception>
    public NxGeneratedNxIr GenerateNxIr()
    {
        IReadOnlyList<NxGeneratedNxIr> artifacts = GenerateNxIr(new NxIrEmitOptions());
        if (artifacts.Count == 0)
        {
            throw new InvalidOperationException("NX native runtime returned no NX IR artifact for the entry module.");
        }

        return artifacts[0];
    }

    /// <summary>
    /// Generates NX IR artifacts for the modules <paramref name="options"/> names, the entry alone
    /// by default, one image per module.
    /// </summary>
    /// <param name="options">Which modules to emit, and whether to include debug data.</param>
    /// <returns>One artifact per emitted module, in the order requested.</returns>
    /// <exception cref="ArgumentNullException">
    /// Thrown when <paramref name="options"/> is <see langword="null"/>.
    /// </exception>
    /// <exception cref="NxEvaluationException">Thrown when IR generation reports NX diagnostics.</exception>
    /// <exception cref="ObjectDisposedException">Thrown when this artifact has already been disposed.</exception>
    /// <exception cref="InvalidOperationException">
    /// Thrown when the native runtime returns an invalid generated NX IR payload.
    /// </exception>
    public IReadOnlyList<NxGeneratedNxIr> GenerateNxIr(NxIrEmitOptions options)
    {
        ArgumentNullException.ThrowIfNull(options);
        NxNativeLibrary.EnsureLoaded();

        byte[] optionsBytes = JsonSerializer.SerializeToUtf8Bytes(options);
        NxEvalStatus status = NxNativeMethods.nx_codegen_nx_ir(
            SafeHandle,
            optionsBytes,
            (nuint)optionsBytes.Length,
            out NxBuffer buffer);
        byte[] payload = NxRuntime.CopyAndFreeBuffer(buffer);

        return status switch
        {
            NxEvalStatus.Ok => DeserializeGeneratedNxIr(payload),
            NxEvalStatus.Error => throw NxRuntime.CreateEvaluationExceptionFromJson(payload),
            _ => throw NxRuntime.CreateInteropStatusException(status),
        };
    }

    private static NxGeneratedJSProgramModule DeserializeGeneratedJSProgramModule(byte[] payload)
    {
        try
        {
            NxGeneratedJSProgramModule? module = JsonSerializer.Deserialize<NxGeneratedJSProgramModule>(payload);
            if (module is null)
            {
                throw new JsonException("Expected generated program-module payload.");
            }

            return module;
        }
        catch (JsonException e)
        {
            throw new InvalidOperationException(
                "NX native runtime returned an invalid generated program-module JSON payload.",
                e);
        }
    }

    /// <summary>
    /// Splits an NX IR bundle into its artifacts. The bundle is a little-endian 32-bit header length, a
    /// JSON header of <c>[{ identity, metadata, offset, length }]</c>, zero padding to a four-byte boundary,
    /// then the images at the offsets the header gives, measured from the start of the payload.
    /// </summary>
    private static IReadOnlyList<NxGeneratedNxIr> DeserializeGeneratedNxIr(byte[] payload)
    {
        try
        {
            if (payload.Length < 4)
            {
                throw new JsonException("Expected an NX IR bundle header length.");
            }

            uint headerLength = BinaryPrimitives.ReadUInt32LittleEndian(payload.AsSpan(0, 4));
            if (headerLength > (uint)(payload.Length - 4))
            {
                throw new JsonException("The NX IR bundle header does not fit its payload.");
            }

            NxIrBundleEntry[]? entries = JsonSerializer.Deserialize<NxIrBundleEntry[]>(
                payload.AsSpan(4, (int)headerLength));
            if (entries is null)
            {
                throw new JsonException("Expected an NX IR bundle header.");
            }

            List<NxGeneratedNxIr> artifacts = new(entries.Length);
            foreach (NxIrBundleEntry entry in entries)
            {
                // Both fit an int once the end is inside the payload, so the casts below cannot overflow.
                long end = (long)entry.Offset + entry.Length;
                if (end > payload.Length)
                {
                    throw new JsonException($"The image of '{entry.Identity}' lies outside its bundle.");
                }

                artifacts.Add(new NxGeneratedNxIr
                {
                    Identity = entry.Identity,
                    Bytes = payload.AsSpan((int)entry.Offset, (int)entry.Length).ToArray(),
                    Metadata = entry.Metadata,
                });
            }

            return artifacts;
        }
        catch (JsonException e)
        {
            throw new InvalidOperationException(
                "NX native runtime returned an invalid generated NX IR payload.",
                e);
        }
    }

    /// <summary>
    /// One entry of an NX IR bundle's header.
    /// </summary>
    private sealed class NxIrBundleEntry
    {
        [JsonPropertyName("identity")]
        public string Identity { get; init; } = string.Empty;

        [JsonPropertyName("metadata")]
        public NxIrMetadata Metadata { get; init; } = new();

        [JsonPropertyName("offset")]
        public uint Offset { get; init; }

        [JsonPropertyName("length")]
        public uint Length { get; init; }
    }

    /// <summary>
    /// Releases the native program-artifact handle.
    /// </summary>
    /// <remarks>Calling <see cref="Dispose()"/> more than once is allowed.</remarks>
    public void Dispose()
    {
        _handle.Dispose();
    }

    internal NxProgramArtifactSafeHandle SafeHandle
    {
        get
        {
            ObjectDisposedException.ThrowIf(_handle.IsClosed || _handle.IsInvalid, this);
            return _handle;
        }
    }
}
