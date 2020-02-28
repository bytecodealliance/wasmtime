using System;
using System.Diagnostics;

namespace Wasmtime.Imports
{
    /// <summary>
    /// Represents a global variable imported to a WebAssembly module.
    /// </summary>
    public class GlobalImport : Import
    {
        internal GlobalImport(IntPtr importType, IntPtr externType) : base(importType)
        {
            Debug.Assert(Interop.wasm_externtype_kind(externType) == Interop.wasm_externkind_t.WASM_EXTERN_GLOBAL);

            var globalType = Interop.wasm_externtype_as_globaltype_const(externType);
            Kind = Interop.wasm_valtype_kind(Interop.wasm_globaltype_content(globalType));
            IsMutable = Interop.wasm_globaltype_mutability(globalType) == Interop.wasm_mutability_t.WASM_VAR;
        }

        /// <summary>
        /// The kind of value for the global variable.
        /// </summary>
        public ValueKind Kind { get; private set; }

        /// <summary>
        /// Determines whether or not the global variable is mutable.
        /// </summary>
        public bool IsMutable { get; private set; }
    }
}
