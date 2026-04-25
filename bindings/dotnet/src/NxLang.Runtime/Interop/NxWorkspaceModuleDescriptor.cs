// Copyright (c) Bret Johnson. All rights reserved.
// Licensed under the MIT license. See LICENSE file in the project root for full license information.

using System;
using System.Runtime.InteropServices;

namespace NxLang.Nx.Interop;

[StructLayout(LayoutKind.Sequential)]
internal struct NxWorkspaceModuleDescriptor
{
    internal IntPtr IdentityPtr;
    internal nuint IdentityLen;
    internal IntPtr SourceUtf8Ptr;
    internal nuint SourceUtf8Len;
}
