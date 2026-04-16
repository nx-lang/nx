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
/// Provides methods for evaluating NX source code and interacting with components through the
/// native NX runtime.
/// </summary>
public static class NxRuntime
{
    private static readonly MessagePackSerializerOptions MessagePackOptions =
        MessagePackSerializerOptions.Standard.WithSecurity(MessagePackSecurity.UntrustedData);

    /// <summary>
    /// Evaluates NX source code and returns the raw result bytes in the canonical MessagePack wire format.
    /// </summary>
    /// <param name="source">The NX source code to evaluate. Must contain a root() function that returns the result.</param>
    /// <param name="fileName">Optional file name for diagnostic messages. Used in error reporting to identify the source location.</param>
    /// <returns>The evaluation result serialized as canonical NX bytes.</returns>
    public static byte[] EvaluateBytes(string source, string? fileName = null)
    {
        return EvaluateBytes(source, NxOutputFormat.MessagePack, fileName);
    }

    /// <summary>
    /// Evaluates NX source code and returns the raw result bytes in the requested output format.
    /// </summary>
    public static byte[] EvaluateBytes(string source, NxOutputFormat outputFormat, string? fileName = null)
    {
        byte[] payload = InvokeSourceNativeCall(
            source,
            fileName,
            outputFormat,
            NxNativeMethods.nx_eval_source,
            out NxEvalStatus status);
        return status switch
        {
            NxEvalStatus.Ok => payload,
            NxEvalStatus.Error => throw CreateEvaluationException(payload, outputFormat),
            _ => throw CreateInteropStatusException(status),
        };
    }

    /// <summary>
    /// Evaluates NX source code against a caller-supplied build context.
    /// </summary>
    public static byte[] EvaluateBytes(string source, NxProgramBuildContext buildContext, string? fileName = null)
    {
        return EvaluateBytes(source, buildContext, NxOutputFormat.MessagePack, fileName);
    }

    /// <summary>
    /// Evaluates NX source code against a caller-supplied build context in the requested output format.
    /// </summary>
    public static byte[] EvaluateBytes(
        string source,
        NxProgramBuildContext buildContext,
        NxOutputFormat outputFormat,
        string? fileName = null)
    {
        ArgumentNullException.ThrowIfNull(buildContext);

        using NxProgramArtifact programArtifact = NxProgramArtifact.Build(source, buildContext, fileName);
        return EvaluateBytes(programArtifact, outputFormat);
    }

    /// <summary>
    /// Evaluates the <c>root()</c> entrypoint of a previously built program artifact.
    /// </summary>
    public static byte[] EvaluateBytes(NxProgramArtifact programArtifact)
    {
        return EvaluateBytes(programArtifact, NxOutputFormat.MessagePack);
    }

    /// <summary>
    /// Evaluates the <c>root()</c> entrypoint of a previously built program artifact in the requested output format.
    /// </summary>
    public static byte[] EvaluateBytes(NxProgramArtifact programArtifact, NxOutputFormat outputFormat)
    {
        byte[] payload = InvokeProgramArtifactNativeCall(
            programArtifact,
            outputFormat,
            NxNativeMethods.nx_eval_program_artifact,
            out NxEvalStatus status);
        return status switch
        {
            NxEvalStatus.Ok => payload,
            NxEvalStatus.Error => throw CreateEvaluationException(payload, outputFormat),
            _ => throw CreateInteropStatusException(status),
        };
    }

    /// <summary>
    /// Evaluates NX source code and returns the JSON result as a <see cref="JsonElement"/>.
    /// </summary>
    public static JsonElement EvaluateJson(string source, string? fileName = null)
    {
        byte[] payload = EvaluateBytes(source, NxOutputFormat.Json, fileName);
        return DeserializeJsonElement(
            payload,
            "NX native runtime returned an invalid evaluation JSON payload.");
    }

