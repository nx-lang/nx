// Copyright (c) Bret Johnson. All rights reserved.
// Licensed under the MIT license. See LICENSE file in the project root for full license information.

using System;
using System.Runtime.InteropServices;

namespace NxLang.Nx.Interop;

/// <summary>
/// One borrowed UTF-8 string, as the native API takes a workspace identity.
/// </summary>
[StructLayout(LayoutKind.Sequential)]
internal struct NxUtf8Slice
{
    internal IntPtr Ptr;
    internal nuint Len;
}
