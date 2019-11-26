using System;
using System.Collections.Generic;
using System.Diagnostics;

namespace Wasmtime.Exports
{
    /// <summary>
    /// Represents a function exported from a WebAssembly module.
    /// </summary>
    public class FunctionExport : Export
    {
        internal FunctionExport(IntPtr exportType, IntPtr externType) : base(exportType)
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
        /// The parameter of the exported WebAssembly function.
        /// </summary>
        public IReadOnlyList<ValueKind> Parameters { get; private set; }

        /// <summary>
        /// The results of the exported WebAssembly function.
        /// </summary>
        public IReadOnlyList<ValueKind> Results { get; private set; }
    }
}
