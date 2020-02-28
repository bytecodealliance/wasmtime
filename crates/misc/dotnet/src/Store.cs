using System;
using System.IO;

namespace Wasmtime
{
    /// <summary>
    /// Represents the Wasmtime store.
    /// </summary>
    public sealed class Store : IDisposable
    {
        internal Store(Engine engine)
        {
            Handle = Interop.wasm_store_new(engine.Handle);

            if (Handle.IsInvalid)
            {
                throw new WasmtimeException("Failed to create Wasmtime store.");
            }
        }

        /// <summary>
        /// Create a <see cref="Module"/> given the module name and bytes.
        /// </summary>
        /// <param name="name">The name of the module.</param>
        /// <param name="bytes">The bytes of the module.</param>
        /// <returns>Retuw <see cref="Module"/>.</returns>
        public Module CreateModule(string name, byte[] bytes)
        {
            if (string.IsNullOrEmpty(name))
            {
                throw new ArgumentNullException(nameof(name));
            }

            if (bytes is null)
            {
                throw new ArgumentNullException(nameof(bytes));
            }

            return new Module(this, name, bytes);
        }

        /// <summary>
        /// Create a <see cref="Module"/> given the module name and path to the WebAssembly file.
        /// </summary>
        /// <param name="name">The name of the module.</param>
        /// <param name="path">The path to the WebAssembly file.</param>
        /// <returns>Returns a new <see cref="Module"/>.</returns>
        public Module CreateModule(string name, string path)
        {
            return CreateModule(name, File.ReadAllBytes(path));
        }

        /// <summary>
        /// Create a <see cref="Module"/> given the path to the WebAssembly file.
        /// </summary>
        /// <param name="path">The path to the WebAssembly file.</param>
        /// <returns>Returns a new <see cref="Module"/>.</returns>
        public Module CreateModule(string path)
        {
            return CreateModule(Path.GetFileNameWithoutExtension(path), path);
        }

        /// <inheritdoc/>
        public void Dispose()
        {
            if (!Handle.IsInvalid)
            {
                Handle.Dispose();
                Handle.SetHandleAsInvalid();
            }
        }

        internal Interop.StoreHandle Handle { get; private set; }
    }
}
