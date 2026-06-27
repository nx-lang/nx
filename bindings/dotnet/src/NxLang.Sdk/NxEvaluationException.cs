// Copyright (c) Bret Johnson. All rights reserved.
// Licensed under the MIT license. See LICENSE file in the project root for full license information.

using System;

namespace NxLang.Nx;

/// <summary>
/// Exception thrown when NX validation, build, IR generation, or evaluation fails with structured diagnostics.
/// </summary>
public sealed class NxEvaluationException : Exception
{
    /// <summary>
    /// Initializes a new instance of the <see cref="NxEvaluationException"/> class.
    /// </summary>
    /// <param name="message">The error message that explains the reason for the exception.</param>
    /// <param name="diagnostics">Diagnostics reported by NX for the failed operation.</param>
    public NxEvaluationException(string message, NxDiagnostic[] diagnostics)
        : base(message)
    {
        Diagnostics = diagnostics;
    }

    /// <summary>
    /// Gets the diagnostics reported by NX for the failed operation.
    /// </summary>
    public NxDiagnostic[] Diagnostics { get; }
}
