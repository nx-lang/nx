// Copyright (c) Bret Johnson. All rights reserved.
// Licensed under the MIT license. See LICENSE file in the project root for full license information.

namespace NxLang.Nx;

internal enum NxEvalStatus : uint
{
    Ok = 0,
    Error = 1,
    InvalidArgument = 2,
    Panic = 255,
}

