using System;
using System.IO;
using System.Text;
using System.Collections.Generic;
using System.Runtime.InteropServices;

namespace Wasmtime
{
    /// <summary>
    /// Represents a WebAssembly host environment.
    /// </summary>
    /// <remarks>
    /// A host is used to configure the environment for WebAssembly modules to execute in.
    /// </remarks>
    public class Host : IDisposable
    {
        /// <summary>
        /// Constructs a new host.
        /// </summary>
        public Host()
        {
            Initialize(Interop.wasm_engine_new());
        }

        /// <summary>
        /// Defines a WASI implementation in the host.
        /// </summary>
        /// <param name="name">The name of the WASI module to define.</param>
        /// <param name="config">The <see cref="WasiConfiguration"/> to configure the WASI implementation with.</param>
        public void DefineWasi(string name, WasiConfiguration config = null)
        {
            CheckDisposed();

            if (string.IsNullOrEmpty(name))
            {
                throw new ArgumentException("Name cannot be null or empty.", nameof(name));
            }

            if (config is null)
            {
                config = new WasiConfiguration();
            }

            using var wasi = config.CreateWasi(Store, name);

            if (!Interop.wasmtime_linker_define_wasi(Linker, wasi))
            {
                throw new WasmtimeException($"Failed to define WASI module '{name}'.");
            }
        }

        /// <summary>
        /// Defines a host function.
        /// </summary>
        /// <param name="moduleName">The module name of the host function.</param>
        /// <param name="name">The name of the host function.</param>
        /// <param name="func">The callback for when the host function is invoked.</param>
        /// <returns>Returns a <see cref="Function"/> representing the host function.</returns>
        public Function DefineFunction(string moduleName, string name, Action func)
        {
            return DefineFunction(moduleName, name, func, false);
        }

        /// <summary>
        /// Defines a host function.
        /// </summary>
        /// <param name="moduleName">The module name of the host function.</param>
        /// <param name="name">The name of the host function.</param>
        /// <param name="func">The callback for when the host function is invoked.</param>
        /// <returns>Returns a <see cref="Function"/> representing the host function.</returns>
        public Function DefineFunction<T>(string moduleName, string name, Action<T> func)
        {
            return DefineFunction(moduleName, name, func, false);
        }

        /// <summary>
        /// Defines a host function.
        /// </summary>
        /// <param name="moduleName">The module name of the host function.</param>
        /// <param name="name">The name of the host function.</param>
        /// <param name="func">The callback for when the host function is invoked.</param>
        /// <returns>Returns a <see cref="Function"/> representing the host function.</returns>
        public Function DefineFunction<T1, T2>(string moduleName, string name, Action<T1, T2> func)
        {
            return DefineFunction(moduleName, name, func, false);
        }

        /// <summary>
        /// Defines a host function.
        /// </summary>
        /// <param name="moduleName">The module name of the host function.</param>
        /// <param name="name">The name of the host function.</param>
        /// <param name="func">The callback for when the host function is invoked.</param>
        /// <returns>Returns a <see cref="Function"/> representing the host function.</returns>
        public Function DefineFunction<T1, T2, T3>(string moduleName, string name, Action<T1, T2, T3> func)
        {
            return DefineFunction(moduleName, name, func, false);
        }

        /// <summary>
        /// Defines a host function.
        /// </summary>
        /// <param name="moduleName">The module name of the host function.</param>
        /// <param name="name">The name of the host function.</param>
        /// <param name="func">The callback for when the host function is invoked.</param>
        /// <returns>Returns a <see cref="Function"/> representing the host function.</returns>
        public Function DefineFunction<T1, T2, T3, T4>(string moduleName, string name, Action<T1, T2, T3, T4> func)
        {
            return DefineFunction(moduleName, name, func, false);
        }

