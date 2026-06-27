// Copyright (c) Bret Johnson. All rights reserved.
// Licensed under the MIT license. See LICENSE file in the project root for full license information.

using System;

namespace NxLang.Nx.Interop;

internal static class NxNativeLibraryInfo
{
    internal static string GetFileName()
    {
        if (OperatingSystem.IsWindows())
        {
            return "nx_ffi.dll";
        }

        if (OperatingSystem.IsMacOS())
        {
            return "libnx_ffi.dylib";
        }

        return "libnx_ffi.so";
    }

    internal static StringComparer GetPathComparer()
    {
        return OperatingSystem.IsWindows() ? StringComparer.OrdinalIgnoreCase : StringComparer.Ordinal;
    }
}
