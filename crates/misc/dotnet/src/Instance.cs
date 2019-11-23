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
        internal Instance(Module module, IHost host)
        {
            Host = host;
            Module = module;

            var bindings = host.GetImportBindings(module);
            var handles = bindings.Select(b => b.Bind(module.Store, host)).ToList();

            unsafe
            {
                Handle = Interop.wasm_instance_new(
                    Module.Store.Handle,
                    Module.Handle,
                    handles.Select(h => ToExtern(h)).ToArray(),
                    out var trap);

                if (trap != IntPtr.Zero)
                {
                    throw TrapException.FromOwnedTrap(trap);
                }
            }

            if (Handle.IsInvalid)
            {
                throw new WasmtimeException($"Failed to instantiate module '{module.Name}'.");
            }

            // Dispose of all function handles (not needed at runtime)
            foreach (var h in handles.Where(h => h is Interop.FunctionHandle))
            {
                h.Dispose();
            }

            Interop.wasm_instance_exports(Handle, out _externs);

            Externs = new Wasmtime.Externs.Externs(Module.Exports, _externs);

            _functions = Externs.Functions.ToDictionary(f => f.Name);
            _globals = Externs.Globals.ToDictionary(g => g.Name);
        }

        /// <summary>
        /// The host associated with this instance.
        /// </summary>
        public IHost Host { get; private set; }

        /// <summary>
        /// The WebAssembly module associated with the instantiation.
        /// </summary>
        public Module Module { get; private set; }

        /// <summary>
        /// The external (instantiated) collection of functions, globals, tables, and memories.
        /// </summary>
        public Wasmtime.Externs.Externs Externs { get; private set; }

        /// <inheritdoc/>
        public void Dispose()
        {
            if (!Handle.IsInvalid)
            {
                Handle.Dispose();
                Handle.SetHandleAsInvalid();
            }
            if (_externs.size != UIntPtr.Zero)
            {
                Interop.wasm_extern_vec_delete(ref _externs);
                _externs.size = UIntPtr.Zero;
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

        private static unsafe IntPtr ToExtern(SafeHandle handle)
        {
            switch (handle)
            {
                case Interop.FunctionHandle f:
                    return Interop.wasm_func_as_extern(f);

                case Interop.GlobalHandle g:
                    return Interop.wasm_global_as_extern(g);

                case Interop.MemoryHandle m:
                    return Interop.wasm_memory_as_extern(m);

                default:
                    throw new NotSupportedException("Unexpected handle type.");
            }
        }

        internal Interop.InstanceHandle Handle { get; private set; }
        private Interop.wasm_extern_vec_t _externs;
        private Dictionary<string, ExternFunction> _functions;
        private Dictionary<string, ExternGlobal> _globals;
    }
}