        /// <summary>
        /// Defines a host function.
        /// </summary>
        /// <param name="moduleName">The module name of the host function.</param>
        /// <param name="name">The name of the host function.</param>
        /// <param name="func">The callback for when the host function is invoked.</param>
        /// <returns>Returns a <see cref="Function"/> representing the host function.</returns>
        public Function DefineFunction<T1, T2, T3, T4, T5>(string moduleName, string name, Action<T1, T2, T3, T4, T5> func)
        {
            return DefineFunction(moduleName, name, func, false);
        }

        /// <summary>
        /// Defines a host function.
        /// </summary>
        /// <param name="moduleName">The module name of the host function.</param>
        /// <param name="name">The name of the host function.</param>
        /// <param name="func">The callback for when the host function is invoked.</param>
        /// <returns>Returns a <see cref="Function"/> representing the host function.</returns>
        public Function DefineFunction<T1, T2, T3, T4, T5, T6>(string moduleName, string name, Action<T1, T2, T3, T4, T5, T6> func)
        {
            return DefineFunction(moduleName, name, func, false);
        }

        /// <summary>
        /// Defines a host function.
        /// </summary>
        /// <param name="moduleName">The module name of the host function.</param>
        /// <param name="name">The name of the host function.</param>
        /// <param name="func">The callback for when the host function is invoked.</param>
        /// <returns>Returns a <see cref="Function"/> representing the host function.</returns>
        public Function DefineFunction<T1, T2, T3, T4, T5, T6, T7>(string moduleName, string name, Action<T1, T2, T3, T4, T5, T6, T7> func)
        {
            return DefineFunction(moduleName, name, func, false);
        }

        /// <summary>
        /// Defines a host function.
        /// </summary>
        /// <param name="moduleName">The module name of the host function.</param>
        /// <param name="name">The name of the host function.</param>
        /// <param name="func">The callback for when the host function is invoked.</param>
        /// <returns>Returns a <see cref="Function"/> representing the host function.</returns>
        public Function DefineFunction<T1, T2, T3, T4, T5, T6, T7, T8>(string moduleName, string name, Action<T1, T2, T3, T4, T5, T6, T7, T8> func)
        {
            return DefineFunction(moduleName, name, func, false);
        }

        /// <summary>
        /// Defines a host function.
        /// </summary>
        /// <param name="moduleName">The module name of the host function.</param>
        /// <param name="name">The name of the host function.</param>
        /// <param name="func">The callback for when the host function is invoked.</param>
        /// <returns>Returns a <see cref="Function"/> representing the host function.</returns>
        public Function DefineFunction<T1, T2, T3, T4, T5, T6, T7, T8, T9>(string moduleName, string name, Action<T1, T2, T3, T4, T5, T6, T7, T8, T9> func)
        {
            return DefineFunction(moduleName, name, func, false);
        }

        /// <summary>
        /// Defines a host function.
        /// </summary>
        /// <param name="moduleName">The module name of the host function.</param>
        /// <param name="name">The name of the host function.</param>
        /// <param name="func">The callback for when the host function is invoked.</param>
        /// <returns>Returns a <see cref="Function"/> representing the host function.</returns>
        public Function DefineFunction<T1, T2, T3, T4, T5, T6, T7, T8, T9, T10>(string moduleName, string name, Action<T1, T2, T3, T4, T5, T6, T7, T8, T9, T10> func)
        {
            return DefineFunction(moduleName, name, func, false);
        }

        /// <summary>
        /// Defines a host function.
        /// </summary>
        /// <param name="moduleName">The module name of the host function.</param>
        /// <param name="name">The name of the host function.</param>
        /// <param name="func">The callback for when the host function is invoked.</param>
        /// <returns>Returns a <see cref="Function"/> representing the host function.</returns>
        public Function DefineFunction<T1, T2, T3, T4, T5, T6, T7, T8, T9, T10, T11>(string moduleName, string name, Action<T1, T2, T3, T4, T5, T6, T7, T8, T9, T10, T11> func)
        {
            return DefineFunction(moduleName, name, func, false);
        }

