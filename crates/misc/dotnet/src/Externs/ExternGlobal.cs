using System;
using Wasmtime.Exports;

namespace Wasmtime.Externs
{
    /// <summary>
    /// Represents an external (instantiated) WebAssembly global.
    /// </summary>
    public class ExternGlobal
    {
        internal ExternGlobal(GlobalExport export, IntPtr global)
        {
            _export = export;
            _global = global;
        }

        /// <summary>
        /// The name of the WebAssembly global.
        /// </summary>
        public string Name => _export.Name;

        /// <summary>
        /// The kind of value for the global variable.
        /// </summary>
        public ValueKind Kind => _export.Kind;

        /// <summary>
        /// Determines whether or not the global variable is mutable.
        /// </summary>
        public bool IsMutable => _export.IsMutable;

        public object Value
        {
            get
            {
                unsafe
                {
                    var v = stackalloc Interop.wasm_val_t[1];
                    Interop.wasm_global_get(_global, v);
                    return Interop.ToObject(v);
                }
            }
            set
            {
                if (!IsMutable)
                {
                    throw new InvalidOperationException($"The value of global '{Name}' cannot be modified.");
                }

                var v = Interop.ToValue(value, Kind);

                unsafe
                {
                    Interop.wasm_global_set(_global, &v);
                }
            }
        }

        private GlobalExport _export;
        private IntPtr _global;
    }
}
