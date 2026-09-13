// Copyright (c) Bret Johnson. All rights reserved.
// Licensed under the MIT license. See LICENSE file in the project root for full license information.

using System.Text.Json;
using MessagePack;
using NxLang.Nx;
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

    [Fact]
    public void PropertyKey_ReadsAndWritesTheFieldOfARecord()
    {
        User user = new() { Name = "Ada" };

        UserProperties.Email.Set(user, "x@y");

        Assert.Equal("x@y", user.Email);
        Assert.Equal("x@y", UserProperties.Email.Get(user));
        Assert.Equal("Ada", UserProperties.Name.GetValue(user));
        UserProperties.Name.SetValue(user, "Grace");
        Assert.Equal("Grace", user.Name);
        Assert.Equal("email", UserProperties.Email.Name);
        Assert.Equal(typeof(string), UserProperties.Email.ValueType);
    }

    /// <summary>
    /// The key carries the field's value type, so reading and writing through it needs no cast and a mistyped
    /// value does not compile: <c>update.Set(UserProperties.Name, 42)</c> is rejected by the compiler.
    /// </summary>
    [Fact]
    public void IndexingAPatchByATypedKey_PreservesTheFieldType()
    {
        User_update update = new();

        update.Set(UserProperties.Name, "Ada");
        NxOptional<string?> email = update.Get(UserProperties.Email);
        NxOptional<string> name = update.Get(UserProperties.Name);

        Assert.False(email.HasValue);
        Assert.Equal("Ada", name.Value);
        Assert.Equal(new[] { "name" }, update.Fields.Keys);
        Assert.True(update.IsSet(UserProperties.Name));
        Assert.False(update.IsSet(UserProperties.Email));

        update.Unset(UserProperties.Name);
        Assert.Empty(update.Fields);
    }

    [Fact]
    public void PropertyCompanionValue_ResolvesToItsKey()
    {
        User_property decoded = JsonSerializer.Deserialize<User_property>("\"email\"");
        User_update update = new() { Email = null };

        NxProperty<User> key = UserProperties.Of(decoded);

        Assert.Same(UserProperties.Email, key);
        Assert.True(update.IsSet(key));
        Assert.True(update.IsSet(decoded));
        Assert.False(update.IsSet(UserProperties.Of(User_property.Name)));
    }
}