        /// <summary>
        /// Defines a host function.
        /// </summary>
        /// <param name="moduleName">The module name of the host function.</param>
        /// <param name="name">The name of the host function.</param>
        /// <param name="func">The callback for when the host function is invoked.</param>
        /// <returns>Returns a <see cref="Function"/> representing the host function.</returns>
        public Function DefineFunction<T1, T2, T3, T4, T5, T6, T7, T8, T9, T10, T11, T12>(string moduleName, string name, Action<T1, T2, T3, T4, T5, T6, T7, T8, T9, T10, T11, T12> func)
        {
            return DefineFunction(moduleName, name, func, false);
        }

        /// <summary>
        /// Defines a host function.
        /// </summary>
        /// <param name="moduleName">The module name of the host function.</param>
        /// <param name="name">The name of the host function.</param>
        /// <param name="func">The callback for when the host function is invoked.</param>
        /// <returns>Returns a <see cref="Function"/> representing the host function.</returns>
        public Function DefineFunction<T1, T2, T3, T4, T5, T6, T7, T8, T9, T10, T11, T12, T13>(string moduleName, string name, Action<T1, T2, T3, T4, T5, T6, T7, T8, T9, T10, T11, T12, T13> func)
        {
            return DefineFunction(moduleName, name, func, false);
        }

        /// <summary>
        /// Defines a host function.
        /// </summary>
        /// <param name="moduleName">The module name of the host function.</param>
        /// <param name="name">The name of the host function.</param>
        /// <param name="func">The callback for when the host function is invoked.</param>
        /// <returns>Returns a <see cref="Function"/> representing the host function.</returns>
        public Function DefineFunction<T1, T2, T3, T4, T5, T6, T7, T8, T9, T10, T11, T12, T13, T14>(string moduleName, string name, Action<T1, T2, T3, T4, T5, T6, T7, T8, T9, T10, T11, T12, T13, T14> func)
        {
            return DefineFunction(moduleName, name, func, false);
        }

        /// <summary>
        /// Defines a host function.
        /// </summary>
        /// <param name="moduleName">The module name of the host function.</param>
        /// <param name="name">The name of the host function.</param>
        /// <param name="func">The callback for when the host function is invoked.</param>
        /// <returns>Returns a <see cref="Function"/> representing the host function.</returns>
        public Function DefineFunction<T1, T2, T3, T4, T5, T6, T7, T8, T9, T10, T11, T12, T13, T14, T15>(string moduleName, string name, Action<T1, T2, T3, T4, T5, T6, T7, T8, T9, T10, T11, T12, T13, T14, T15> func)
        {
            return DefineFunction(moduleName, name, func, false);
        }

        /// <summary>
        /// Defines a host function.
        /// </summary>
        /// <param name="moduleName">The module name of the host function.</param>
        /// <param name="name">The name of the host function.</param>
        /// <param name="func">The callback for when the host function is invoked.</param>
        /// <returns>Returns a <see cref="Function"/> representing the host function.</returns>
        public Function DefineFunction<T1, T2, T3, T4, T5, T6, T7, T8, T9, T10, T11, T12, T13, T14, T15, T16>(string moduleName, string name, Action<T1, T2, T3, T4, T5, T6, T7, T8, T9, T10, T11, T12, T13, T14, T15, T16> func)
        {
            return DefineFunction(moduleName, name, func, false);
        }

        /// <summary>
        /// Defines a host function.
        /// </summary>
        /// <param name="moduleName">The module name of the host function.</param>
        /// <param name="name">The name of the host function.</param>
        /// <param name="func">The callback for when the host function is invoked.</param>
        /// <returns>Returns a <see cref="Function"/> representing the host function.</returns>
        public Function DefineFunction<TResult>(string moduleName, string name, Func<TResult> func)
        {
            return DefineFunction(moduleName, name, func, true);
        }

        /// <summary>
        /// Defines a host function.
        /// </summary>
        /// <param name="moduleName">The module name of the host function.</param>
        /// <param name="name">The name of the host function.</param>
        /// <param name="func">The callback for when the host function is invoked.</param>
        /// <returns>Returns a <see cref="Function"/> representing the host function.</returns>
        public Function DefineFunction<T, TResult>(string moduleName, string name, Func<T, TResult> func)
        {
            return DefineFunction(moduleName, name, func, true);
        }

