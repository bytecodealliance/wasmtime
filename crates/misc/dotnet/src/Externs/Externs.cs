using System;
using System.Collections.Generic;
using Wasmtime.Exports;

namespace Wasmtime.Externs
{
    /// <summary>
    /// Represents external (instantiated) WebAssembly functions, globals, tables, and memories.
    /// </summary>
    public class Externs
    {
        internal Externs(Wasmtime.Exports.Exports exports, Interop.wasm_extern_vec_t externs)
        {
            var functions = new List<ExternFunction>();
            var globals = new List<ExternGlobal>();
            var memories = new List<ExternMemory>();

            for (int i = 0; i < (int)externs.size; ++i)
            {
                unsafe
                {
                    var ext = externs.data[i];

                    switch (Interop.wasm_extern_kind(ext))
                    {
                        case Interop.wasm_externkind_t.WASM_EXTERN_FUNC:
                            var function = new ExternFunction((FunctionExport)exports.All[i], Interop.wasm_extern_as_func(ext));
                            functions.Add(function);
                            break;

                        case Interop.wasm_externkind_t.WASM_EXTERN_GLOBAL:
                            var global = new ExternGlobal((GlobalExport)exports.All[i], Interop.wasm_extern_as_global(ext));
                            globals.Add(global);
                            break;

                        case Interop.wasm_externkind_t.WASM_EXTERN_MEMORY:
                            var memory = new ExternMemory((MemoryExport)exports.All[i], Interop.wasm_extern_as_memory(ext));
                            memories.Add(memory);
                            break;

                        default:
                            throw new NotSupportedException("Unsupported extern type.");
                    }
                }
            }

            Functions = functions;
            Globals = globals;
            Memories = memories;
        }

        /// <summary>
        /// The extern functions from an instantiated WebAssembly module.
        /// </summary>
        public IReadOnlyList<ExternFunction> Functions { get; private set; }

        /// <summary>
        /// The extern globals from an instantiated WebAssembly module.
        /// </summary>
        public IReadOnlyList<ExternGlobal> Globals { get; private set; }

        /// <summary>
        /// The extern memories from an instantiated WebAssembly module.
        /// </summary>
        public IReadOnlyList<ExternMemory> Memories { get; private set; }
    }
}
