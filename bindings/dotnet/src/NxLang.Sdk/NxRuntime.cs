// Copyright (c) The NX Authors.
// Licensed under the MIT license. See LICENSE file in the project root for full license information.

using System;
using System.Collections.Generic;
using System.Runtime.InteropServices;
using System.Text;
using System.Text.Json;
using MessagePack;
using NxLang.Nx.Interop;

namespace NxLang.Nx;

/// <summary>
/// Provides convenience methods for validating, building, generating IR from, and evaluating NX source through the
/// native NX SDK.
/// </summary>
public static class NxRuntime
{
    private static readonly MessagePackSerializerOptions MessagePackOptions =
        MessagePackSerializerOptions.Standard.WithSecurity(MessagePackSecurity.UntrustedData);





    /// <summary>
    /// Validates an in-memory workspace against a supplied build context and returns all NX diagnostics as data.
    /// </summary>
    /// <param name="workspace">Workspace containing source modules to validate.</param>
    /// <param name="buildContext">
    /// Build context used to resolve libraries that are not provided by workspace modules.
    /// </param>
    /// <param name="implicitImports">
    /// Workspace identities every other module imports implicitly, as if it began with a wildcard
    /// import of each; a listed module imports nothing implicitly itself. An identity the workspace
    /// does not hold is reported as a diagnostic naming it.
    /// </param>
    /// <returns>All diagnostics reported by NX validation, or an empty list when the workspace is valid.</returns>
    /// <exception cref="ArgumentNullException">
    /// Thrown when <paramref name="workspace"/> or <paramref name="buildContext"/> is <see langword="null"/>.
    /// </exception>
    /// <exception cref="ObjectDisposedException">
    /// Thrown when <paramref name="buildContext"/> has already been disposed.
    /// </exception>
    /// <exception cref="InvalidOperationException">
    /// Thrown when the native runtime cannot validate the workspace.
    /// </exception>
    public static IReadOnlyList<NxDiagnostic> ValidateWorkspace(
        NxWorkspace workspace,
        NxProgramBuildContext buildContext,
        IReadOnlyList<string>? implicitImports = null)
    {
        ArgumentNullException.ThrowIfNull(workspace);
        ArgumentNullException.ThrowIfNull(buildContext);

        NxNativeLibrary.EnsureLoaded();

        using NxWorkspaceDescriptorScope descriptors = new(workspace);
        using NxUtf8SliceScope implicitImportSlices = new(implicitImports);
        NxEvalStatus status = NxNativeMethods.nx_validate_workspace(
            buildContext.SafeHandle,
            descriptors.Pointer,
            descriptors.Count,
            implicitImportSlices.Pointer,
            implicitImportSlices.Count,
            out NxBuffer buffer);
        byte[] payload = CopyAndFreeBuffer(buffer);

        return status switch
        {
            NxEvalStatus.Ok => DeserializeMessagePackDiagnostics(payload),
            _ => throw CreateInteropStatusException(status),
        };
    }









    /// <summary>
    /// Builds a short-lived source artifact, generates its NX IR image and metadata, then disposes the artifact.
    /// </summary>
    /// <param name="source">The NX source code to compile into NX IR.</param>
    /// <param name="fileName">Optional file name used for diagnostics and local import normalization.</param>
    /// <returns>The NX IR image and structured metadata.</returns>
    /// <exception cref="ArgumentNullException">
    /// Thrown when <paramref name="source"/> is <see langword="null"/>.
    /// </exception>
    /// <exception cref="NxEvaluationException">Thrown when build or IR generation reports NX diagnostics.</exception>
    /// <exception cref="InvalidOperationException">
    /// Thrown when the native runtime returns invalid IR metadata.
    /// </exception>
    public static NxGeneratedNxIr GenerateNxIr(string source, string? fileName = null)
    {
        using NxProgramArtifact programArtifact = NxProgramArtifact.Build(source, fileName);
        return programArtifact.GenerateNxIr();
    }