        /// <summary>
        /// Defines a host function.
        /// </summary>
        /// <param name="moduleName">The module name of the host function.</param>
        /// <param name="name">The name of the host function.</param>
        /// <param name="func">The callback for when the host function is invoked.</param>
        /// <returns>Returns a <see cref="Function"/> representing the host function.</returns>
        public Function DefineFunction<T1, T2, TResult>(string moduleName, string name, Func<T1, T2, TResult> func)
        {
            return DefineFunction(moduleName, name, func, true);
        }

        /// <summary>
        /// Defines a host function.
        /// </summary>
        /// <param name="moduleName">The module name of the host function.</param>
        /// <param name="name">The name of the host function.</param>
        /// <param name="func">The callback for when the host function is invoked.</param>
        /// <returns>Returns a <see cref="Function"/> representing the host function.</returns>
        public Function DefineFunction<T1, T2, T3, TResult>(string moduleName, string name, Func<T1, T2, T3, TResult> func)
        {
            return DefineFunction(moduleName, name, func, true);
        }

        /// <summary>
        /// Defines a host function.
        /// </summary>
        /// <param name="moduleName">The module name of the host function.</param>
        /// <param name="name">The name of the host function.</param>
        /// <param name="func">The callback for when the host function is invoked.</param>
        /// <returns>Returns a <see cref="Function"/> representing the host function.</returns>
        public Function DefineFunction<T1, T2, T3, T4, TResult>(string moduleName, string name, Func<T1, T2, T3, T4, TResult> func)
        {
            return DefineFunction(moduleName, name, func, true);
        }

        /// <summary>
        /// Defines a host function.
        /// </summary>
        /// <param name="moduleName">The module name of the host function.</param>
        /// <param name="name">The name of the host function.</param>
        /// <param name="func">The callback for when the host function is invoked.</param>
        /// <returns>Returns a <see cref="Function"/> representing the host function.</returns>
        public Function DefineFunction<T1, T2, T3, T4, T5, TResult>(string moduleName, string name, Func<T1, T2, T3, T4, T5, TResult> func)
        {
            return DefineFunction(moduleName, name, func, true);
        }

        /// <summary>
        /// Defines a host function.
        /// </summary>
        /// <param name="moduleName">The module name of the host function.</param>
        /// <param name="name">The name of the host function.</param>
        /// <param name="func">The callback for when the host function is invoked.</param>
        /// <returns>Returns a <see cref="Function"/> representing the host function.</returns>
        public Function DefineFunction<T1, T2, T3, T4, T5, T6, TResult>(string moduleName, string name, Func<T1, T2, T3, T4, T5, T6, TResult> func)
        {
            return DefineFunction(moduleName, name, func, true);
        }

        /// <summary>
        /// Defines a host function.
        /// </summary>
        /// <param name="moduleName">The module name of the host function.</param>
        /// <param name="name">The name of the host function.</param>
        /// <param name="func">The callback for when the host function is invoked.</param>
        /// <returns>Returns a <see cref="Function"/> representing the host function.</returns>
        public Function DefineFunction<T1, T2, T3, T4, T5, T6, T7, TResult>(string moduleName, string name, Func<T1, T2, T3, T4, T5, T6, T7, TResult> func)
        {
            return DefineFunction(moduleName, name, func, true);
        }

        /// <summary>
        /// Defines a host function.
        /// </summary>
        /// <param name="moduleName">The module name of the host function.</param>
        /// <param name="name">The name of the host function.</param>
        /// <param name="func">The callback for when the host function is invoked.</param>
        /// <returns>Returns a <see cref="Function"/> representing the host function.</returns>
        public Function DefineFunction<T1, T2, T3, T4, T5, T6, T7, T8, TResult>(string moduleName, string name, Func<T1, T2, T3, T4, T5, T6, T7, T8, TResult> func)
        {
            return DefineFunction(moduleName, name, func, true);
        }

