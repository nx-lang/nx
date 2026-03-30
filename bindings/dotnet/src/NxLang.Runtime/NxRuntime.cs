// Copyright (c) Bret Johnson. All rights reserved.
// Licensed under the MIT license. See LICENSE file in the project root for full license information.

using System;
using System.Runtime.InteropServices;
using System.Text;
using MessagePack;
using NxLang.Nx.Interop;

namespace NxLang.Nx;

/// <summary>
/// Provides methods for evaluating NX source code and interacting with components through the
/// native MessagePack-based NX runtime.
/// </summary>
public static class NxRuntime
{
    private static readonly MessagePackSerializerOptions MessagePackOptions =
        MessagePackSerializerOptions.Standard.WithSecurity(MessagePackSecurity.UntrustedData);
    private static readonly UTF8Encoding StrictUtf8 = new(encoderShouldEmitUTF8Identifier: false, throwOnInvalidBytes: true);

    /// <summary>
    /// Evaluates NX source code and returns the raw result bytes in the canonical MessagePack wire format.
    /// </summary>
    /// <param name="source">The NX source code to evaluate. Must contain a root() function that returns the result.</param>
    /// <param name="fileName">Optional file name for diagnostic messages. Used in error reporting to identify the source location.</param>
    /// <returns>The evaluation result serialized as canonical NX bytes.</returns>
    /// <exception cref="ArgumentNullException">Thrown when <paramref name="source"/> is null.</exception>
    /// <exception cref="NxEvaluationException">Thrown when evaluation fails due to syntax errors, missing root function, or runtime errors.</exception>
    /// <exception cref="InvalidOperationException">Thrown when the NX native runtime cannot be loaded or is incompatible.</exception>
    public static byte[] EvaluateBytes(string source, string? fileName = null)
    {
        byte[] payload = InvokeNativeCall(source, fileName, NxNativeMethods.nx_eval_source, out NxEvalStatus status);

        return status switch
        {
            NxEvalStatus.Ok => payload,
            NxEvalStatus.Error => throw CreateEvaluationExceptionFromMessagePack(payload),
            _ => throw CreateInteropStatusException(status),
        };
    }

    /// <summary>
    /// Converts canonical NX value bytes into a JSON string for debugging or inspection.
    /// </summary>
    /// <param name="valueBytes">The canonical NX value bytes to convert.</param>
    /// <returns>The equivalent JSON string.</returns>
    public static string ValueBytesToJson(byte[] valueBytes)
    {
        return ConvertMessagePackPayloadToJson(
            valueBytes,
            NxNativeMethods.nx_value_msgpack_to_json,
            "NX native runtime returned an invalid evaluation MessagePack value payload.");
    }

    /// <summary>
    /// Converts canonical NX diagnostic bytes into a JSON string for debugging or inspection.
    /// </summary>
    /// <param name="diagnosticsBytes">The canonical NX diagnostic bytes to convert.</param>
    /// <returns>The equivalent JSON string.</returns>
    public static string DiagnosticsBytesToJson(byte[] diagnosticsBytes)
    {
        return ConvertMessagePackPayloadToJson(
            diagnosticsBytes,
            NxNativeMethods.nx_diagnostics_msgpack_to_json,
            "NX native runtime returned an invalid evaluation MessagePack diagnostics payload.");
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
        byte[] bytes = EvaluateBytes(source, fileName);
        return MessagePackSerializer.Deserialize<T>(bytes, MessagePackOptions);
    }

