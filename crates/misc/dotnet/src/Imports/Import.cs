using System;
using System.Runtime.InteropServices;

namespace Wasmtime.Imports
{
    /// <summary>
    /// The base class for import types.
    /// </summary>
    public abstract class Import
    {
        internal Import(IntPtr importType)
        {
            unsafe
            {
                var moduleName = Interop.wasm_importtype_module(importType);
                ModuleName = Marshal.PtrToStringUTF8((IntPtr)moduleName->data, (int)moduleName->size);

                var name = Interop.wasm_importtype_name(importType);
                Name = Marshal.PtrToStringUTF8((IntPtr)name->data, (int)name->size);
            }
        }

        /// <summary>
        /// The module name of the import.
        /// </summary>
        public string ModuleName { get; private set; }

        /// <summary>
        /// The name of the import.
        /// </summary>
        public string Name { get; private set; }

        /// <inheritdoc/>
        public override string ToString()
        {
            return $"{ModuleName}{(string.IsNullOrEmpty(ModuleName) ? "" : ".")}{Name}";
        }
    }
}