        /// <summary>
        /// Defines a host function.
        /// </summary>
        /// <param name="moduleName">The module name of the host function.</param>
        /// <param name="name">The name of the host function.</param>
        /// <param name="func">The callback for when the host function is invoked.</param>
        /// <returns>Returns a <see cref="Function"/> representing the host function.</returns>
        public Function DefineFunction<T1, T2, T3, T4, T5, T6, T7, T8, T9, TResult>(string moduleName, string name, Func<T1, T2, T3, T4, T5, T6, T7, T8, T9, TResult> func)
        {
            return DefineFunction(moduleName, name, func, true);
        }

        /// <summary>
        /// Defines a host function.
        /// </summary>
        /// <param name="moduleName">The module name of the host function.</param>
        /// <param name="name">The name of the host function.</param>
        /// <param name="func">The callback for when the host function is invoked.</param>
        /// <returns>Returns a <see cref="Function"/> representing the host function.</returns>
        public Function DefineFunction<T1, T2, T3, T4, T5, T6, T7, T8, T9, T10, TResult>(string moduleName, string name, Func<T1, T2, T3, T4, T5, T6, T7, T8, T9, T10, TResult> func)
        {
            return DefineFunction(moduleName, name, func, true);
        }

        /// <summary>
        /// Defines a host function.
        /// </summary>
        /// <param name="moduleName">The module name of the host function.</param>
        /// <param name="name">The name of the host function.</param>
        /// <param name="func">The callback for when the host function is invoked.</param>
        /// <returns>Returns a <see cref="Function"/> representing the host function.</returns>
        public Function DefineFunction<T1, T2, T3, T4, T5, T6, T7, T8, T9, T10, T11, TResult>(string moduleName, string name, Func<T1, T2, T3, T4, T5, T6, T7, T8, T9, T10, T11, TResult> func)
        {
            return DefineFunction(moduleName, name, func, true);
        }

        /// <summary>
        /// Defines a host function.
        /// </summary>
        /// <param name="moduleName">The module name of the host function.</param>
        /// <param name="name">The name of the host function.</param>
        /// <param name="func">The callback for when the host function is invoked.</param>
        /// <returns>Returns a <see cref="Function"/> representing the host function.</returns>
        public Function DefineFunction<T1, T2, T3, T4, T5, T6, T7, T8, T9, T10, T11, T12, TResult>(string moduleName, string name, Func<T1, T2, T3, T4, T5, T6, T7, T8, T9, T10, T11, T12, TResult> func)
        {
            return DefineFunction(moduleName, name, func, true);
        }

        /// <summary>
        /// Defines a host function.
        /// </summary>
        /// <param name="moduleName">The module name of the host function.</param>
        /// <param name="name">The name of the host function.</param>
        /// <param name="func">The callback for when the host function is invoked.</param>
        /// <returns>Returns a <see cref="Function"/> representing the host function.</returns>
        public Function DefineFunction<T1, T2, T3, T4, T5, T6, T7, T8, T9, T10, T11, T12, T13, TResult>(string moduleName, string name, Func<T1, T2, T3, T4, T5, T6, T7, T8, T9, T10, T11, T12, T13, TResult> func)
        {
            return DefineFunction(moduleName, name, func, true);
        }

        /// <summary>
        /// Defines a host function.
        /// </summary>
        /// <param name="moduleName">The module name of the host function.</param>
        /// <param name="name">The name of the host function.</param>
        /// <param name="func">The callback for when the host function is invoked.</param>
        /// <returns>Returns a <see cref="Function"/> representing the host function.</returns>
        public Function DefineFunction<T1, T2, T3, T4, T5, T6, T7, T8, T9, T10, T11, T12, T13, T14, TResult>(string moduleName, string name, Func<T1, T2, T3, T4, T5, T6, T7, T8, T9, T10, T11, T12, T13, T14, TResult> func)
        {
            return DefineFunction(moduleName, name, func, true);
        }

