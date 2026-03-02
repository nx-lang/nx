// Copyright (c) Bret Johnson. All rights reserved.
// Licensed under the MIT license. See LICENSE file in the project root for full license information.

using System;
using System.Runtime.InteropServices;
using System.Text;
using System.Text.Json;
using MessagePack;
using NxLang.Nx.Interop;

namespace NxLang.Nx;

/// <summary>
/// Provides methods for evaluating NX language source code and returning results in various formats.
/// </summary>
public static class NxRuntime
{
    private static readonly MessagePackSerializerOptions MessagePackOptions =
        MessagePackSerializerOptions.Standard.WithSecurity(MessagePackSecurity.UntrustedData);

    /// <summary>
    /// Evaluates NX source code and returns the result as MessagePack-serialized bytes.
    /// </summary>
    /// <param name="source">The NX source code to evaluate. Must contain a root() function that returns the result.</param>
    /// <param name="fileName">Optional file name for diagnostic messages. Used in error reporting to identify the source location.</param>
    /// <returns>The evaluation result serialized as MessagePack bytes.</returns>
    /// <exception cref="ArgumentNullException">Thrown when <paramref name="source"/> is null.</exception>
    /// <exception cref="NxEvaluationException">Thrown when evaluation fails due to syntax errors, missing root function, or runtime errors.</exception>
    /// <exception cref="InvalidOperationException">Thrown when the NX native runtime cannot be loaded or is incompatible.</exception>
    public static byte[] EvaluateToMessagePack(string source, string? fileName = null)
    {
        byte[] payload = InvokeNativeCall(source, fileName, NxNativeMethods.nx_eval_source_msgpack, out NxEvalStatus status);

        return status switch
        {
            NxEvalStatus.Ok => payload,
            NxEvalStatus.Error => throw CreateEvaluationExceptionFromMessagePack(payload),
            _ => throw CreateInteropStatusException(status),
        };
    }

    /// <summary>
    /// Evaluates NX source code and returns the result as a JSON string.
    /// </summary>
    /// <param name="source">The NX source code to evaluate. Must contain a root() function that returns the result.</param>
    /// <param name="fileName">Optional file name for diagnostic messages. Used in error reporting to identify the source location.</param>
    /// <returns>The evaluation result serialized as a JSON string.</returns>
    /// <exception cref="ArgumentNullException">Thrown when <paramref name="source"/> is null.</exception>
    /// <exception cref="NxEvaluationException">Thrown when evaluation fails due to syntax errors, missing root function, or runtime errors.</exception>
    /// <exception cref="InvalidOperationException">Thrown when the NX native runtime cannot be loaded or is incompatible.</exception>
    public static string EvaluateToJson(string source, string? fileName = null)
    {
        byte[] payload = InvokeNativeCall(source, fileName, NxNativeMethods.nx_eval_source_json, out NxEvalStatus status);
        string json = Encoding.UTF8.GetString(payload);

        return status switch
        {
            NxEvalStatus.Ok => json,
            NxEvalStatus.Error => throw CreateEvaluationExceptionFromJson(json),
            _ => throw CreateInteropStatusException(status),
        };
    }

    /// <summary>
    /// Evaluates NX source code and deserializes the result to the specified type.
    /// </summary>
    /// <typeparam name="T">The type to deserialize the result to. Must be compatible with MessagePack serialization.</typeparam>
    /// <param name="source">The NX source code to evaluate. Must contain a root() function that returns the result.</param>
    /// <param name="fileName">Optional file name for diagnostic messages. Used in error reporting to identify the source location.</param>
    /// <returns>The evaluation result deserialized to type <typeparamref name="T"/>.</returns>
    /// <exception cref="ArgumentNullException">Thrown when <paramref name="source"/> is null.</exception>
    /// <exception cref="NxEvaluationException">Thrown when evaluation fails due to syntax errors, missing root function, or runtime errors.</exception>
    /// <exception cref="InvalidOperationException">Thrown when the NX native runtime cannot be loaded or is incompatible.</exception>
    /// <exception cref="MessagePackSerializationException">Thrown when deserialization to type <typeparamref name="T"/> fails.</exception>
    public static T Evaluate<T>(string source, string? fileName = null)
    {
        byte[] bytes = EvaluateToMessagePack(source, fileName);
        return MessagePackSerializer.Deserialize<T>(bytes, MessagePackOptions);
    }

    private delegate NxEvalStatus EvalSourceCallback(
        byte[] sourceBytes,
        nuint sourceLength,
        byte[] fileNameBytes,
        nuint fileNameLength,
        out NxBuffer buffer);

    private static byte[] InvokeNativeCall(
        string source,
        string? fileName,
        EvalSourceCallback callback,
        out NxEvalStatus status)
    {
        ArgumentNullException.ThrowIfNull(source);

        NxNativeLibrary.EnsureLoaded();

        byte[] sourceBytes = Encoding.UTF8.GetBytes(source);
        byte[] fileNameBytes = fileName is null ? Array.Empty<byte>() : Encoding.UTF8.GetBytes(fileName);

        status = callback(
            sourceBytes,
            (nuint)sourceBytes.Length,
            fileNameBytes,
            (nuint)fileNameBytes.Length,
            out NxBuffer buffer);

        return CopyAndFreeBuffer(buffer);
    }

    private static NxEvaluationException CreateEvaluationExceptionFromMessagePack(byte[] payload)
    {
        try
        {
            NxDiagnostic[] diagnostics = MessagePackSerializer.Deserialize<NxDiagnostic[]>(payload, MessagePackOptions);
            return new NxEvaluationException("NX evaluation failed.", diagnostics);
        }
        catch (MessagePackSerializationException e)
        {
            throw new InvalidOperationException("NX native runtime returned an invalid MessagePack diagnostics payload.", e);
        }
    }

    private static NxEvaluationException CreateEvaluationExceptionFromJson(string json)
    {
        try
        {
            NxDiagnostic[] diagnostics = JsonSerializer.Deserialize<NxDiagnostic[]>(json) ?? Array.Empty<NxDiagnostic>();
            return new NxEvaluationException("NX evaluation failed.", diagnostics);
        }
        catch (JsonException e)
        {
            throw new InvalidOperationException("NX native runtime returned an invalid JSON diagnostics payload.", e);
        }
    }

    private static InvalidOperationException CreateInteropStatusException(NxEvalStatus status)
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

    private static byte[] CopyAndFreeBuffer(NxBuffer buffer)
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
