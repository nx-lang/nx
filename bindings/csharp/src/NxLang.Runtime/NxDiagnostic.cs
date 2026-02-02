// Copyright (c) Bret Johnson. All rights reserved.
// Licensed under the MIT license. See LICENSE file in the project root for full license information.

using System;
using System.Text.Json.Serialization;
using MessagePack;

namespace NxLang.Nx;

/// <summary>
/// Represents a diagnostic message (error, warning, or info) from the NX language runtime.
/// </summary>
[MessagePackObject]
public sealed class NxDiagnostic
{
    /// <summary>
    /// Gets or sets the severity level of the diagnostic (e.g., "error", "warning", "info").
    /// </summary>
    [Key("severity")]
    [JsonPropertyName("severity")]
    public string Severity { get; set; } = string.Empty;

    /// <summary>
    /// Gets or sets the diagnostic code, if available. Used to identify the specific type of diagnostic.
    /// </summary>
    [Key("code")]
    [JsonPropertyName("code")]
    public string? Code { get; set; }

    /// <summary>
    /// Gets or sets the main diagnostic message describing the issue.
    /// </summary>
    [Key("message")]
    [JsonPropertyName("message")]
    public string Message { get; set; } = string.Empty;

    /// <summary>
    /// Gets or sets the labels that point to specific locations in the source code related to this diagnostic.
    /// </summary>
    [Key("labels")]
    [JsonPropertyName("labels")]
    public NxDiagnosticLabel[] Labels { get; set; } = Array.Empty<NxDiagnosticLabel>();

    /// <summary>
    /// Gets or sets an optional help message providing additional context or suggestions for resolving the issue.
    /// </summary>
    [Key("help")]
    [JsonPropertyName("help")]
    public string? Help { get; set; }

    /// <summary>
    /// Gets or sets an optional note providing additional information about the diagnostic.
    /// </summary>
    [Key("note")]
    [JsonPropertyName("note")]
    public string? Note { get; set; }
}

/// <summary>
/// Represents a label that points to a specific location in the source code related to a diagnostic.
/// </summary>
[MessagePackObject]
public sealed class NxDiagnosticLabel
{
    /// <summary>
    /// Gets or sets the file name where this diagnostic label is located.
    /// </summary>
    [Key("file")]
    [JsonPropertyName("file")]
    public string File { get; set; } = string.Empty;

    /// <summary>
    /// Gets or sets the text span indicating the location in the source code.
    /// </summary>
    [Key("span")]
    [JsonPropertyName("span")]
    public NxTextSpan Span { get; set; } = new();

    /// <summary>
    /// Gets or sets an optional message specific to this label location.
    /// </summary>
    [Key("message")]
    [JsonPropertyName("message")]
    public string? Message { get; set; }

    /// <summary>
    /// Gets or sets a value indicating whether this is the primary label for the diagnostic. The primary label typically
    /// indicates the main location of the issue.
    /// </summary>
    [Key("primary")]
    [JsonPropertyName("primary")]
    public bool Primary { get; set; }
}

/// <summary>
/// Represents a span of text in a source file, including both byte offsets and line/column positions.
/// </summary>
[MessagePackObject]
public sealed class NxTextSpan
{
    /// <summary>
    /// Gets or sets the starting byte offset of the span in the source file.
    /// </summary>
    [Key("start_byte")]
    [JsonPropertyName("start_byte")]
    public uint StartByte { get; set; }

    /// <summary>
    /// Gets or sets the ending byte offset of the span in the source file.
    /// </summary>
    [Key("end_byte")]
    [JsonPropertyName("end_byte")]
    public uint EndByte { get; set; }

    /// <summary>
    /// Gets or sets the starting line number (0-based) of the span.
    /// </summary>
    [Key("start_line")]
    [JsonPropertyName("start_line")]
    public uint StartLine { get; set; }

    /// <summary>
    /// Gets or sets the starting column number (0-based) of the span.
    /// </summary>
    [Key("start_column")]
    [JsonPropertyName("start_column")]
    public uint StartColumn { get; set; }

    /// <summary>
    /// Gets or sets the ending line number (0-based) of the span.
    /// </summary>
    [Key("end_line")]
    [JsonPropertyName("end_line")]
    public uint EndLine { get; set; }

    /// <summary>
    /// Gets or sets the ending column number (0-based) of the span.
    /// </summary>
    [Key("end_column")]
    [JsonPropertyName("end_column")]
    public uint EndColumn { get; set; }
}