        /// <summary>
        /// Defines a host function.
        /// </summary>
        /// <param name="moduleName">The module name of the host function.</param>
        /// <param name="name">The name of the host function.</param>
        /// <param name="func">The callback for when the host function is invoked.</param>
        /// <returns>Returns a <see cref="Function"/> representing the host function.</returns>
        public Function DefineFunction<T1, T2, T3, T4, T5, T6, T7, T8, T9, T10, T11, T12, T13, T14, T15, TResult>(string moduleName, string name, Func<T1, T2, T3, T4, T5, T6, T7, T8, T9, T10, T11, T12, T13, T14, T15, TResult> func)
        {
            return DefineFunction(moduleName, name, func, true);
        }

        /// <summary>
        /// Defines a host function.
        /// </summary>
        /// <param name="moduleName">The module name of the host function.</param>
        /// <param name="name">The name of the host function.</param>
        /// <param name="func">The callback for when the host function is invoked.</param>
        /// <returns>Returns a <see cref="Function"/> representing the host function.</returns>
        public Function DefineFunction<T1, T2, T3, T4, T5, T6, T7, T8, T9, T10, T11, T12, T13, T14, T15, T16, TResult>(string moduleName, string name, Func<T1, T2, T3, T4, T5, T6, T7, T8, T9, T10, T11, T12, T13, T14, T15, T16, TResult> func)
        {
            return DefineFunction(moduleName, name, func, true);
        }

        /// <summary>
        /// Defines a new host global variable.
        /// </summary>
        /// <param name="moduleName">The module name of the host variable.</param>
        /// <param name="name">The name of the host variable.</param>
        /// <param name="initialValue">The initial value of the host variable.</param>
        /// <typeparam name="T">The type of the host variable.</typeparam>
        /// <returns>Returns a new <see cref="Global"/> representing the defined global variable.</returns>
        public Global<T> DefineGlobal<T>(string moduleName, string name, T initialValue)
        {
            CheckDisposed();

            if (moduleName is null)
            {
                throw new ArgumentNullException(nameof(moduleName));
            }

            if (name is null)
            {
                throw new ArgumentNullException(nameof(name));
            }

            var global = new Global<T>(Store, initialValue);

            if (!Define(moduleName, name, Interop.wasm_global_as_extern(global.Handle)))
            {
                global.Dispose();
                throw new WasmtimeException($"Failed to define global '{name}' in module '{moduleName}'.");
            }

            return global;
        }

        /// <summary>
        /// Defines a new host mutable global variable.
        /// </summary>
        /// <param name="moduleName">The module name of the host variable.</param>
        /// <param name="name">The name of the host variable.</param>
        /// <param name="initialValue">The initial value of the host variable.</param>
        /// <typeparam name="T">The type of the host variable.</typeparam>
        /// <returns>Returns a new <see cref="MutableGlobal"/> representing the defined mutable global variable.</returns>
        public MutableGlobal<T> DefineMutableGlobal<T>(string moduleName, string name, T initialValue)
        {
            CheckDisposed();

            if (moduleName is null)
            {
                throw new ArgumentNullException(nameof(moduleName));
            }

            if (name is null)
            {
                throw new ArgumentNullException(nameof(name));
            }

            var global = new MutableGlobal<T>(Store, initialValue);

            if (!Define(moduleName, name, Interop.wasm_global_as_extern(global.Handle)))
            {
                global.Dispose();
                throw new WasmtimeException($"Failed to define global '{name}' in module '{moduleName}'.");
            }

            return global;
        }

