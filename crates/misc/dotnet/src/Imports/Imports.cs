using System;
using System.Collections.Generic;

namespace Wasmtime.Imports
{
    /// <summary>
    /// Represents imported functions, globals, tables, and memories to a WebAssembly module.
    /// </summary>
    public class Imports : IDisposable
    {
        internal Imports(Module module)
        {
            Interop.wasm_importtype_vec_t imports;
            Interop.wasm_module_imports(module.Handle, out imports);

            var all = new List<Import>((int)imports.size);
            var functions = new List<FunctionImport>();
            var globals = new List<GlobalImport>();
            var tables = new List<TableImport>();
            var memories = new List<MemoryImport>();

            for (int i = 0; i < (int)imports.size; ++i)
            {
                unsafe
                {
                    var importType = imports.data[i];
                    var externType = Interop.wasm_importtype_type(importType);

                    switch (Interop.wasm_externtype_kind(externType))
                    {
                        case Interop.wasm_externkind_t.WASM_EXTERN_FUNC:
                            var function = new FunctionImport(importType, externType);
                            functions.Add(function);
                            all.Add(function);
                            break;

                        case Interop.wasm_externkind_t.WASM_EXTERN_GLOBAL:
                            var global = new GlobalImport(importType, externType);
                            globals.Add(global);
                            all.Add(global);
                            break;

                        case Interop.wasm_externkind_t.WASM_EXTERN_TABLE:
                            var table = new TableImport(importType, externType);
                            tables.Add(table);
                            all.Add(table);
                            break;

                        case Interop.wasm_externkind_t.WASM_EXTERN_MEMORY:
                            var memory = new MemoryImport(importType, externType);
                            memories.Add(memory);
                            all.Add(memory);
                            break;

                        default:
                            throw new NotSupportedException("Unsupported import extern type.");
                    }
                }
            }

            Functions = functions;
            Globals = globals;
            Tables = tables;
            Memories = memories;
            All = all;
        }

        /// <inheritdoc/>
        public unsafe void Dispose()
        {
            if (!(_imports.data is null))
            {
                Interop.wasm_importtype_vec_delete(ref _imports);
                _imports.data = null;
            }
        }

        /// <summary>
        /// The imported functions required by a WebAssembly module.
        /// </summary>
        public IReadOnlyList<FunctionImport> Functions { get; private set; }

        /// <summary>
        /// The imported globals required by a WebAssembly module.
        /// </summary>
        public IReadOnlyList<GlobalImport> Globals { get; private set; }

        /// <summary>
        /// The imported tables required by a WebAssembly module.
        /// </summary>
        public IReadOnlyList<TableImport> Tables { get; private set; }

        /// <summary>
        /// The imported memories required by a WebAssembly module.
        /// </summary>
        public IReadOnlyList<MemoryImport> Memories { get; private set; }

        internal IReadOnlyList<Import> All { get; private set; }

        private Interop.wasm_importtype_vec_t _imports;
    }
}