    /// <summary>
    /// Initializes a named component and returns the raw result bytes in the canonical MessagePack wire format.
    /// </summary>
    /// <param name="source">The NX source code containing the target component definition.</param>
    /// <param name="componentName">The component name to initialize.</param>
    /// <param name="propsBytes">Optional canonical NX bytes for component props encoded using the NX value model.</param>
    /// <param name="fileName">Optional file name used for diagnostics.</param>
    /// <remarks>
    /// Any returned state snapshot is source-revision-specific and must only be reused with the exact same
    /// NX source text that produced it.
    /// </remarks>
    /// <returns>The initialization result serialized as canonical NX bytes.</returns>
    public static byte[] InitializeComponentBytes(
        string source,
        string componentName,
        byte[]? propsBytes = null,
        string? fileName = null)
    {
        byte[] payload = InvokeComponentInitNativeCall(
            source,
            componentName,
            propsBytes,
            fileName,
            NxNativeMethods.nx_component_init,
            out NxEvalStatus status);

        return status switch
        {
            NxEvalStatus.Ok => payload,
            NxEvalStatus.Error => throw CreateEvaluationExceptionFromMessagePack(payload),
            _ => throw CreateInteropStatusException(status),
        };
    }

    /// <summary>
    /// Converts canonical component initialization result bytes into a JSON string for debugging or inspection.
    /// </summary>
    /// <param name="resultBytes">The canonical component initialization result bytes to convert.</param>
    /// <returns>The equivalent JSON string.</returns>
    public static string ComponentInitResultBytesToJson(byte[] resultBytes)
    {
        return ConvertMessagePackPayloadToJson(
            resultBytes,
            NxNativeMethods.nx_component_init_result_msgpack_to_json,
            "NX native runtime returned an invalid component initialization MessagePack payload.");
    }

    /// <summary>
    /// Initializes a named component using no explicit props and deserializes the rendered result.
    /// </summary>
    /// <typeparam name="TElement">The managed type for the rendered element payload.</typeparam>
    /// <param name="source">The NX source code containing the target component definition.</param>
    /// <param name="componentName">The component name to initialize.</param>
    /// <param name="fileName">Optional file name used for diagnostics.</param>
    /// <returns>The typed component initialization result.</returns>
    public static NxComponentInitResult<TElement> InitializeComponent<TElement>(
        string source,
        string componentName,
        string? fileName = null)
    {
        byte[] payload = InitializeComponentBytes(source, componentName, null, fileName);
        return DeserializeMessagePackResult<NxComponentInitResult<TElement>>(
            payload,
            "NX native runtime returned an invalid component initialization MessagePack payload.");
    }

    /// <summary>
    /// Initializes a named component with MessagePack-serializable props and deserializes the rendered result.
    /// </summary>
    /// <typeparam name="TProps">The managed type used to serialize component props.</typeparam>
    /// <typeparam name="TElement">The managed type for the rendered element payload.</typeparam>
    /// <param name="source">The NX source code containing the target component definition.</param>
    /// <param name="componentName">The component name to initialize.</param>
    /// <param name="props">The component props to serialize using MessagePack.</param>
    /// <param name="fileName">Optional file name used for diagnostics.</param>
    /// <returns>The typed component initialization result.</returns>
    public static NxComponentInitResult<TElement> InitializeComponent<TProps, TElement>(
        string source,
        string componentName,
        TProps props,
        string? fileName = null)
    {
        byte[] propsBytes = props is null
            ? Array.Empty<byte>()
            : MessagePackSerializer.Serialize(props, MessagePackOptions);
        byte[] payload = InitializeComponentBytes(source, componentName, propsBytes, fileName);
        return DeserializeMessagePackResult<NxComponentInitResult<TElement>>(
            payload,
            "NX native runtime returned an invalid component initialization MessagePack payload.");
    }

    /// <summary>
    /// Dispatches actions against a prior component state snapshot and returns the raw result bytes in the canonical
    /// MessagePack wire format.
    /// </summary>
    /// <param name="source">The NX source code containing the component definition.</param>
    /// <param name="stateSnapshot">
    /// The opaque state snapshot returned by initialization or a prior dispatch for the exact same NX source text.
    /// </param>
    /// <param name="actionsBytes">Optional canonical NX bytes for the action list encoded using the NX value model.</param>
    /// <param name="fileName">Optional file name used for diagnostics.</param>
    /// <remarks>
    /// Reusing a component snapshot with different source text is undefined behavior.
    /// </remarks>
    /// <returns>The dispatch result serialized as canonical NX bytes.</returns>
    public static byte[] DispatchComponentActionsBytes(
        string source,
        byte[] stateSnapshot,
        byte[]? actionsBytes = null,
        string? fileName = null)
    {
        byte[] payload = InvokeComponentDispatchNativeCall(
            source,
            stateSnapshot,
            actionsBytes,
            fileName,
            NxNativeMethods.nx_component_dispatch_actions,
            out NxEvalStatus status);

        return status switch
        {
            NxEvalStatus.Ok => payload,
            NxEvalStatus.Error => throw CreateEvaluationExceptionFromMessagePack(payload),
            _ => throw CreateInteropStatusException(status),
        };
    }