    /// <summary>
    /// Builds a short-lived source artifact against a caller-supplied build context, generates its NX IR image and
    /// metadata, then disposes the artifact.
    /// </summary>
    /// <param name="source">The NX source code to compile into NX IR.</param>
    /// <param name="buildContext">Build context used to resolve preloaded libraries.</param>
    /// <param name="fileName">Optional file name used for diagnostics and local import normalization.</param>
    /// <returns>The NX IR image and structured metadata.</returns>
    /// <exception cref="ArgumentNullException">
    /// Thrown when <paramref name="source"/> or <paramref name="buildContext"/> is <see langword="null"/>.
    /// </exception>
    /// <exception cref="NxEvaluationException">Thrown when build or IR generation reports NX diagnostics.</exception>
    /// <exception cref="ObjectDisposedException">
    /// Thrown when <paramref name="buildContext"/> has already been disposed.
    /// </exception>
    /// <exception cref="InvalidOperationException">
    /// Thrown when the native runtime returns invalid IR metadata.
    /// </exception>
    public static NxGeneratedNxIr GenerateNxIr(
        string source,
        NxProgramBuildContext buildContext,
        string? fileName = null)
    {
        ArgumentNullException.ThrowIfNull(buildContext);

        using NxProgramArtifact programArtifact = NxProgramArtifact.Build(source, buildContext, fileName);
        return programArtifact.GenerateNxIr();
    }

    /// <summary>
    /// Renders an NX IR image as text with every table index resolved: the same text <c>nxlang ir explain</c>
    /// prints.
    /// </summary>
    /// <param name="image">The NX IR image.</param>
    /// <returns>The explained text.</returns>
    /// <exception cref="ArgumentNullException">
    /// Thrown when <paramref name="image"/> is <see langword="null"/>.
    /// </exception>
    /// <exception cref="NxEvaluationException">
    /// Thrown when the bytes are not an NX IR image this build reads, with a diagnostic saying why.
    /// </exception>
    public static string ExplainNxIr(byte[] image)
    {
        ArgumentNullException.ThrowIfNull(image);
        NxNativeLibrary.EnsureLoaded();

        NxEvalStatus status = NxNativeMethods.nx_ir_explain(image, (nuint)image.Length, out NxBuffer buffer);
        byte[] payload = CopyAndFreeBuffer(buffer);

        return status switch
        {
            NxEvalStatus.Ok => Encoding.UTF8.GetString(payload),
            NxEvalStatus.Error => throw CreateEvaluationExceptionFromJson(payload),
            _ => throw CreateInteropStatusException(status),
        };
    }





















































    internal static NxEvaluationException CreateEvaluationExceptionFromMessagePack(byte[] payload)
    {
        try
        {
            NxDiagnostic[] diagnostics = MessagePackSerializer.Deserialize<NxDiagnostic[]>(payload, MessagePackOptions);
            return new NxEvaluationException("NX evaluation failed.", diagnostics);
        }
        catch (MessagePackSerializationException e)
        {
            throw new InvalidOperationException(
                "NX native runtime returned an invalid MessagePack diagnostics payload.",
                e);
        }
    }

    private static NxDiagnostic[] DeserializeMessagePackDiagnostics(byte[] payload)
    {
        try
        {
            return MessagePackSerializer.Deserialize<NxDiagnostic[]>(payload, MessagePackOptions);
        }
        catch (MessagePackSerializationException e)
        {
            throw new InvalidOperationException(
                "NX native runtime returned an invalid MessagePack diagnostics payload.",
                e);
        }
    }

    internal static NxEvaluationException CreateEvaluationExceptionFromJson(byte[] payload)
    {
        try
        {
            NxDiagnostic[]? diagnostics = JsonSerializer.Deserialize<NxDiagnostic[]>(payload);
            if (diagnostics is null)
            {
                throw new JsonException("Expected JSON diagnostics payload.");
            }

            return new NxEvaluationException("NX evaluation failed.", diagnostics);
        }
        catch (JsonException e)
        {
            throw new InvalidOperationException(
                "NX native runtime returned an invalid JSON diagnostics payload.",
                e);
        }
    }






    internal static InvalidOperationException CreateInteropStatusException(NxEvalStatus status)
    {
        string message = status switch
        {
            NxEvalStatus.InvalidArgument =>
                "NX native runtime rejected the evaluation request because the interop arguments were invalid.",
            NxEvalStatus.Panic =>
                "NX native runtime panicked while processing the evaluation request.",
            _ =>
                $"NX native runtime returned an unexpected status code: {status}.",
        };

        return new InvalidOperationException(message);
    }

    internal static byte[] CopyAndFreeBuffer(NxBuffer buffer)
    {
        try
        {
            if (buffer.Ptr == IntPtr.Zero)
            {
                return Array.Empty<byte>();
            }

            int length = checked((int)(nuint)buffer.Len);
            byte[] result = new byte[length];
            Marshal.Copy(buffer.Ptr, result, 0, length);
            return result;
        }
        finally
        {
            NxNativeMethods.nx_free_buffer(buffer);
        }
    }
}
