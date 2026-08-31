// Copyright (c) Bret Johnson. All rights reserved.
// Licensed under the MIT license. See LICENSE file in the project root for full license information.

using System;

namespace NxLang.Nx.Serialization;

/// <summary>
/// Marks a generated union case that declares no fields in a union that declares no base.
/// </summary>
/// <remarks>
/// <para>Such a case carries nothing beyond its own name, so its wire form is the bare case name
/// rather than a <c>$type</c> map. The attribute is what tells the serializers which shape to
/// write and to accept; it is not inferred from the type having no properties, because an empty
/// map is a legitimate shape for a case that simply has no fields of its own.</para>
/// </remarks>
[AttributeUsage(AttributeTargets.Class, AllowMultiple = false, Inherited = false)]
public sealed class NxConstantCaseAttribute : Attribute
{
    /// <summary>
    /// Initializes a new instance of the <see cref="NxConstantCaseAttribute"/> class.
    /// </summary>
    /// <param name="wireName">The authored case name, which is this case's entire wire form.</param>
    public NxConstantCaseAttribute(string wireName)
    {
        WireName = wireName;
    }

    /// <summary>Gets the authored case name, which is this case's entire wire form.</summary>
    public string WireName { get; }
}
