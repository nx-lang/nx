// Copyright (c) Bret Johnson. All rights reserved.
// Licensed under the MIT license. See LICENSE file in the project root for full license information.

using System;
using System.Runtime.InteropServices;

namespace NxLang.Nx;

[StructLayout(LayoutKind.Sequential)]
internal struct NxBuffer
{
    public IntPtr Ptr;
    public UIntPtr Len;
    public UIntPtr Cap;
}

