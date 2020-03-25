using System;
using System.Collections.Generic;
using System.Linq;
using System.Runtime.InteropServices;
using System.Dynamic;
using Wasmtime.Externs;

namespace Wasmtime
{
    /// <summary>
    /// Represents an instantiated WebAssembly module.
    /// </summary>
    public class Instance : DynamicObject, IDisposable
    {
        /// <summary>
        /// The WebAssembly module associated with the instantiation.
        /// </summary>
        public Module Module { get; private set; }

        /// <summary>
        /// The external (instantiated) collection of functions, globals, tables, and memories.
        /// </summary>
        public Wasmtime.Externs.Externs Externs { get; private set; }

        /// <inheritdoc/>
        public unsafe void Dispose()
        {
            if (!Handle.IsInvalid)
            {
                Handle.Dispose();
                Handle.SetHandleAsInvalid();
            }

            if (!(_externs.data is null))
            {
                Interop.wasm_extern_vec_delete(ref _externs);
                _externs.data = null;
            }
        }

        /// <inheritdoc/>
        public override bool TryGetMember(GetMemberBinder binder, out object result)
        {
            if (_globals.TryGetValue(binder.Name, out var global))
            {
                result = global.Value;
                return true;
            }
            result = null;
            return false;
        }

        /// <inheritdoc/>
        public override bool TrySetMember(SetMemberBinder binder, object value)
        {
            if (_globals.TryGetValue(binder.Name, out var global))
            {
                global.Value = value;
                return true;
            }
            return false;
        }

        /// <inheritdoc/>
        public override bool TryInvokeMember(InvokeMemberBinder binder, object[] args, out object result)
        {
            if (!_functions.TryGetValue(binder.Name, out var func))
            {
                result = null;
                return false;
            }

            result = func.Invoke(args);
            return true;
        }

        internal Instance(Interop.LinkerHandle linker, Module module)
        {
            Module = module;

            unsafe
            {
                Handle = Interop.wasmtime_linker_instantiate(linker, module.Handle, out var trap);

                if (trap != IntPtr.Zero)
                {
                    throw TrapException.FromOwnedTrap(trap);
                }
            }

            if (Handle.IsInvalid)
            {
                throw new WasmtimeException("Failed to create Wasmtime instance.");
            }

            Interop.wasm_instance_exports(Handle, out _externs);

            Externs = new Wasmtime.Externs.Externs(Module.Exports, _externs);

            _functions = Externs.Functions.ToDictionary(f => f.Name);
            _globals = Externs.Globals.ToDictionary(g => g.Name);
        }


        internal Interop.InstanceHandle Handle { get; private set; }
        private Interop.wasm_extern_vec_t _externs;
        private Dictionary<string, ExternFunction> _functions;
        private Dictionary<string, ExternGlobal> _globals;
    }
}