    /// <summary>
    /// Converts canonical component dispatch result bytes into a JSON string for debugging or inspection.
    /// </summary>
    /// <param name="resultBytes">The canonical component dispatch result bytes to convert.</param>
    /// <returns>The equivalent JSON string.</returns>
    public static string ComponentDispatchResultBytesToJson(byte[] resultBytes)
    {
        return ConvertMessagePackPayloadToJson(
            resultBytes,
            NxNativeMethods.nx_component_dispatch_result_msgpack_to_json,
            "NX native runtime returned an invalid component dispatch MessagePack payload.");
    }

    /// <summary>
    /// Dispatches no actions against a prior component state snapshot and deserializes the result.
    /// </summary>
    /// <typeparam name="TEffect">The managed type for effect action payloads.</typeparam>
    /// <param name="source">The NX source code containing the component definition.</param>
    /// <param name="stateSnapshot">The opaque state snapshot returned by initialization or a prior dispatch.</param>
    /// <param name="fileName">Optional file name used for diagnostics.</param>
    /// <returns>The typed component dispatch result.</returns>
    public static NxComponentDispatchResult<TEffect> DispatchComponentActions<TEffect>(
        string source,
        byte[] stateSnapshot,
        string? fileName = null)
    {
        byte[] payload = DispatchComponentActionsBytes(source, stateSnapshot, null, fileName);
        return DeserializeMessagePackResult<NxComponentDispatchResult<TEffect>>(
            payload,
            "NX native runtime returned an invalid component dispatch MessagePack payload.");
    }

    /// <summary>
    /// Dispatches MessagePack-serializable actions against a prior component state snapshot and deserializes the result.
    /// </summary>
    /// <typeparam name="TActions">The managed type used to serialize the action list.</typeparam>
    /// <typeparam name="TEffect">The managed type for effect action payloads.</typeparam>
    /// <param name="source">The NX source code containing the component definition.</param>
    /// <param name="stateSnapshot">The opaque state snapshot returned by initialization or a prior dispatch.</param>
    /// <param name="actions">The action list to serialize using MessagePack.</param>
    /// <param name="fileName">Optional file name used for diagnostics.</param>
    /// <returns>The typed component dispatch result.</returns>
    public static NxComponentDispatchResult<TEffect> DispatchComponentActions<TActions, TEffect>(
        string source,
        byte[] stateSnapshot,
        TActions actions,
        string? fileName = null)
    {
        byte[] actionsBytes = actions is null
            ? Array.Empty<byte>()
            : MessagePackSerializer.Serialize(actions, MessagePackOptions);
        byte[] payload = DispatchComponentActionsBytes(source, stateSnapshot, actionsBytes, fileName);
        return DeserializeMessagePackResult<NxComponentDispatchResult<TEffect>>(
            payload,
            "NX native runtime returned an invalid component dispatch MessagePack payload.");
    }

    private delegate NxEvalStatus EvalSourceCallback(
        byte[] sourceBytes,
        nuint sourceLength,
        byte[] fileNameBytes,
        nuint fileNameLength,
        out NxBuffer buffer);

    private delegate NxEvalStatus ComponentInitCallback(
        byte[] sourceBytes,
        nuint sourceLength,
        byte[] fileNameBytes,
        nuint fileNameLength,
        byte[] componentNameBytes,
        nuint componentNameLength,
        byte[] propsBytes,
        nuint propsLength,
        out NxBuffer buffer);