        /// <summary>
        /// Defines a new host memory.
        /// </summary>
        /// <param name="moduleName">The module name of the host memory.</param>
        /// <param name="name">The name of the host memory.</param>
        /// <param name="minimum">The minimum number of pages for the host memory.</param>
        /// <param name="maximum">The maximum number of pages for the host memory.</param>
        /// <returns>Returns a new <see cref="Memory"/> representing the defined memory.</returns>
        public Memory DefineMemory(string moduleName, string name, uint minimum = 1, uint maximum = uint.MaxValue)
        {
            CheckDisposed();

            if (moduleName is null)
            {
                throw new ArgumentNullException(nameof(moduleName));
            }

            if (name is null)
            {
                throw new ArgumentNullException(nameof(name));
            }

            var memory = new Memory(Store, minimum, maximum);

            if (!Define(moduleName, name, Interop.wasm_memory_as_extern(memory.Handle)))
            {
                memory.Dispose();
                throw new WasmtimeException($"Failed to define memory '{name}' in module '{moduleName}'.");
            }

            return memory;
        }

        /// <summary>
        /// Loads a <see cref="Module"/> given the module name and bytes.
        /// </summary>
        /// <param name="name">The name of the module.</param>
        /// <param name="bytes">The bytes of the module.</param>
        /// <returns>Returns a new <see cref="Module"/>.</returns>
        public Module LoadModule(string name, byte[] bytes)
        {
            CheckDisposed();

            if (string.IsNullOrEmpty(name))
            {
                throw new ArgumentNullException(nameof(name));
            }

            if (bytes is null)
            {
                throw new ArgumentNullException(nameof(bytes));
            }

            return new Module(Store, name, bytes);
        }

        /// <summary>
        /// Loads a <see cref="Module"/> given the path to the WebAssembly file.
        /// </summary>
        /// <param name="path">The path to the WebAssembly file.</param>
        /// <returns>Returns a new <see cref="Module"/>.</returns>
        public Module LoadModule(string path)
        {
            return LoadModule(Path.GetFileNameWithoutExtension(path), File.ReadAllBytes(path));
        }

        /// <summary>
        /// Loads a <see cref="Module"/> based on a WebAssembly text format representation.
        /// </summary>
        /// <param name="name">The name of the module.</param>
        /// <param name="text">The WebAssembly text format representation of the module.</param>
        /// <returns>Returns a new <see cref="Module"/>.</returns>
        public Module LoadModuleText(string name, string text)
        {
            CheckDisposed();

            if (string.IsNullOrEmpty(name))
            {
                throw new ArgumentNullException(nameof(name));
            }

            if (text is null)
            {
                throw new ArgumentNullException(nameof(text));
            }

            var textBytes = Encoding.UTF8.GetBytes(text);
            unsafe
            {
                fixed (byte *ptr = textBytes)
                {
                    Interop.wasm_byte_vec_t textVec;
                    textVec.size = (UIntPtr)textBytes.Length;
                    textVec.data = ptr;

                    if (!Interop.wasmtime_wat2wasm(ref textVec, out var bytes, out var error))
                    {
                        var errorSpan = new ReadOnlySpan<byte>(error.data, checked((int)error.size));
                        var message = Encoding.UTF8.GetString(errorSpan);
                        Interop.wasm_byte_vec_delete(ref error);
                        throw new WasmtimeException($"Failed to parse module text: {message}");
                    }

                    var byteSpan = new ReadOnlySpan<byte>(bytes.data, checked((int)bytes.size));
                    var moduleBytes = byteSpan.ToArray();
                    Interop.wasm_byte_vec_delete(ref bytes);
                    return LoadModule(name, moduleBytes);
                }
            }
        }

        /// <summary>
        /// Loads a <see cref="Module"/> based on the path to a WebAssembly text format file.
        /// </summary>
        /// <param name="path">The path to the WebAssembly text format file.</param>
        /// <returns>Returns a new <see cref="Module"/>.</returns>
        public Module LoadModuleText(string path)
        {
            return LoadModuleText(Path.GetFileNameWithoutExtension(path), File.ReadAllText(path));
        }

        /// <summary>
        /// Instantiates a WebAssembly module.
        /// </summary>
        /// <param name="module">The module to instantiate.</param>
        /// <returns>Returns a new <see cref="Instance" />.</returns>
        public Instance Instantiate(Module module)
        {
            CheckDisposed();

            if (module is null)
            {
                throw new ArgumentNullException(nameof(module));
            }

            return new Instance(Linker, module);
        }

