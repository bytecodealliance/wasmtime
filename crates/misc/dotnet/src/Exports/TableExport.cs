using System;
using System.Diagnostics;

namespace Wasmtime.Exports
{
    /// <summary>
    /// Represents a table exported from a WebAssembly module.
    /// </summary>
    public class TableExport : Export
    {
        internal TableExport(IntPtr exportType, IntPtr externType) : base(exportType)
        {
            Debug.Assert(Interop.wasm_externtype_kind(externType) == Interop.wasm_externkind_t.WASM_EXTERN_TABLE);

            var tableType = Interop.wasm_externtype_as_tabletype_const(externType);

            Kind = Interop.wasm_valtype_kind(Interop.wasm_tabletype_element(tableType));

            unsafe
            {
                var limits = Interop.wasm_tabletype_limits(tableType);
                Minimum = limits->min;
                Maximum = limits->max;
            }
        }

        /// <summary>
        /// The value kind of the table.
        /// </summary>
        public ValueKind Kind { get; private set; }

        /// <summary>
        /// The minimum number of elements in the table.
        /// </summary>
        public uint Minimum { get; private set; }

        /// <summary>
        /// The maximum number of elements in the table.
        /// </summary>
        public uint Maximum { get; private set; }
    }
}