    private delegate NxEvalStatus ComponentDispatchCallback(
        byte[] sourceBytes,
        nuint sourceLength,
        byte[] fileNameBytes,
        nuint fileNameLength,
        byte[] stateSnapshotBytes,
        nuint stateSnapshotLength,
        byte[] actionsBytes,
        nuint actionsLength,
        out NxBuffer buffer);

    private delegate NxEvalStatus MsgpackToJsonCallback(
        byte[] payloadBytes,
        nuint payloadLength,
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

    private static byte[] InvokeComponentInitNativeCall(
        string source,
        string componentName,
        byte[]? propsBytes,
        string? fileName,
        ComponentInitCallback callback,
        out NxEvalStatus status)
    {
        ArgumentNullException.ThrowIfNull(source);
        ArgumentNullException.ThrowIfNull(componentName);

        NxNativeLibrary.EnsureLoaded();

        byte[] sourceBytes = Encoding.UTF8.GetBytes(source);
        byte[] fileNameBytes = fileName is null ? Array.Empty<byte>() : Encoding.UTF8.GetBytes(fileName);
        byte[] componentNameBytes = Encoding.UTF8.GetBytes(componentName);
        byte[] payloadBytes = propsBytes ?? Array.Empty<byte>();

        status = callback(
            sourceBytes,
            (nuint)sourceBytes.Length,
            fileNameBytes,
            (nuint)fileNameBytes.Length,
            componentNameBytes,
            (nuint)componentNameBytes.Length,
            payloadBytes,
            (nuint)payloadBytes.Length,
            out NxBuffer buffer);

        return CopyAndFreeBuffer(buffer);
    }

    private static byte[] InvokeComponentDispatchNativeCall(
        string source,
        byte[] stateSnapshot,
        byte[]? actionsBytes,
        string? fileName,
        ComponentDispatchCallback callback,
        out NxEvalStatus status)
    {
        ArgumentNullException.ThrowIfNull(source);
        ArgumentNullException.ThrowIfNull(stateSnapshot);

        NxNativeLibrary.EnsureLoaded();

        byte[] sourceBytes = Encoding.UTF8.GetBytes(source);
        byte[] fileNameBytes = fileName is null ? Array.Empty<byte>() : Encoding.UTF8.GetBytes(fileName);
        byte[] payloadBytes = actionsBytes ?? Array.Empty<byte>();

        status = callback(
            sourceBytes,
            (nuint)sourceBytes.Length,
            fileNameBytes,
            (nuint)fileNameBytes.Length,
            stateSnapshot,
            (nuint)stateSnapshot.Length,
            payloadBytes,
            (nuint)payloadBytes.Length,
            out NxBuffer buffer);

        return CopyAndFreeBuffer(buffer);
    }

    private static string ConvertMessagePackPayloadToJson(
        byte[] payload,
        MsgpackToJsonCallback callback,
        string invalidPayloadMessage)
    {
        ArgumentNullException.ThrowIfNull(payload);

        NxNativeLibrary.EnsureLoaded();

        NxEvalStatus status = callback(
            payload,
            (nuint)payload.Length,
            out NxBuffer buffer);

        byte[] jsonBytes = CopyAndFreeBuffer(buffer);
        string json = DecodeUtf8Payload(jsonBytes, invalidPayloadMessage);

        return status switch
        {
            NxEvalStatus.Ok => json,
            NxEvalStatus.Error => throw new InvalidOperationException(
                $"{invalidPayloadMessage} Details: {json}"),
            _ => throw CreateInteropStatusException(status),
        };
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

    private static T DeserializeMessagePackResult<T>(byte[] payload, string message)
    {
        try
        {
            return MessagePackSerializer.Deserialize<T>(payload, MessagePackOptions);
        }
        catch (MessagePackSerializationException e)
        {
            throw new InvalidOperationException(message, e);
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

    private static string DecodeUtf8Payload(byte[] payload, string message)
    {
        try
        {
            return StrictUtf8.GetString(payload);
        }
        catch (DecoderFallbackException e)
        {
            throw new InvalidOperationException(message, e);
        }
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
