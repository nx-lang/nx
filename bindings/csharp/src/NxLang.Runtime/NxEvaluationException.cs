// Copyright (c) Bret Johnson. All rights reserved.
// Licensed under the MIT license. See LICENSE file in the project root for full license information.

using System;

namespace NxLang.Nx;

/// <summary>
/// Exception thrown when NX source code evaluation fails due to syntax errors, missing root function, or runtime errors.
/// </summary>
public sealed class NxEvaluationException : Exception
{
    /// <summary>
    /// Initializes a new instance of the <see cref="NxEvaluationException"/> class.
    /// </summary>
    /// <param name="message">The error message that explains the reason for the exception.</param>
    /// <param name="diagnostics">The array of diagnostics containing detailed error information.</param>
    public NxEvaluationException(string message, NxDiagnostic[] diagnostics)
        : base(message)
    {
        Diagnostics = diagnostics;
    }

    /// <summary>
    /// Gets the array of diagnostics containing detailed information about the evaluation failure.
    /// </summary>
    public NxDiagnostic[] Diagnostics { get; }
}

