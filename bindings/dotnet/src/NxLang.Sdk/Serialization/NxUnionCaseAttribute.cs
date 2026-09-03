// Copyright (c) Bret Johnson. All rights reserved.
// Licensed under the MIT license. See LICENSE file in the project root for full license information.

using System;
using System.Text.Json.Serialization;

namespace NxLang.Nx.Serialization;

/// <summary>
/// Registers one case of a discriminated union that has more than one wire shape.
/// </summary>
/// <remarks>
/// <para>A union with both constant and payload cases cannot use <see cref="JsonDerivedTypeAttribute"/>:
/// System.Text.Json refuses to combine its polymorphism metadata with a custom converter on the
/// same base type, and a custom converter is exactly what reading two shapes requires. Such a
/// union carries these registrations instead, and <see cref="NxPolymorphicJsonConverter{TBase}"/>
/// and <see cref="NxPolymorphicMessagePackFormatter{TBase}"/> both read them.</para>
/// <para>A union whose cases all share the <c>$type</c> shape keeps
/// <see cref="JsonDerivedTypeAttribute"/> and needs nothing here.</para>
/// </remarks>
[AttributeUsage(AttributeTargets.Class, AllowMultiple = true, Inherited = false)]
public sealed class NxUnionCaseAttribute : Attribute
{
    /// <summary>
    /// Initializes a new instance of the <see cref="NxUnionCaseAttribute"/> class.
    /// </summary>
    /// <param name="caseType">The concrete case type.</param>
    /// <param name="discriminator">The case's <c>$type</c> discriminator, as <c>Union.case</c>.</param>
    public NxUnionCaseAttribute(Type caseType, string discriminator)
    {
        CaseType = caseType;
        Discriminator = discriminator;
    }

    /// <summary>Gets the concrete case type.</summary>
    public Type CaseType { get; }

    /// <summary>Gets the case's <c>$type</c> discriminator.</summary>
    public string Discriminator { get; }
}
