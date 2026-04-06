// Copyright (c) Bret Johnson. All rights reserved.
// Licensed under the MIT license. See LICENSE file in the project root for full license information.

using System;
using System.Runtime.InteropServices;

namespace NxLang.Nx.Interop;

internal sealed class NxProgramArtifactSafeHandle : SafeHandle
{
    internal NxProgramArtifactSafeHandle()
        : base(IntPtr.Zero, ownsHandle: true)
    {
    }

    internal NxProgramArtifactSafeHandle(IntPtr handle)
        : this()
    {
        SetHandle(handle);
    }

    public override bool IsInvalid => handle == IntPtr.Zero;

    protected override bool ReleaseHandle()
    {
        if (!IsInvalid)
        {
            NxNativeMethods.nx_free_program_artifact(handle);
        }

        return true;
    }
}
