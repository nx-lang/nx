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
    private static readonly UTF8Encoding StrictUtf8 =
        new(encoderShouldEmitUTF8Identifier: false, throwOnInvalidBytes: true);

    /// <summary>
    /// Evaluates NX source code and returns the raw result bytes in the canonical MessagePack wire format.
    /// </summary>
    /// <param name="source">The NX source code to evaluate. Must contain a root() function that returns the result.</param>
    /// <param name="fileName">Optional file name for diagnostic messages. Used in error reporting to identify the source location.</param>
    /// <returns>The evaluation result serialized as canonical NX bytes.</returns>
    public static byte[] EvaluateBytes(string source, string? fileName = null)
    {
        byte[] payload = InvokeSourceNativeCall(source, fileName, NxNativeMethods.nx_eval_source, out NxEvalStatus status);
        return status switch
        {
            NxEvalStatus.Ok => payload,
            NxEvalStatus.Error => throw CreateEvaluationExceptionFromMessagePack(payload),
            _ => throw CreateInteropStatusException(status),
        };
    }

    /// <summary>
    /// Evaluates the <c>root()</c> entrypoint of a previously built program artifact.
    /// </summary>
    public static byte[] EvaluateBytes(NxProgramArtifact programArtifact)
    {
        byte[] payload = InvokeProgramArtifactNativeCall(
            programArtifact,
            NxNativeMethods.nx_eval_program_artifact,
            out NxEvalStatus status);
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
    public static T Evaluate<T>(string source, string? fileName = null)
    {
        byte[] bytes = EvaluateBytes(source, fileName);
        return MessagePackSerializer.Deserialize<T>(bytes, MessagePackOptions);
    }

    /// <summary>
    /// Evaluates the <c>root()</c> entrypoint of a previously built program artifact and deserializes the result.
    /// </summary>
    public static T Evaluate<T>(NxProgramArtifact programArtifact)
    {
        byte[] bytes = EvaluateBytes(programArtifact);
        return MessagePackSerializer.Deserialize<T>(bytes, MessagePackOptions);
    }

    /// <summary>
    /// Initializes a named component and returns the raw result bytes in the canonical MessagePack wire format.
    /// </summary>
    public static byte[] InitializeComponentBytes(
        string source,
        string componentName,
        byte[]? propsBytes = null,
        string? fileName = null)
    {
        byte[] payload = InvokeComponentInitSourceNativeCall(
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
    /// Initializes a named component from a previously built program artifact and returns the raw result bytes.
    /// </summary>
    public static byte[] InitializeComponentBytes(
        NxProgramArtifact programArtifact,
        string componentName,
        byte[]? propsBytes = null)
    {
        byte[] payload = InvokeComponentInitProgramArtifactNativeCall(
            programArtifact,
            componentName,
            propsBytes,
            NxNativeMethods.nx_component_init_program_artifact,
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
    /// Initializes a named component from a program artifact using no explicit props and deserializes the rendered result.
    /// </summary>
    public static NxComponentInitResult<TElement> InitializeComponent<TElement>(
        NxProgramArtifact programArtifact,
        string componentName)
    {
        byte[] payload = InitializeComponentBytes(programArtifact, componentName, null);
        return DeserializeMessagePackResult<NxComponentInitResult<TElement>>(
            payload,
            "NX native runtime returned an invalid component initialization MessagePack payload.");
    }

    /// <summary>
    /// Initializes a named component with MessagePack-serializable props and deserializes the rendered result.
    /// </summary>
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
    /// Initializes a named component from a program artifact with MessagePack-serializable props and deserializes the result.
    /// </summary>
    public static NxComponentInitResult<TElement> InitializeComponent<TProps, TElement>(
        NxProgramArtifact programArtifact,
        string componentName,
        TProps props)
    {
        byte[] propsBytes = props is null
            ? Array.Empty<byte>()
            : MessagePackSerializer.Serialize(props, MessagePackOptions);
        byte[] payload = InitializeComponentBytes(programArtifact, componentName, propsBytes);
        return DeserializeMessagePackResult<NxComponentInitResult<TElement>>(
            payload,
            "NX native runtime returned an invalid component initialization MessagePack payload.");
    }

    /// <summary>
    /// Dispatches actions against a prior component state snapshot and returns the raw result bytes in the canonical
    /// MessagePack wire format.
    /// </summary>
    public static byte[] DispatchComponentActionsBytes(
        string source,
        byte[] stateSnapshot,
        byte[]? actionsBytes = null,
        string? fileName = null)
    {
        byte[] payload = InvokeComponentDispatchSourceNativeCall(
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
    /// Dispatches actions against a prior component state snapshot for a previously built program artifact.
    /// </summary>
    public static byte[] DispatchComponentActionsBytes(
        NxProgramArtifact programArtifact,
        byte[] stateSnapshot,
        byte[]? actionsBytes = null)
    {
        byte[] payload = InvokeComponentDispatchProgramArtifactNativeCall(
            programArtifact,
            stateSnapshot,
            actionsBytes,
            NxNativeMethods.nx_component_dispatch_actions_program_artifact,
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
    /// Dispatches no actions against a prior component state snapshot for a previously built program artifact.
    /// </summary>
    public static NxComponentDispatchResult<TEffect> DispatchComponentActions<TEffect>(
        NxProgramArtifact programArtifact,
        byte[] stateSnapshot)
    {
        byte[] payload = DispatchComponentActionsBytes(programArtifact, stateSnapshot, null);
        return DeserializeMessagePackResult<NxComponentDispatchResult<TEffect>>(
            payload,
            "NX native runtime returned an invalid component dispatch MessagePack payload.");
    }

    /// <summary>
    /// Dispatches MessagePack-serializable actions against a prior component state snapshot and deserializes the result.
    /// </summary>
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

    /// <summary>
    /// Dispatches MessagePack-serializable actions against a prior component state snapshot for a program artifact.
    /// </summary>
    public static NxComponentDispatchResult<TEffect> DispatchComponentActions<TActions, TEffect>(
        NxProgramArtifact programArtifact,
        byte[] stateSnapshot,
        TActions actions)
    {
        byte[] actionsBytes = actions is null
            ? Array.Empty<byte>()
            : MessagePackSerializer.Serialize(actions, MessagePackOptions);
        byte[] payload = DispatchComponentActionsBytes(programArtifact, stateSnapshot, actionsBytes);
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

    private delegate NxEvalStatus EvalProgramArtifactCallback(
        IntPtr programArtifactHandle,
        out NxBuffer buffer);

    private delegate NxEvalStatus ComponentInitSourceCallback(
        byte[] sourceBytes,
        nuint sourceLength,
        byte[] fileNameBytes,
        nuint fileNameLength,
        byte[] componentNameBytes,
        nuint componentNameLength,
        byte[] propsBytes,
        nuint propsLength,
        out NxBuffer buffer);

    private delegate NxEvalStatus ComponentInitProgramArtifactCallback(
        IntPtr programArtifactHandle,
        byte[] componentNameBytes,
        nuint componentNameLength,
        byte[] propsBytes,
        nuint propsLength,
        out NxBuffer buffer);

    private delegate NxEvalStatus ComponentDispatchSourceCallback(
        byte[] sourceBytes,
        nuint sourceLength,
        byte[] fileNameBytes,
        nuint fileNameLength,
        byte[] stateSnapshotBytes,
        nuint stateSnapshotLength,
        byte[] actionsBytes,
        nuint actionsLength,
        out NxBuffer buffer);

    private delegate NxEvalStatus ComponentDispatchProgramArtifactCallback(
        IntPtr programArtifactHandle,
        byte[] stateSnapshotBytes,
        nuint stateSnapshotLength,
        byte[] actionsBytes,
        nuint actionsLength,
        out NxBuffer buffer);

    private delegate NxEvalStatus MsgpackToJsonCallback(
        byte[] payloadBytes,
        nuint payloadLength,
        out NxBuffer buffer);

    private static byte[] InvokeSourceNativeCall(
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

    private static byte[] InvokeProgramArtifactNativeCall(
        NxProgramArtifact programArtifact,
        EvalProgramArtifactCallback callback,
        out NxEvalStatus status)
    {
        ArgumentNullException.ThrowIfNull(programArtifact);

        NxNativeLibrary.EnsureLoaded();

        status = callback(
            programArtifact.DangerousGetHandle(),
            out NxBuffer buffer);
        return CopyAndFreeBuffer(buffer);
    }

    private static byte[] InvokeComponentInitSourceNativeCall(
        string source,
        string componentName,
        byte[]? propsBytes,
        string? fileName,
        ComponentInitSourceCallback callback,
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

    private static byte[] InvokeComponentInitProgramArtifactNativeCall(
        NxProgramArtifact programArtifact,
        string componentName,
        byte[]? propsBytes,
        ComponentInitProgramArtifactCallback callback,
        out NxEvalStatus status)
    {
        ArgumentNullException.ThrowIfNull(programArtifact);
        ArgumentNullException.ThrowIfNull(componentName);

        NxNativeLibrary.EnsureLoaded();

        byte[] componentNameBytes = Encoding.UTF8.GetBytes(componentName);
        byte[] payloadBytes = propsBytes ?? Array.Empty<byte>();

        status = callback(
            programArtifact.DangerousGetHandle(),
            componentNameBytes,
            (nuint)componentNameBytes.Length,
            payloadBytes,
            (nuint)payloadBytes.Length,
            out NxBuffer buffer);
        return CopyAndFreeBuffer(buffer);
    }

    private static byte[] InvokeComponentDispatchSourceNativeCall(
        string source,
        byte[] stateSnapshot,
        byte[]? actionsBytes,
        string? fileName,
        ComponentDispatchSourceCallback callback,
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

    private static byte[] InvokeComponentDispatchProgramArtifactNativeCall(
        NxProgramArtifact programArtifact,
        byte[] stateSnapshot,
        byte[]? actionsBytes,
        ComponentDispatchProgramArtifactCallback callback,
        out NxEvalStatus status)
    {
        ArgumentNullException.ThrowIfNull(programArtifact);
        ArgumentNullException.ThrowIfNull(stateSnapshot);

        NxNativeLibrary.EnsureLoaded();

        byte[] payloadBytes = actionsBytes ?? Array.Empty<byte>();

        status = callback(
            programArtifact.DangerousGetHandle(),
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
            NxEvalStatus.Error => throw new InvalidOperationException($"{invalidPayloadMessage} Details: {json}"),
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
