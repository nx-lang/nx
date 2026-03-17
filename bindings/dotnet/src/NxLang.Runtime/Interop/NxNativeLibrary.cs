// Copyright (c) Bret Johnson. All rights reserved.
// Licensed under the MIT license. See LICENSE file in the project root for full license information.

using System;
using System.Collections.Generic;
using System.Diagnostics.CodeAnalysis;
using System.IO;
using System.Reflection;
using System.Runtime.ExceptionServices;
using System.Runtime.InteropServices;

namespace NxLang.Nx.Interop;

internal static class NxNativeLibrary
{
    internal const uint SupportedAbiVersion = 2;

    private static readonly object SyncRoot = new();
    private static Exception? _loadException;
    private static bool _initialized;

    static NxNativeLibrary()
    {
        NativeLibrary.SetDllImportResolver(typeof(NxNativeLibrary).Assembly, Resolve);
    }

    internal static void EnsureLoaded()
    {
        if (_initialized)
        {
            return;
        }

        if (_loadException is not null)
        {
            ThrowStoredException(_loadException);
        }

        lock (SyncRoot)
        {
            if (_initialized)
            {
                return;
            }

            if (_loadException is not null)
            {
                ThrowStoredException(_loadException);
            }

            try
            {
                uint abiVersion = NxNativeMethods.nx_ffi_abi_version();
                if (abiVersion != SupportedAbiVersion)
                {
                    _loadException = CreateAbiVersionException(abiVersion);
                    ThrowStoredException(_loadException);
                }

                _initialized = true;
            }
            catch (DllNotFoundException e)
            {
                _loadException = CreateMissingLibraryException(e);
            }
            catch (BadImageFormatException e)
            {
                _loadException = CreateIncompatibleLibraryException(e);
            }
            catch (EntryPointNotFoundException e)
            {
                _loadException = CreateIncompatibleLibraryException(e);
            }

            if (_loadException is not null)
            {
                ThrowStoredException(_loadException);
            }
        }
    }

    private static IntPtr Resolve(string libraryName, Assembly assembly, DllImportSearchPath? searchPath)
    {
        if (!string.Equals(libraryName, NxNativeMethods.LibraryName, StringComparison.Ordinal))
        {
            return IntPtr.Zero;
        }

        string nativeFileName = NxNativeLibraryInfo.GetFileName();
        foreach (string directory in GetSearchDirectories(assembly))
        {
            string candidatePath = Path.Combine(directory, nativeFileName);
            if (NativeLibrary.TryLoad(candidatePath, out IntPtr handle))
            {
                return handle;
            }
        }

        return IntPtr.Zero;
    }

    private static IEnumerable<string> GetSearchDirectories(Assembly assembly)
    {
        HashSet<string> directories = new(NxNativeLibraryInfo.GetPathComparer());
        AddSearchDirectory(directories, AppContext.BaseDirectory);
        AddSearchDirectory(directories, Path.GetDirectoryName(assembly.Location));
        return directories;
    }

    private static void AddSearchDirectory(HashSet<string> directories, string? path)
    {
        if (string.IsNullOrWhiteSpace(path))
        {
            return;
        }

        directories.Add(Path.GetFullPath(path));
    }

    private static InvalidOperationException CreateAbiVersionException(uint actualAbiVersion)
    {
        return new InvalidOperationException(
            $"NX native runtime ABI mismatch. Managed binding expects ABI {SupportedAbiVersion}, but loaded ABI {actualAbiVersion}. Rebuild `crates/nx-ffi` from the same NX source revision as `NxLang.Runtime.dll`.");
    }

    private static InvalidOperationException CreateMissingLibraryException(DllNotFoundException innerException)
    {
        return new InvalidOperationException(
            "NX native runtime could not be found. Build `crates/nx-ffi` and stage the native library next to the application output, or import `bindings/dotnet/build/NxLang.Runtime.targets` when consuming NX from a vendored source checkout.",
            innerException);
    }

    private static InvalidOperationException CreateIncompatibleLibraryException(Exception innerException)
    {
        return new InvalidOperationException(
            "NX native runtime could not be loaded. Ensure the staged native library matches the current platform and comes from the same NX source revision as `NxLang.Runtime.dll`.",
            innerException);
    }

    [DoesNotReturn]
    private static void ThrowStoredException(Exception exception)
    {
        ExceptionDispatchInfo.Capture(exception).Throw();
        throw new InvalidOperationException("Unreachable code.");
    }
}
