// Copyright (c) Bret Johnson. All rights reserved.
// Licensed under the MIT license. See LICENSE file in the project root for full license information.

using System.Text.Json;
using MessagePack;
using NxLang.Sdk.Tests.Generated;
using Xunit;

/// <summary>
/// The generated <c>User_property</c> enum is the <c>User.Property</c> companion typegen emits from
/// <c>Generated/update-records.nx</c>: one member per field of <c>User</c>, serialized as the bare field name.
/// </summary>
public class NxPropertyReferenceTests
{
    [Fact]
    public void GeneratedPropertyEnum_RoundTripsBareFieldNameThroughJson()
    {
        string json = JsonSerializer.Serialize(User_property.Email);

        Assert.Equal("\"email\"", json);
        Assert.Equal(User_property.Email, JsonSerializer.Deserialize<User_property>(json));
        Assert.Equal(User_property.Name, JsonSerializer.Deserialize<User_property>("\"name\""));
    }

    [Fact]
    public void GeneratedPropertyEnum_RoundTripsBareFieldNameThroughMessagePack()
    {
        byte[] payload = MessagePackSerializer.Serialize(
            User_property.Email,
            cancellationToken: TestContext.Current.CancellationToken);

        Assert.Equal(
            "email",
            MessagePackSerializer.Deserialize<string>(
                payload,
                cancellationToken: TestContext.Current.CancellationToken));
        Assert.Equal(
            User_property.Email,
            MessagePackSerializer.Deserialize<User_property>(
                payload,
                cancellationToken: TestContext.Current.CancellationToken));
    }

    [Fact]
    public void PropertyTypedField_CarriesTheBareFieldNameInsideARecord()
    {
        Form form = new()
        {
            Pending = null,
            Drafts = new User_update[0],
            SortBy = User_property.Name,
        };

        string json = JsonSerializer.Serialize(form);
        Assert.Contains("\"sortBy\":\"name\"", json);
        Form parsed = JsonSerializer.Deserialize<Form>(json)!;
        Assert.Equal(User_property.Name, parsed.SortBy);

        byte[] payload = MessagePackSerializer.Serialize(
            form,
            cancellationToken: TestContext.Current.CancellationToken);
        Form decoded = MessagePackSerializer.Deserialize<Form>(
            payload,
            cancellationToken: TestContext.Current.CancellationToken);
        Assert.Equal(User_property.Name, decoded.SortBy);
    }
}
