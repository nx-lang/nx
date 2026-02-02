// Copyright (c) Bret Johnson. All rights reserved.
// Licensed under the MIT license. See LICENSE file in the project root for full license information.

using System.Runtime.InteropServices;

namespace NxLang.Nx;

internal static class NxNativeMethods
{
    private const string LibraryName = "nx_ffi";

    [DllImport(LibraryName, CallingConvention = CallingConvention.Cdecl)]
    internal static extern NxEvalStatus nx_eval_source_msgpack(
        byte[] sourcePtr,
        nuint sourceLen,
        byte[] fileNamePtr,
        nuint fileNameLen,
        out NxBuffer outBuffer);

    [DllImport(LibraryName, CallingConvention = CallingConvention.Cdecl)]
    internal static extern NxEvalStatus nx_eval_source_json(
        byte[] sourcePtr,
        nuint sourceLen,
        byte[] fileNamePtr,
        nuint fileNameLen,
        out NxBuffer outBuffer);

    [DllImport(LibraryName, CallingConvention = CallingConvention.Cdecl)]
    internal static extern void nx_free_buffer(NxBuffer buffer);
}

