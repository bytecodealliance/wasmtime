using System;
using System.Collections.Generic;

namespace Wasmtime.Exports
{
    /// <summary>
    /// Represents the exports of a WebAssembly module.
    /// </summary>
    public class Exports
    {
        internal Exports(Module module)
        {
            Interop.wasm_exporttype_vec_t exports;
            Interop.wasm_module_exports(module.Handle, out exports);

            try
            {
                var all = new List<Export>((int)exports.size);
                var functions = new List<FunctionExport>();
                var globals = new List<GlobalExport>();
                var tables = new List<TableExport>();
                var memories = new List<MemoryExport>();

                for (int i = 0; i < (int)exports.size; ++i)
                {
                    unsafe
                    {
                        var exportType = exports.data[i];
                        var externType = Interop.wasm_exporttype_type(exportType);

                        switch (Interop.wasm_externtype_kind(externType))
                        {
                            case Interop.wasm_externkind_t.WASM_EXTERN_FUNC:
                                var function = new FunctionExport(exportType, externType);
                                functions.Add(function);
                                all.Add(function);
                                break;

                            case Interop.wasm_externkind_t.WASM_EXTERN_GLOBAL:
                                var global = new GlobalExport(exportType, externType);
                                globals.Add(global);
                                all.Add(global);
                                break;

                            case Interop.wasm_externkind_t.WASM_EXTERN_TABLE:
                                var table = new TableExport(exportType, externType);
                                tables.Add(table);
                                all.Add(table);
                                break;

                            case Interop.wasm_externkind_t.WASM_EXTERN_MEMORY:
                                var memory = new MemoryExport(exportType, externType);
                                memories.Add(memory);
                                all.Add(memory);
                                break;

                            default:
                                throw new NotSupportedException("Unsupported export extern type.");
                        }
                    }
                }

                Functions = functions;
                Globals = globals;
                Tables = tables;
                Memories = memories;
                All = all;
            }
            finally
            {
                Interop.wasm_exporttype_vec_delete(ref exports);
            }
        }

        /// <summary>
        /// The exported functions of a WebAssembly module.
        /// </summary>
        public IReadOnlyList<FunctionExport> Functions { get; private set; }

        /// <summary>
        /// The exported globals of a WebAssembly module.
        /// </summary>
        public IReadOnlyList<GlobalExport> Globals { get; private set; }

        /// <summary>
        /// The exported tables of a WebAssembly module.
        /// </summary>
        public IReadOnlyList<TableExport> Tables { get; private set; }

        /// <summary>
        /// The exported memories of a WebAssembly module.
        /// </summary>
        public IReadOnlyList<MemoryExport> Memories { get; private set; }

        internal List<Export> All { get; private set; }
    }
}