    /// <summary>
    /// Evaluates NX source code against a caller-supplied build context and returns the JSON result.
    /// </summary>
    public static JsonElement EvaluateJson(
        string source,
        NxProgramBuildContext buildContext,
        string? fileName = null)
    {
        byte[] payload = EvaluateBytes(source, buildContext, NxOutputFormat.Json, fileName);
        return DeserializeJsonElement(
            payload,
            "NX native runtime returned an invalid evaluation JSON payload.");
    }

    /// <summary>
    /// Evaluates the <c>root()</c> entrypoint of a previously built program artifact and returns the JSON result.
    /// </summary>
    public static JsonElement EvaluateJson(NxProgramArtifact programArtifact)
    {
        byte[] payload = EvaluateBytes(programArtifact, NxOutputFormat.Json);
        return DeserializeJsonElement(
            payload,
            "NX native runtime returned an invalid evaluation JSON payload.");
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
    /// Evaluates NX source code against a caller-supplied build context and deserializes the result.
    /// </summary>
    public static T Evaluate<T>(string source, NxProgramBuildContext buildContext, string? fileName = null)
    {
        byte[] bytes = EvaluateBytes(source, buildContext, fileName);
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
        return InitializeComponentBytes(
            source,
            componentName,
            NxOutputFormat.MessagePack,
            propsBytes,
            fileName);
    }

    /// <summary>
    /// Initializes a named component and returns the raw result bytes in the requested output format.
    /// </summary>
    public static byte[] InitializeComponentBytes(
        string source,
        string componentName,
        NxOutputFormat outputFormat,
        byte[]? propsBytes = null,
        string? fileName = null)
    {
        using NxProgramArtifact programArtifact = NxProgramArtifact.Build(source, fileName);
        return InitializeComponentBytes(programArtifact, componentName, outputFormat, propsBytes);
    }

    /// <summary>
    /// Initializes a named component from source text against a caller-supplied build context.
    /// </summary>
    public static byte[] InitializeComponentBytes(
        string source,
        string componentName,
        NxProgramBuildContext buildContext,
        byte[]? propsBytes = null,
        string? fileName = null)
    {
        return InitializeComponentBytes(
            source,
            componentName,
            buildContext,
            NxOutputFormat.MessagePack,
            propsBytes,
            fileName);
    }

    /// <summary>
    /// Initializes a named component from source text against a caller-supplied build context in the requested output
    /// format.
    /// </summary>
    public static byte[] InitializeComponentBytes(
        string source,
        string componentName,
        NxProgramBuildContext buildContext,
        NxOutputFormat outputFormat,
        byte[]? propsBytes = null,
        string? fileName = null)
    {
        ArgumentNullException.ThrowIfNull(buildContext);

        using NxProgramArtifact programArtifact = NxProgramArtifact.Build(source, buildContext, fileName);
        return InitializeComponentBytes(programArtifact, componentName, outputFormat, propsBytes);
    }

    /// <summary>
    /// Initializes a named component from a previously built program artifact and returns the raw result bytes.
    /// </summary>
    public static byte[] InitializeComponentBytes(
        NxProgramArtifact programArtifact,
        string componentName,
        byte[]? propsBytes = null)
    {
        return InitializeComponentBytes(
            programArtifact,
            componentName,
            NxOutputFormat.MessagePack,
            propsBytes);
    }

    /// <summary>
    /// Initializes a named component from a previously built program artifact and returns the raw result bytes in the
    /// requested output format.
    /// </summary>
    public static byte[] InitializeComponentBytes(
        NxProgramArtifact programArtifact,
        string componentName,
        NxOutputFormat outputFormat,
        byte[]? propsBytes = null)
    {
        byte[] payload = InvokeComponentInitProgramArtifactNativeCall(
            programArtifact,
            componentName,
            outputFormat,
            propsBytes,
            NxNativeMethods.nx_component_init_program_artifact,
            out NxEvalStatus status);
        return status switch
        {
            NxEvalStatus.Ok => payload,
            NxEvalStatus.Error => throw CreateEvaluationException(payload, outputFormat),
            _ => throw CreateInteropStatusException(status),
        };
    }

    /// <summary>
    /// Initializes a named component using no explicit props and returns the JSON result.
    /// </summary>
    public static NxComponentInitResult<JsonElement> InitializeComponentJson(
        string source,
        string componentName,
        string? fileName = null)
    {
        byte[] payload = InitializeComponentBytes(source, componentName, NxOutputFormat.Json, null, fileName);
        return DeserializeJsonComponentInitResult(
            payload,
            "NX native runtime returned an invalid component initialization JSON payload.");
    }

    /// <summary>
    /// Initializes a named component from source text using a caller-supplied build context and returns the JSON
    /// result.
    /// </summary>
    public static NxComponentInitResult<JsonElement> InitializeComponentJson(
        string source,
        string componentName,
        NxProgramBuildContext buildContext,
        string? fileName = null)
    {
        byte[] payload = InitializeComponentBytes(source, componentName, buildContext, NxOutputFormat.Json, null, fileName);
        return DeserializeJsonComponentInitResult(
            payload,
            "NX native runtime returned an invalid component initialization JSON payload.");
    }

    /// <summary>
    /// Initializes a named component from a program artifact using no explicit props and returns the JSON result.
    /// </summary>
    public static NxComponentInitResult<JsonElement> InitializeComponentJson(
        NxProgramArtifact programArtifact,
        string componentName)
    {
        byte[] payload = InitializeComponentBytes(programArtifact, componentName, NxOutputFormat.Json, null);
        return DeserializeJsonComponentInitResult(
            payload,
            "NX native runtime returned an invalid component initialization JSON payload.");
    }

    /// <summary>
    /// Initializes a named component with MessagePack-serializable props and returns the JSON result.
    /// </summary>
    public static NxComponentInitResult<JsonElement> InitializeComponentJson<TProps>(
        string source,
        string componentName,
        TProps props,
        string? fileName = null)
    {
        byte[] payload = InitializeComponentBytes(
            source,
            componentName,
            NxOutputFormat.Json,
            SerializeMessagePackInput(props),
            fileName);
        return DeserializeJsonComponentInitResult(
            payload,
            "NX native runtime returned an invalid component initialization JSON payload.");
    }

    /// <summary>
    /// Initializes a named component with MessagePack-serializable props against a caller-supplied build context and
    /// returns the JSON result.
    /// </summary>
    public static NxComponentInitResult<JsonElement> InitializeComponentJson<TProps>(
        string source,
        string componentName,
        NxProgramBuildContext buildContext,
        TProps props,
        string? fileName = null)
    {
        byte[] payload = InitializeComponentBytes(
            source,
            componentName,
            buildContext,
            NxOutputFormat.Json,
            SerializeMessagePackInput(props),
            fileName);
        return DeserializeJsonComponentInitResult(
            payload,
            "NX native runtime returned an invalid component initialization JSON payload.");
    }

    /// <summary>
    /// Initializes a named component from a program artifact with MessagePack-serializable props and returns the JSON
    /// result.
    /// </summary>
    public static NxComponentInitResult<JsonElement> InitializeComponentJson<TProps>(
        NxProgramArtifact programArtifact,
        string componentName,
        TProps props)
    {
        byte[] payload = InitializeComponentBytes(
            programArtifact,
            componentName,
            NxOutputFormat.Json,
            SerializeMessagePackInput(props));
        return DeserializeJsonComponentInitResult(
            payload,
            "NX native runtime returned an invalid component initialization JSON payload.");
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
    /// Initializes a named component from source text using a caller-supplied build context.
    /// </summary>
    public static NxComponentInitResult<TElement> InitializeComponent<TElement>(
        string source,
        string componentName,
        NxProgramBuildContext buildContext,
        string? fileName = null)
    {
        byte[] payload = InitializeComponentBytes(source, componentName, buildContext, null, fileName);
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
        byte[] propsBytes = SerializeMessagePackInput(props);
        byte[] payload = InitializeComponentBytes(source, componentName, propsBytes, fileName);
        return DeserializeMessagePackResult<NxComponentInitResult<TElement>>(
            payload,
            "NX native runtime returned an invalid component initialization MessagePack payload.");
    }

    /// <summary>
    /// Initializes a named component with MessagePack-serializable props against a caller-supplied build context.
    /// </summary>
    public static NxComponentInitResult<TElement> InitializeComponent<TProps, TElement>(
        string source,
        string componentName,
        NxProgramBuildContext buildContext,
        TProps props,
        string? fileName = null)
    {
        byte[] propsBytes = SerializeMessagePackInput(props);
        byte[] payload = InitializeComponentBytes(source, componentName, buildContext, propsBytes, fileName);
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
        byte[] propsBytes = SerializeMessagePackInput(props);
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
        return DispatchComponentActionsBytes(
            source,
            stateSnapshot,
            NxOutputFormat.MessagePack,
            actionsBytes,
            fileName);
    }

    /// <summary>
    /// Dispatches actions against a prior component state snapshot and returns the raw result bytes in the requested
    /// output format.
    /// </summary>
    public static byte[] DispatchComponentActionsBytes(
        string source,
        byte[] stateSnapshot,
        NxOutputFormat outputFormat,
        byte[]? actionsBytes = null,
        string? fileName = null)
    {
        using NxProgramArtifact programArtifact = NxProgramArtifact.Build(source, fileName);
        return DispatchComponentActionsBytes(programArtifact, stateSnapshot, outputFormat, actionsBytes);
    }

    /// <summary>
    /// Dispatches actions against source text using a caller-supplied build context.
    /// </summary>
    public static byte[] DispatchComponentActionsBytes(
        string source,
        byte[] stateSnapshot,
        NxProgramBuildContext buildContext,
        byte[]? actionsBytes = null,
        string? fileName = null)
    {
        return DispatchComponentActionsBytes(
            source,
            stateSnapshot,
            buildContext,
            NxOutputFormat.MessagePack,
            actionsBytes,
            fileName);
    }

    /// <summary>
    /// Dispatches actions against source text using a caller-supplied build context and returns the raw result bytes
    /// in the requested output format.
    /// </summary>
    public static byte[] DispatchComponentActionsBytes(
        string source,
        byte[] stateSnapshot,
        NxProgramBuildContext buildContext,
        NxOutputFormat outputFormat,
        byte[]? actionsBytes = null,
        string? fileName = null)
    {
        ArgumentNullException.ThrowIfNull(buildContext);

        using NxProgramArtifact programArtifact = NxProgramArtifact.Build(source, buildContext, fileName);
        return DispatchComponentActionsBytes(programArtifact, stateSnapshot, outputFormat, actionsBytes);
    }

    /// <summary>
    /// Dispatches actions against a prior component state snapshot for a previously built program artifact.
    /// </summary>
    public static byte[] DispatchComponentActionsBytes(
        NxProgramArtifact programArtifact,
        byte[] stateSnapshot,
        byte[]? actionsBytes = null)
    {
        return DispatchComponentActionsBytes(
            programArtifact,
            stateSnapshot,
            NxOutputFormat.MessagePack,
            actionsBytes);
    }

    /// <summary>
    /// Dispatches actions against a prior component state snapshot for a previously built program artifact and returns
    /// the raw result bytes in the requested output format.
    /// </summary>
    public static byte[] DispatchComponentActionsBytes(
        NxProgramArtifact programArtifact,
        byte[] stateSnapshot,
        NxOutputFormat outputFormat,
        byte[]? actionsBytes = null)
    {
        byte[] payload = InvokeComponentDispatchProgramArtifactNativeCall(
            programArtifact,
            stateSnapshot,
            outputFormat,
            actionsBytes,
            NxNativeMethods.nx_component_dispatch_actions_program_artifact,
            out NxEvalStatus status);
        return status switch
        {
            NxEvalStatus.Ok => payload,
            NxEvalStatus.Error => throw CreateEvaluationException(payload, outputFormat),
            _ => throw CreateInteropStatusException(status),
        };
    }

    /// <summary>
    /// Dispatches no actions against a prior component state snapshot and returns the JSON result.
    /// </summary>
    public static NxComponentDispatchResult<JsonElement> DispatchComponentActionsJson(
        string source,
        byte[] stateSnapshot,
        string? fileName = null)
    {
        byte[] payload = DispatchComponentActionsBytes(source, stateSnapshot, NxOutputFormat.Json, null, fileName);
        return DeserializeJsonComponentDispatchResult(
            payload,
            "NX native runtime returned an invalid component dispatch JSON payload.");
    }

    /// <summary>
    /// Dispatches no actions against a prior component state snapshot using a caller-supplied build context and
    /// returns the JSON result.
    /// </summary>
    public static NxComponentDispatchResult<JsonElement> DispatchComponentActionsJson(
        string source,
        byte[] stateSnapshot,
        NxProgramBuildContext buildContext,
        string? fileName = null)
    {
        byte[] payload = DispatchComponentActionsBytes(
            source,
            stateSnapshot,
            buildContext,
            NxOutputFormat.Json,
            null,
            fileName);
        return DeserializeJsonComponentDispatchResult(
            payload,
            "NX native runtime returned an invalid component dispatch JSON payload.");
    }

    /// <summary>
    /// Dispatches no actions against a prior component state snapshot for a previously built program artifact and
    /// returns the JSON result.
    /// </summary>
    public static NxComponentDispatchResult<JsonElement> DispatchComponentActionsJson(
        NxProgramArtifact programArtifact,
        byte[] stateSnapshot)
    {
        byte[] payload = DispatchComponentActionsBytes(programArtifact, stateSnapshot, NxOutputFormat.Json, null);
        return DeserializeJsonComponentDispatchResult(
            payload,
            "NX native runtime returned an invalid component dispatch JSON payload.");
    }

    /// <summary>
    /// Dispatches MessagePack-serializable actions against a prior component state snapshot and returns the JSON
    /// result.
    /// </summary>
    public static NxComponentDispatchResult<JsonElement> DispatchComponentActionsJson<TActions>(
        string source,
        byte[] stateSnapshot,
        TActions actions,
        string? fileName = null)
    {
        byte[] payload = DispatchComponentActionsBytes(
            source,
            stateSnapshot,
            NxOutputFormat.Json,
            SerializeMessagePackInput(actions),
            fileName);
        return DeserializeJsonComponentDispatchResult(
            payload,
            "NX native runtime returned an invalid component dispatch JSON payload.");
    }

    /// <summary>
    /// Dispatches MessagePack-serializable actions against source text using a caller-supplied build context and
    /// returns the JSON result.
    /// </summary>
    public static NxComponentDispatchResult<JsonElement> DispatchComponentActionsJson<TActions>(
        string source,
        byte[] stateSnapshot,
        NxProgramBuildContext buildContext,
        TActions actions,
        string? fileName = null)
    {
        byte[] payload = DispatchComponentActionsBytes(
            source,
            stateSnapshot,
            buildContext,
            NxOutputFormat.Json,
            SerializeMessagePackInput(actions),
            fileName);
        return DeserializeJsonComponentDispatchResult(
            payload,
            "NX native runtime returned an invalid component dispatch JSON payload.");
    }

    /// <summary>
    /// Dispatches MessagePack-serializable actions against a prior component state snapshot for a previously built
    /// program artifact and returns the JSON result.
    /// </summary>
    public static NxComponentDispatchResult<JsonElement> DispatchComponentActionsJson<TActions>(
        NxProgramArtifact programArtifact,
        byte[] stateSnapshot,
        TActions actions)
    {
        byte[] payload = DispatchComponentActionsBytes(
            programArtifact,
            stateSnapshot,
            NxOutputFormat.Json,
            SerializeMessagePackInput(actions));
        return DeserializeJsonComponentDispatchResult(
            payload,
            "NX native runtime returned an invalid component dispatch JSON payload.");
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
    /// Dispatches no actions against a prior component state snapshot using a caller-supplied build context.
    /// </summary>
    public static NxComponentDispatchResult<TEffect> DispatchComponentActions<TEffect>(
        string source,
        byte[] stateSnapshot,
        NxProgramBuildContext buildContext,
        string? fileName = null)
    {
        byte[] payload = DispatchComponentActionsBytes(source, stateSnapshot, buildContext, null, fileName);
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
        byte[] actionsBytes = SerializeMessagePackInput(actions);
        byte[] payload = DispatchComponentActionsBytes(source, stateSnapshot, actionsBytes, fileName);
        return DeserializeMessagePackResult<NxComponentDispatchResult<TEffect>>(
            payload,
            "NX native runtime returned an invalid component dispatch MessagePack payload.");
    }

    /// <summary>
    /// Dispatches MessagePack-serializable actions against source text using a caller-supplied build context.
    /// </summary>
    public static NxComponentDispatchResult<TEffect> DispatchComponentActions<TActions, TEffect>(
        string source,
        byte[] stateSnapshot,
        NxProgramBuildContext buildContext,
        TActions actions,
        string? fileName = null)
    {
        byte[] actionsBytes = SerializeMessagePackInput(actions);
        byte[] payload = DispatchComponentActionsBytes(source, stateSnapshot, buildContext, actionsBytes, fileName);
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
        byte[] actionsBytes = SerializeMessagePackInput(actions);
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
        NxOutputFormat outputFormat,
        out NxBuffer buffer);

    private delegate NxEvalStatus EvalProgramArtifactCallback(
        NxProgramArtifactSafeHandle programArtifactHandle,
        NxOutputFormat outputFormat,
        out NxBuffer buffer);

    private delegate NxEvalStatus ComponentInitProgramArtifactCallback(
        NxProgramArtifactSafeHandle programArtifactHandle,
        byte[] componentNameBytes,
        nuint componentNameLength,
        byte[] propsBytes,
        nuint propsLength,
        NxOutputFormat outputFormat,
        out NxBuffer buffer);

    private delegate NxEvalStatus ComponentDispatchProgramArtifactCallback(
        NxProgramArtifactSafeHandle programArtifactHandle,
        byte[] stateSnapshotBytes,
        nuint stateSnapshotLength,
        byte[] actionsBytes,
        nuint actionsLength,
        NxOutputFormat outputFormat,
        out NxBuffer buffer);

    private static byte[] InvokeSourceNativeCall(
        string source,
        string? fileName,
        NxOutputFormat outputFormat,
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
            outputFormat,
            out NxBuffer buffer);
        return CopyAndFreeBuffer(buffer);
    }

    private static byte[] InvokeProgramArtifactNativeCall(
        NxProgramArtifact programArtifact,
        NxOutputFormat outputFormat,
        EvalProgramArtifactCallback callback,
        out NxEvalStatus status)
    {
        ArgumentNullException.ThrowIfNull(programArtifact);

        NxNativeLibrary.EnsureLoaded();

        status = callback(
            programArtifact.SafeHandle,
            outputFormat,
            out NxBuffer buffer);
        return CopyAndFreeBuffer(buffer);
    }

    private static byte[] InvokeComponentInitProgramArtifactNativeCall(
        NxProgramArtifact programArtifact,
        string componentName,
        NxOutputFormat outputFormat,
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
            programArtifact.SafeHandle,
            componentNameBytes,
            (nuint)componentNameBytes.Length,
            payloadBytes,
            (nuint)payloadBytes.Length,
            outputFormat,
            out NxBuffer buffer);
        return CopyAndFreeBuffer(buffer);
    }

    private static byte[] InvokeComponentDispatchProgramArtifactNativeCall(
        NxProgramArtifact programArtifact,
        byte[] stateSnapshot,
        NxOutputFormat outputFormat,
        byte[]? actionsBytes,
        ComponentDispatchProgramArtifactCallback callback,
        out NxEvalStatus status)
    {
        ArgumentNullException.ThrowIfNull(programArtifact);
        ArgumentNullException.ThrowIfNull(stateSnapshot);

        NxNativeLibrary.EnsureLoaded();

        byte[] payloadBytes = actionsBytes ?? Array.Empty<byte>();

        status = callback(
            programArtifact.SafeHandle,
            stateSnapshot,
            (nuint)stateSnapshot.Length,
            payloadBytes,
            (nuint)payloadBytes.Length,
            outputFormat,
            out NxBuffer buffer);
        return CopyAndFreeBuffer(buffer);
    }

    internal static NxEvaluationException CreateEvaluationException(
        byte[] payload,
        NxOutputFormat outputFormat)
    {
        return outputFormat switch
        {
            NxOutputFormat.MessagePack => CreateEvaluationExceptionFromMessagePack(payload),
            NxOutputFormat.Json => CreateEvaluationExceptionFromJson(payload),
            _ => throw new InvalidOperationException(
                $"NX native runtime returned an unsupported output format: {outputFormat}."),
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

    private static JsonElement DeserializeJsonElement(byte[] payload, string message)
    {
        try
        {
            using JsonDocument document = JsonDocument.Parse(payload);
            return document.RootElement.Clone();
        }
        catch (JsonException e)
        {
            throw new InvalidOperationException(message, e);
        }
    }

    private static NxComponentInitResult<JsonElement> DeserializeJsonComponentInitResult(
        byte[] payload,
        string message)
    {
        try
        {
            using JsonDocument document = JsonDocument.Parse(payload);
            JsonElement root = document.RootElement;
            if (!root.TryGetProperty("rendered", out JsonElement rendered))
            {
                throw new JsonException("Expected rendered property.");
            }

            if (!root.TryGetProperty("state_snapshot", out JsonElement stateSnapshotElement) ||
                stateSnapshotElement.ValueKind is not JsonValueKind.String)
            {
                throw new JsonException("Expected state_snapshot string property.");
            }

            string? stateSnapshotBase64 = stateSnapshotElement.GetString();
            if (stateSnapshotBase64 is null)
            {
                throw new JsonException("Expected state_snapshot string value.");
            }

            return new NxComponentInitResult<JsonElement>
            {
                Rendered = rendered.Clone(),
                StateSnapshot = Convert.FromBase64String(stateSnapshotBase64),
            };
        }
        catch (Exception e) when (e is JsonException or FormatException)
        {
            throw new InvalidOperationException(message, e);
        }
    }

    private static NxComponentDispatchResult<JsonElement> DeserializeJsonComponentDispatchResult(
        byte[] payload,
        string message)
    {
        try
        {
            using JsonDocument document = JsonDocument.Parse(payload);
            JsonElement root = document.RootElement;
            if (!root.TryGetProperty("effects", out JsonElement effectsElement) ||
                effectsElement.ValueKind is not JsonValueKind.Array)
            {
                throw new JsonException("Expected effects array property.");
            }

            if (!root.TryGetProperty("state_snapshot", out JsonElement stateSnapshotElement) ||
                stateSnapshotElement.ValueKind is not JsonValueKind.String)
            {
                throw new JsonException("Expected state_snapshot string property.");
            }

            string? stateSnapshotBase64 = stateSnapshotElement.GetString();
            if (stateSnapshotBase64 is null)
            {
                throw new JsonException("Expected state_snapshot string value.");
            }

            int effectCount = effectsElement.GetArrayLength();
            JsonElement[] effects = new JsonElement[effectCount];
            int index = 0;
            foreach (JsonElement effect in effectsElement.EnumerateArray())
            {
                effects[index++] = effect.Clone();
            }

            return new NxComponentDispatchResult<JsonElement>
            {
                Effects = effects,
                StateSnapshot = Convert.FromBase64String(stateSnapshotBase64),
            };
        }
        catch (Exception e) when (e is JsonException or FormatException)
        {
            throw new InvalidOperationException(message, e);
        }
    }

    private static byte[] SerializeMessagePackInput<T>(T value)
    {
        return value is null
            ? Array.Empty<byte>()
            : MessagePackSerializer.Serialize(value, MessagePackOptions);
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