        /// <summary>
        /// Clears all existing definitions in the host.
        /// </summary>
        public void ClearDefinitions()
        {
            CheckDisposed();

            var linker = Interop.wasmtime_linker_new(Store);
            if (linker.IsInvalid)
            {
                throw new WasmtimeException("Failed to create Wasmtime linker.");
            }

            Interop.wasmtime_linker_allow_shadowing(linker, allowShadowing: true);

            Linker.Dispose();
            Linker = linker;
        }

        /// <inheritdoc/>
        public void Dispose()
        {
            if (!Linker.IsInvalid)
            {
                Linker.Dispose();
                Linker.SetHandleAsInvalid();
            }

            if (!Store.IsInvalid)
            {
                Store.Dispose();
                Store.SetHandleAsInvalid();
            }

            if (!Engine.IsInvalid)
            {
                Engine.Dispose();
                Engine.SetHandleAsInvalid();
            }
        }

        internal Host(Interop.WasmConfigHandle config)
        {
            var engine = Interop.wasm_engine_new_with_config(config);
            config.SetHandleAsInvalid();

            Initialize(engine);
        }

        private void Initialize(Interop.EngineHandle engine)
        {
            if (engine.IsInvalid)
            {
                throw new WasmtimeException("Failed to create Wasmtime engine.");
            }

            var store = Interop.wasm_store_new(engine);

            if (store.IsInvalid)
            {
                throw new WasmtimeException("Failed to create Wasmtime store.");
            }

            var linker = Interop.wasmtime_linker_new(store);
            if (linker.IsInvalid)
            {
                throw new WasmtimeException("Failed to create Wasmtime linker.");
            }

            Interop.wasmtime_linker_allow_shadowing(linker, allowShadowing: true);

            Engine = engine;
            Store = store;
            Linker = linker;
        }

        private void CheckDisposed()
        {
            if (Engine.IsInvalid)
            {
                throw new ObjectDisposedException(typeof(Host).FullName);
            }
        }

        private Function DefineFunction(string moduleName, string name, Delegate func, bool hasReturn)
        {
            if (moduleName is null)
            {
                throw new ArgumentNullException(nameof(moduleName));
            }

            if (name is null)
            {
                throw new ArgumentNullException(nameof(name));
            }

            if (func is null)
            {
                throw new ArgumentNullException(nameof(func));
            }

            var function = new Function(Store, func, hasReturn);

            if (!Define(moduleName, name, Interop.wasm_func_as_extern(function.Handle)))
            {
                function.Dispose();
                throw new WasmtimeException($"Failed to define function '{name}' in module '{moduleName}'.");
            }

            _callbacks.Add(function.Callback);
            return function;
        }

        private bool Define(string moduleName, string name, IntPtr ext)
        {
            var moduleNameBytes = Encoding.UTF8.GetBytes(moduleName);
            var nameBytes = Encoding.UTF8.GetBytes(name);

            unsafe
            {
                fixed (byte* moduleNamePtr = moduleNameBytes)
                fixed (byte* namePtr = nameBytes)
                {
                    Interop.wasm_byte_vec_t moduleNameVec = new Interop.wasm_byte_vec_t();
                    moduleNameVec.size = (UIntPtr)moduleNameBytes.Length;
                    moduleNameVec.data = moduleNamePtr;

                    Interop.wasm_byte_vec_t nameVec = new Interop.wasm_byte_vec_t();
                    nameVec.size = (UIntPtr)nameBytes.Length;
                    nameVec.data = namePtr;

                    return Interop.wasmtime_linker_define(Linker, ref moduleNameVec, ref nameVec, ext);
                }
            }
        }

        internal Interop.EngineHandle Engine { get; private set; }
        internal Interop.StoreHandle Store { get; private set; }
        internal Interop.LinkerHandle Linker { get; private set; }

        private List<Delegate> _callbacks = new List<Delegate>();
    }
}
