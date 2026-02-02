// Copyright (c) Bret Johnson. All rights reserved.
// Licensed under the MIT license. See LICENSE file in the project root for full license information.

using System;
using System.Runtime.InteropServices;
using System.Text;
using System.Text.Json;
using MessagePack;

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
    /// <exception cref="InvalidOperationException">Thrown when the NX native library (nx_ffi) cannot be found.</exception>
    public static byte[] EvaluateToMessagePack(string source, string? fileName = null)
    {
        if (source is null)
        {
            throw new ArgumentNullException(nameof(source));
        }

        try
        {
            byte[] sourceBytes = Encoding.UTF8.GetBytes(source);
            byte[] fileNameBytes = fileName is null ? Array.Empty<byte>() : Encoding.UTF8.GetBytes(fileName);

            NxEvalStatus status = NxNativeMethods.nx_eval_source_msgpack(
                sourceBytes,
                (nuint)sourceBytes.Length,
                fileNameBytes,
                (nuint)fileNameBytes.Length,
                out NxBuffer buffer);

            if (status is NxEvalStatus.Ok)
            {
                return CopyAndFreeBuffer(buffer);
            }

            if (status is NxEvalStatus.Error)
            {
                byte[] diagBytes = CopyAndFreeBuffer(buffer);
                NxDiagnostic[] diagnostics = MessagePackSerializer.Deserialize<NxDiagnostic[]>(
                    diagBytes,
                    MessagePackOptions);
                throw new NxEvaluationException("NX evaluation failed.", diagnostics);
            }

            CopyAndFreeBuffer(buffer);
            throw new NxEvaluationException($"NX evaluation failed with status: {status}.", Array.Empty<NxDiagnostic>());
        }
        catch (DllNotFoundException e)
        {
            throw new InvalidOperationException(
                "NX native library (nx_ffi) was not found. Build `crates/nx-ffi` as a cdylib and ensure it is discoverable via PATH/LD_LIBRARY_PATH/DYLD_LIBRARY_PATH or alongside the process.",
                e);
        }
    }

    /// <summary>
    /// Evaluates NX source code and returns the result as a JSON string.
    /// </summary>
    /// <param name="source">The NX source code to evaluate. Must contain a root() function that returns the result.</param>
    /// <param name="fileName">Optional file name for diagnostic messages. Used in error reporting to identify the source location.</param>
    /// <returns>The evaluation result serialized as a JSON string.</returns>
    /// <exception cref="ArgumentNullException">Thrown when <paramref name="source"/> is null.</exception>
    /// <exception cref="NxEvaluationException">Thrown when evaluation fails due to syntax errors, missing root function, or runtime errors.</exception>
    /// <exception cref="InvalidOperationException">Thrown when the NX native library (nx_ffi) cannot be found.</exception>
    public static string EvaluateToJson(string source, string? fileName = null)
    {
        if (source is null)
        {
            throw new ArgumentNullException(nameof(source));
        }

        try
        {
            byte[] sourceBytes = Encoding.UTF8.GetBytes(source);
            byte[] fileNameBytes = fileName is null ? Array.Empty<byte>() : Encoding.UTF8.GetBytes(fileName);

            NxEvalStatus status = NxNativeMethods.nx_eval_source_json(
                sourceBytes,
                (nuint)sourceBytes.Length,
                fileNameBytes,
                (nuint)fileNameBytes.Length,
                out NxBuffer buffer);

            string json = Encoding.UTF8.GetString(CopyAndFreeBuffer(buffer));

            if (status is NxEvalStatus.Ok)
            {
                return json;
            }

            if (status is NxEvalStatus.Error)
            {
                NxDiagnostic[] diagnostics = JsonSerializer.Deserialize<NxDiagnostic[]>(json)
                    ?? Array.Empty<NxDiagnostic>();
                throw new NxEvaluationException("NX evaluation failed.", diagnostics);
            }

            throw new NxEvaluationException($"NX evaluation failed with status: {status}.", Array.Empty<NxDiagnostic>());
        }
        catch (DllNotFoundException e)
        {
            throw new InvalidOperationException(
                "NX native library (nx_ffi) was not found. Build `crates/nx-ffi` as a cdylib and ensure it is discoverable via PATH/LD_LIBRARY_PATH/DYLD_LIBRARY_PATH or alongside the process.",
                e);
        }
    }

    /// <summary>
    /// Evaluates NX source code and deserializes the result to the specified type.
    /// </summary>
    /// <typeparam name="T">The type to deserialize the result to. Must be compatible with MessagePack serialization.</typeparam>
    /// <param name="source">The NX source code to evaluate. Must contain a root() function that returns the result.</param>
    /// <param name="fileName">Optional file name for diagnostic messages. Used in error reporting to identify the source location.</param>
    /// <param name="options">Optional MessagePack serialization options. If null, uses default options with untrusted data security.</param>
    /// <returns>The evaluation result deserialized to type <typeparamref name="T"/>.</returns>
    /// <exception cref="ArgumentNullException">Thrown when <paramref name="source"/> is null.</exception>
    /// <exception cref="NxEvaluationException">Thrown when evaluation fails due to syntax errors, missing root function, or runtime errors.</exception>
    /// <exception cref="InvalidOperationException">Thrown when the NX native library (nx_ffi) cannot be found.</exception>
    /// <exception cref="MessagePackSerializationException">Thrown when deserialization to type <typeparamref name="T"/> fails.</exception>
    /// <example>
    /// <code>
    /// int result = NxRuntime.Evaluate&lt;int&gt;("let root() = { 42 }");
    /// string text = NxRuntime.Evaluate&lt;string&gt;("let root() = { \"Hello, NX!\" }");
    /// </code>
    /// </example>
    public static T Evaluate<T>(string source, string? fileName = null, MessagePackSerializerOptions? options = null)
    {
        byte[] bytes = EvaluateToMessagePack(source, fileName);
        return MessagePackSerializer.Deserialize<T>(bytes, options ?? MessagePackOptions);
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
