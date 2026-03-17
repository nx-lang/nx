// Copyright (c) Bret Johnson. All rights reserved.
// Licensed under the MIT license. See LICENSE file in the project root for full license information.

using System.Runtime.InteropServices;

namespace NxLang.Nx.Interop;

internal static class NxNativeMethods
{
    internal const string LibraryName = "nx_ffi";

    [DllImport(LibraryName, CallingConvention = CallingConvention.Cdecl, ExactSpelling = true)]
    internal static extern uint nx_ffi_abi_version();

    [DllImport(LibraryName, CallingConvention = CallingConvention.Cdecl, ExactSpelling = true)]
    internal static extern NxEvalStatus nx_eval_source(
        byte[] sourcePtr,
        nuint sourceLen,
        byte[] fileNamePtr,
        nuint fileNameLen,
        out NxBuffer outBuffer);

    [DllImport(LibraryName, CallingConvention = CallingConvention.Cdecl, ExactSpelling = true)]
    internal static extern NxEvalStatus nx_component_init(
        byte[] sourcePtr,
        nuint sourceLen,
        byte[] fileNamePtr,
        nuint fileNameLen,
        byte[] componentNamePtr,
        nuint componentNameLen,
        byte[] propsPtr,
        nuint propsLen,
        out NxBuffer outBuffer);

    [DllImport(LibraryName, CallingConvention = CallingConvention.Cdecl, ExactSpelling = true)]
    internal static extern NxEvalStatus nx_component_dispatch_actions(
        byte[] sourcePtr,
        nuint sourceLen,
        byte[] fileNamePtr,
        nuint fileNameLen,
        byte[] stateSnapshotPtr,
        nuint stateSnapshotLen,
        byte[] actionsPtr,
        nuint actionsLen,
        out NxBuffer outBuffer);

    [DllImport(LibraryName, CallingConvention = CallingConvention.Cdecl, ExactSpelling = true)]
    internal static extern NxEvalStatus nx_value_msgpack_to_json(
        byte[] payloadPtr,
        nuint payloadLen,
        out NxBuffer outBuffer);

    [DllImport(LibraryName, CallingConvention = CallingConvention.Cdecl, ExactSpelling = true)]
    internal static extern NxEvalStatus nx_diagnostics_msgpack_to_json(
        byte[] payloadPtr,
        nuint payloadLen,
        out NxBuffer outBuffer);

    [DllImport(LibraryName, CallingConvention = CallingConvention.Cdecl, ExactSpelling = true)]
    internal static extern NxEvalStatus nx_component_init_result_msgpack_to_json(
        byte[] payloadPtr,
        nuint payloadLen,
        out NxBuffer outBuffer);

    [DllImport(LibraryName, CallingConvention = CallingConvention.Cdecl, ExactSpelling = true)]
    internal static extern NxEvalStatus nx_component_dispatch_result_msgpack_to_json(
        byte[] payloadPtr,
        nuint payloadLen,
        out NxBuffer outBuffer);

    [DllImport(LibraryName, CallingConvention = CallingConvention.Cdecl, ExactSpelling = true)]
    internal static extern void nx_free_buffer(NxBuffer buffer);
}
