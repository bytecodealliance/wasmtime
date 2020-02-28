using System;
using System.Collections.Generic;
using System.Diagnostics;

namespace Wasmtime.Imports
{
    /// <summary>
    /// Represents a function imported to a WebAssembly module.
    /// </summary>
    public class FunctionImport : Import
    {
        internal FunctionImport(IntPtr importType, IntPtr externType) : base(importType)
        {
            Debug.Assert(Interop.wasm_externtype_kind(externType) == Interop.wasm_externkind_t.WASM_EXTERN_FUNC);

            unsafe
            {
                var funcType = Interop.wasm_externtype_as_functype_const(externType);
                Parameters = Interop.ToValueKindList(Interop.wasm_functype_params(funcType));
                Results = Interop.ToValueKindList(Interop.wasm_functype_results(funcType));
            }
        }

        /// <summary>
        /// The parameters of the imported function.
        /// </summary>
        public IReadOnlyList<ValueKind> Parameters { get; private set; }

        /// <summary>
        /// The results of the imported function.
        /// </summary>
        public IReadOnlyList<ValueKind> Results { get; private set; }
    }
}
