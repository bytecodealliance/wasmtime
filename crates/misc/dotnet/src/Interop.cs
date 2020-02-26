using System;
using System.Collections.Generic;
using System.Runtime.InteropServices;
using System.Text;

namespace Wasmtime
{
    /// <summary>
    /// Implements the Wasmtime API bindings.
    /// </summary>
    /// <remarks>See https://github.com/WebAssembly/wasm-c-api/blob/master/include/wasm.h for the C API reference.</remarks>
    internal static class Interop
    {
        const string LibraryName = "wasmtime";

        internal class EngineHandle : SafeHandle
        {
            public EngineHandle() : base(IntPtr.Zero, true)
            {
            }

            public override bool IsInvalid => handle == IntPtr.Zero;

            protected override bool ReleaseHandle()
            {
                Interop.wasm_engine_delete(handle);
                return true;
            }
        }

        internal class StoreHandle : SafeHandle
        {
            public StoreHandle() : base(IntPtr.Zero, true)
            {
            }

            public override bool IsInvalid => handle == IntPtr.Zero;

            protected override bool ReleaseHandle()
            {
                Interop.wasm_store_delete(handle);
                return true;
            }
        }

        internal class ModuleHandle : SafeHandle
        {
            public ModuleHandle() : base(IntPtr.Zero, true)
            {
            }

            public override bool IsInvalid => handle == IntPtr.Zero;

            protected override bool ReleaseHandle()
            {
                Interop.wasm_module_delete(handle);
                return true;
            }
        }

        internal class FunctionHandle : SafeHandle
        {
            public FunctionHandle() : base(IntPtr.Zero, true)
            {
            }

            public WasmFuncCallback Callback { get; set; } = null;

            public override bool IsInvalid => handle == IntPtr.Zero;

            protected override bool ReleaseHandle()
            {
                Interop.wasm_func_delete(handle);
                return true;
            }
        }

        internal class GlobalHandle : SafeHandle
        {
            public GlobalHandle() : base(IntPtr.Zero, true)
            {
            }

            public override bool IsInvalid => handle == IntPtr.Zero;

            protected override bool ReleaseHandle()
            {
                Interop.wasm_global_delete(handle);
                return true;
            }
        }

        internal class MemoryHandle : SafeHandle
        {
            public MemoryHandle() : base(IntPtr.Zero, true)
            {
            }

            public override bool IsInvalid => handle == IntPtr.Zero;

            protected override bool ReleaseHandle()
            {
                Interop.wasm_memory_delete(handle);
                return true;
            }
        }

        internal class InstanceHandle : SafeHandle
        {
            public InstanceHandle() : base(IntPtr.Zero, true)
            {
            }

            public override bool IsInvalid => handle == IntPtr.Zero;

            protected override bool ReleaseHandle()
            {
                Interop.wasm_instance_delete(handle);
                return true;
            }
        }

        internal class FuncTypeHandle : SafeHandle
        {
            public FuncTypeHandle() : base(IntPtr.Zero, true)
            {
            }

            public override bool IsInvalid => handle == IntPtr.Zero;

            protected override bool ReleaseHandle()
            {
                Interop.wasm_functype_delete(handle);
                return true;
            }
        }

        internal class GlobalTypeHandle : SafeHandle
        {
            public GlobalTypeHandle() : base(IntPtr.Zero, true)
            {
            }

            public override bool IsInvalid => handle == IntPtr.Zero;

            protected override bool ReleaseHandle()
            {
                Interop.wasm_globaltype_delete(handle);
                return true;
            }
        }

        internal class MemoryTypeHandle : SafeHandle
        {
            public MemoryTypeHandle() : base(IntPtr.Zero, true)
            {
            }

            public override bool IsInvalid => handle == IntPtr.Zero;

            protected override bool ReleaseHandle()
            {
                Interop.wasm_memorytype_delete(handle);
                return true;
            }
        }

        internal class ValueTypeHandle : SafeHandle
        {
            public ValueTypeHandle() : base(IntPtr.Zero, true)
            {
            }

            public override bool IsInvalid => handle == IntPtr.Zero;

            protected override bool ReleaseHandle()
            {
                Interop.wasm_valtype_delete(handle);
                return true;
            }
        }

        internal class WasiConfigHandle : SafeHandle
        {
            public WasiConfigHandle() : base(IntPtr.Zero, true)
            {
            }

            public override bool IsInvalid => handle == IntPtr.Zero;

            protected override bool ReleaseHandle()
            {
                Interop.wasi_config_delete(handle);
                return true;
            }
        }

        internal class WasiInstanceHandle : SafeHandle
        {
            public WasiInstanceHandle() : base(IntPtr.Zero, true)
            {
            }

            public override bool IsInvalid => handle == IntPtr.Zero;

            protected override bool ReleaseHandle()
            {
                Interop.wasi_instance_delete(handle);
                return true;
            }
        }

        internal class WasiExportHandle : SafeHandle
        {
            public WasiExportHandle(IntPtr handle) : base(IntPtr.Zero, false /* not owned */)
            {
                SetHandle(handle);
            }

            public override bool IsInvalid => handle == IntPtr.Zero;

            protected override bool ReleaseHandle()
            {
                return true;
            }
        }

        [StructLayout(LayoutKind.Sequential)]
        internal unsafe struct wasm_byte_vec_t
        {
            public UIntPtr size;
            public byte* data;
        }

        [StructLayout(LayoutKind.Sequential)]
        internal unsafe struct wasm_valtype_vec_t
        {
            public UIntPtr size;
            public IntPtr* data;
        }

        [StructLayout(LayoutKind.Sequential)]
        internal unsafe struct wasm_export_vec_t
        {
            public UIntPtr size;
            public IntPtr* data;
        }

        [StructLayout(LayoutKind.Sequential)]
        internal unsafe struct wasm_extern_vec_t
        {
            public UIntPtr size;
            public IntPtr* data;
        }

        [StructLayout(LayoutKind.Sequential)]
        internal unsafe struct wasm_importtype_vec_t
        {
            public UIntPtr size;
            public IntPtr* data;
        }

        [StructLayout(LayoutKind.Sequential)]
        internal unsafe struct wasm_exporttype_vec_t
        {
            public UIntPtr size;
            public IntPtr* data;
        }

        internal enum wasm_valkind_t : byte
        {
            WASM_I32,
            WASM_I64,
            WASM_F32,
            WASM_F64,
            WASM_ANYREF = 128,
            WASM_FUNCREF,
        }

        [StructLayout(LayoutKind.Explicit)]
        internal struct wasm_val_union_t
        {
            [FieldOffset(0)]
            public int i32;

            [FieldOffset(0)]
            public long i64;

            [FieldOffset(0)]
            public float f32;

            [FieldOffset(0)]
            public double f64;

            [FieldOffset(0)]
            public IntPtr reference;
        }

        [StructLayout(LayoutKind.Sequential)]
        internal struct wasm_val_t
        {
            public wasm_valkind_t kind;
            public wasm_val_union_t of;
        }

        [StructLayout(LayoutKind.Sequential)]
        internal struct wasm_limits_t
        {
            public uint min;

            public uint max;
        }

        public static wasm_val_t ToValue(object o, ValueKind kind)
        {
            wasm_val_t value = new wasm_val_t();
            switch (kind)
            {
                case ValueKind.Int32:
                    value.kind = wasm_valkind_t.WASM_I32;
                    value.of.i32 = (int)Convert.ChangeType(o, TypeCode.Int32);
                    break;

                case ValueKind.Int64:
                    value.kind = wasm_valkind_t.WASM_I64;
                    value.of.i64 = (long)Convert.ChangeType(o, TypeCode.Int64);
                    break;

                case ValueKind.Float32:
                    value.kind = wasm_valkind_t.WASM_F32;
                    value.of.f32 = (float)Convert.ChangeType(o, TypeCode.Single);
                    break;

                case ValueKind.Float64:
                    value.kind = wasm_valkind_t.WASM_F64;
                    value.of.f64 = (double)Convert.ChangeType(o, TypeCode.Double);
                    break;

                // TODO: support AnyRef

                default:
                    throw new NotSupportedException("Unsupported value type.");
            }
            return value;
        }

        public static unsafe object ToObject(wasm_val_t* v)
        {
            switch (v->kind)
            {
                case Interop.wasm_valkind_t.WASM_I32:
                    return v->of.i32;

                case Interop.wasm_valkind_t.WASM_I64:
                    return v->of.i64;

                case Interop.wasm_valkind_t.WASM_F32:
                    return v->of.f32;

                case Interop.wasm_valkind_t.WASM_F64:
                    return v->of.f64;

                // TODO: support AnyRef

                default:
                    throw new NotSupportedException("Unsupported value kind.");
            }
        }

        public static bool TryGetValueKind(Type type, out ValueKind kind)
        {
            if (type == typeof(int))
            {
                kind = ValueKind.Int32;
                return true;
            }

            if (type == typeof(long))
            {
                kind = ValueKind.Int64;
                return true;
            }

            if (type == typeof(float))
            {
                kind = ValueKind.Float32;
                return true;
            }

            if (type == typeof(double))
            {
                kind = ValueKind.Float64;
                return true;
            }

            kind = default(ValueKind);
            return false;
        }

        public static ValueKind ToValueKind(Type type)
        {
            if (TryGetValueKind(type, out var kind))
            {
                return kind;
            }

            throw new NotSupportedException($"Type '{type}' is not a supported WebAssembly value type.");
        }

        public static string ToString(ValueKind kind)
        {
            switch (kind)
            {
                case ValueKind.Int32:
                    return "int";

                case ValueKind.Int64:
                    return "long";

                case ValueKind.Float32:
                    return "float";

                case ValueKind.Float64:
                    return "double";

                default:
                    throw new NotSupportedException("Unsupported value kind.");
            }
        }

        public static bool IsMatchingKind(ValueKind kind, ValueKind expected)
        {
            if (kind == expected)
            {
                return true;
            }

            if (expected == ValueKind.AnyRef)
            {
                return kind == ValueKind.FuncRef;
            }

            return false;
        }

        internal unsafe delegate IntPtr WasmFuncCallback(wasm_val_t* parameters, wasm_val_t* results);

        internal enum wasm_externkind_t : byte
        {
            WASM_EXTERN_FUNC,
            WASM_EXTERN_GLOBAL,
            WASM_EXTERN_TABLE,
            WASM_EXTERN_MEMORY,
        }

        internal enum wasm_mutability_t : byte
        {
            WASM_CONST,
            WASM_VAR,
        }

        internal static unsafe List<ValueKind> ToValueKindList(Interop.wasm_valtype_vec_t* vec)
        {
            var list = new List<ValueKind>((int)vec->size);

            for (int i = 0; i < (int)vec->size; ++i)
            {
                list.Add(Interop.wasm_valtype_kind(vec->data[i]));
            }

            return list;
        }

        internal static Interop.wasm_valtype_vec_t ToValueTypeVec(IReadOnlyList<ValueKind> collection)
        {
            Interop.wasm_valtype_vec_t vec;
            Interop.wasm_valtype_vec_new_uninitialized(out vec, (UIntPtr)collection.Count);

            int i = 0;
            foreach (var type in collection)
            {
                var valType = Interop.wasm_valtype_new((wasm_valkind_t)type);
                unsafe
                {
                    vec.data[i++] = valType.DangerousGetHandle();
                }
                valType.SetHandleAsInvalid();
            }

            return vec;
        }

        internal static unsafe (byte*[], GCHandle[]) ToUTF8PtrArray(IList<string> strings)
        {
            // Unfortunately .NET cannot currently marshal string[] as UTF-8
            // See: https://github.com/dotnet/runtime/issues/7315
            // Therefore, we need to marshal the strings manually
            var handles = new GCHandle[strings.Count];
            var ptrs = new byte*[strings.Count];
            for (int i = 0; i < strings.Count; ++i)
            {
                handles[i] = GCHandle.Alloc(
                    Encoding.UTF8.GetBytes(strings[i] + '\0'),
                    GCHandleType.Pinned
                );
                ptrs[i] = (byte*)handles[i].AddrOfPinnedObject();
            }

            return (ptrs, handles);
        }

        // Engine imports

        [DllImport(LibraryName)]
        public static extern EngineHandle wasm_engine_new();

        [DllImport(LibraryName)]
        public static extern void wasm_engine_delete(IntPtr engine);

        // Store imports

        [DllImport(LibraryName)]
        public static extern StoreHandle wasm_store_new(EngineHandle engine);

        [DllImport(LibraryName)]
        public static extern void wasm_store_delete(IntPtr engine);

        // Byte vec imports

        [DllImport(LibraryName)]
        public static extern void wasm_byte_vec_new_empty(out wasm_byte_vec_t vec);

        [DllImport(LibraryName)]
        public static extern void wasm_byte_vec_new_uninitialized(out wasm_byte_vec_t vec, UIntPtr length);

        [DllImport(LibraryName)]
        public static extern void wasm_byte_vec_new(out wasm_byte_vec_t vec, UIntPtr length, byte[] data);

        [DllImport(LibraryName)]
        public static extern void wasm_byte_vec_copy(out wasm_byte_vec_t vec, ref wasm_byte_vec_t src);

        [DllImport(LibraryName)]
        public static extern void wasm_byte_vec_delete(ref wasm_byte_vec_t vec);

        // Value type vec imports

        [DllImport(LibraryName)]
        public static extern void wasm_valtype_vec_new_empty(out wasm_valtype_vec_t vec);

        [DllImport(LibraryName)]
        public static extern void wasm_valtype_vec_new_uninitialized(out wasm_valtype_vec_t vec, UIntPtr length);

        [DllImport(LibraryName)]
        public static extern void wasm_valtype_vec_new(out wasm_valtype_vec_t vec, UIntPtr length, IntPtr[] data);

        [DllImport(LibraryName)]
        public static extern void wasm_valtype_vec_copy(out wasm_valtype_vec_t vec, ref wasm_valtype_vec_t src);

        [DllImport(LibraryName)]
        public static extern void wasm_valtype_vec_delete(ref wasm_valtype_vec_t vec);

        // Extern vec imports

        [DllImport(LibraryName)]
        public static extern void wasm_extern_vec_new_empty(out wasm_extern_vec_t vec);

        [DllImport(LibraryName)]
        public static extern void wasm_extern_vec_new_uninitialized(out wasm_extern_vec_t vec, UIntPtr length);

        [DllImport(LibraryName)]
        public static extern void wasm_extern_vec_new(out wasm_extern_vec_t vec, UIntPtr length, IntPtr[] data);

        [DllImport(LibraryName)]
        public static extern void wasm_extern_vec_copy(out wasm_extern_vec_t vec, ref wasm_extern_vec_t src);

        [DllImport(LibraryName)]
        public static extern void wasm_extern_vec_delete(ref wasm_extern_vec_t vec);

        // Import type vec imports

        [DllImport(LibraryName)]
        public static extern void wasm_importtype_vec_new_empty(out wasm_importtype_vec_t vec);

        [DllImport(LibraryName)]
        public static extern void wasm_importtype_vec_new_uninitialized(out wasm_importtype_vec_t vec, UIntPtr length);

        [DllImport(LibraryName)]
        public static extern void wasm_importtype_vec_new(out wasm_importtype_vec_t vec, UIntPtr length, IntPtr[] data);

        [DllImport(LibraryName)]
        public static extern void wasm_importtype_vec_copy(out wasm_importtype_vec_t vec, ref wasm_importtype_vec_t src);

        [DllImport(LibraryName)]
        public static extern void wasm_importtype_vec_delete(ref wasm_importtype_vec_t vec);

        // Export type vec imports

        [DllImport(LibraryName)]
        public static extern void wasm_exporttype_vec_new_empty(out wasm_exporttype_vec_t vec);

        [DllImport(LibraryName)]
        public static extern void wasm_exporttype_vec_new_uninitialized(out wasm_exporttype_vec_t vec, UIntPtr length);

        [DllImport(LibraryName)]
        public static extern void wasm_exporttype_vec_new(out wasm_exporttype_vec_t vec, UIntPtr length, IntPtr[] data);

        [DllImport(LibraryName)]
        public static extern void wasm_exporttype_vec_copy(out wasm_exporttype_vec_t vec, ref wasm_exporttype_vec_t src);

        [DllImport(LibraryName)]
        public static extern void wasm_exporttype_vec_delete(ref wasm_exporttype_vec_t vec);

        // Import type imports

        [DllImport(LibraryName)]
        public static extern unsafe wasm_byte_vec_t* wasm_importtype_module(IntPtr importType);

        [DllImport(LibraryName)]
        public static extern unsafe wasm_byte_vec_t* wasm_importtype_name(IntPtr importType);

        [DllImport(LibraryName)]
        public static extern unsafe IntPtr wasm_importtype_type(IntPtr importType);

        // Export type imports

        [DllImport(LibraryName)]
        public static extern unsafe wasm_byte_vec_t* wasm_exporttype_name(IntPtr exportType);

        [DllImport(LibraryName)]
        public static extern unsafe IntPtr wasm_exporttype_type(IntPtr exportType);

        // Module imports

        [DllImport(LibraryName)]
        public static extern ModuleHandle wasm_module_new(StoreHandle store, ref wasm_byte_vec_t bytes);

        [DllImport(LibraryName)]
        public static extern void wasm_module_imports(ModuleHandle module, out wasm_importtype_vec_t imports);

        [DllImport(LibraryName)]
        public static extern void wasm_module_exports(ModuleHandle module, out wasm_exporttype_vec_t exports);

        [DllImport(LibraryName)]
        public static extern void wasm_module_delete(IntPtr module);

        // Value type imports

        [DllImport(LibraryName)]
        public static extern ValueTypeHandle wasm_valtype_new(wasm_valkind_t kind);

        [DllImport(LibraryName)]
        public static extern void wasm_valtype_delete(IntPtr valueType);

        [DllImport(LibraryName)]
        public static extern ValueKind wasm_valtype_kind(IntPtr valueType);

        // Extern imports

        [DllImport(LibraryName)]
        public static extern wasm_externkind_t wasm_extern_kind(IntPtr ext);

        [DllImport(LibraryName)]
        public static extern IntPtr wasm_extern_type(IntPtr ext);

        [DllImport(LibraryName)]
        public static extern IntPtr wasm_extern_as_func(IntPtr ext);

        [DllImport(LibraryName)]
        public static extern IntPtr wasm_extern_as_global(IntPtr ext);

        [DllImport(LibraryName)]
        public static extern IntPtr wasm_extern_as_table(IntPtr ext);

        [DllImport(LibraryName)]
        public static extern IntPtr wasm_extern_as_memory(IntPtr ext);

        // Extern type imports

        [DllImport(LibraryName)]
        public static extern wasm_externkind_t wasm_externtype_kind(IntPtr externType);

        [DllImport(LibraryName)]
        public static extern IntPtr wasm_externtype_as_functype_const(IntPtr externType);

        [DllImport(LibraryName)]
        public static extern IntPtr wasm_externtype_as_globaltype_const(IntPtr externType);

        [DllImport(LibraryName)]
        public static extern IntPtr wasm_externtype_as_tabletype_const(IntPtr externType);

        [DllImport(LibraryName)]
        public static extern IntPtr wasm_externtype_as_memorytype_const(IntPtr externType);

        // Function imports

        [DllImport(LibraryName)]
        public static extern FunctionHandle wasm_func_new(StoreHandle store, FuncTypeHandle type, WasmFuncCallback callback);

        [DllImport(LibraryName)]
        public static extern void wasm_func_delete(IntPtr function);

        [DllImport(LibraryName)]
        public static unsafe extern IntPtr wasm_func_call(IntPtr function, wasm_val_t* args, wasm_val_t* results);

        [DllImport(LibraryName)]
        public static extern IntPtr wasm_func_as_extern(FunctionHandle function);

        [DllImport(LibraryName)]
        public static extern IntPtr wasm_global_as_extern(GlobalHandle global);

        [DllImport(LibraryName)]
        public static extern IntPtr wasm_memory_as_extern(MemoryHandle memory);

        // Function type imports

        [DllImport(LibraryName)]
        public static extern unsafe wasm_valtype_vec_t* wasm_functype_params(IntPtr funcType);

        [DllImport(LibraryName)]
        public static extern unsafe wasm_valtype_vec_t* wasm_functype_results(IntPtr funcType);

        // Instance imports

        [DllImport(LibraryName)]
        public static extern unsafe InstanceHandle wasm_instance_new(StoreHandle store, ModuleHandle module, IntPtr[] imports, out IntPtr trap);

        [DllImport(LibraryName)]
        public static extern void wasm_instance_delete(IntPtr ext);

        [DllImport(LibraryName)]
        public static extern void wasm_instance_exports(InstanceHandle instance, out wasm_extern_vec_t exports);

        // Function type imports

        [DllImport(LibraryName)]
        public static extern FuncTypeHandle wasm_functype_new(ref wasm_valtype_vec_t parameters, ref wasm_valtype_vec_t results);

        [DllImport(LibraryName)]
        public static extern void wasm_functype_delete(IntPtr functype);

        // Global type imports

        [DllImport(LibraryName)]
        public static extern GlobalTypeHandle wasm_globaltype_new(IntPtr valueType, wasm_mutability_t mutability);

        [DllImport(LibraryName)]
        public static extern IntPtr wasm_globaltype_delete(IntPtr globalType);

        [DllImport(LibraryName)]
        public static extern IntPtr wasm_globaltype_content(IntPtr globalType);

        [DllImport(LibraryName)]
        public static extern wasm_mutability_t wasm_globaltype_mutability(IntPtr globalType);

        // Memory type imports

        [DllImport(LibraryName)]
        public static extern unsafe MemoryTypeHandle wasm_memorytype_new(wasm_limits_t* limits);

        [DllImport(LibraryName)]
        public static extern IntPtr wasm_memorytype_delete(IntPtr memoryType);


        [DllImport(LibraryName)]
        public static extern unsafe wasm_limits_t* wasm_memorytype_limits(MemoryTypeHandle memoryType);

        // Trap imports

        [DllImport(LibraryName)]
        public static extern IntPtr wasm_trap_new(StoreHandle store, ref wasm_byte_vec_t message);

        [DllImport(LibraryName)]
        public static extern void wasm_trap_delete(IntPtr trap);

        [DllImport(LibraryName)]
        public static extern void wasm_trap_message(IntPtr trap, out wasm_byte_vec_t message);

        // Table type imports

        [DllImport(LibraryName)]
        public static extern IntPtr wasm_tabletype_element(IntPtr tableType);

        [DllImport(LibraryName)]
        public static unsafe extern wasm_limits_t* wasm_tabletype_limits(IntPtr tableType);

        // Memory type imports

        [DllImport(LibraryName)]
        public static unsafe extern wasm_limits_t* wasm_memorytype_limits(IntPtr memoryType);

        // Global imports

        [DllImport(LibraryName)]
        public static unsafe extern GlobalHandle wasm_global_new(StoreHandle handle, GlobalTypeHandle globalType, wasm_val_t* initialValue);

        [DllImport(LibraryName)]
        public static extern void wasm_global_delete(IntPtr global);

        [DllImport(LibraryName)]
        public static extern IntPtr wasm_global_type(IntPtr global);

        [DllImport(LibraryName)]
        public static unsafe extern void wasm_global_get(IntPtr global, wasm_val_t* value);

        [DllImport(LibraryName)]
        public static unsafe extern void wasm_global_set(IntPtr global, wasm_val_t* value);

        // Memory imports

        [DllImport(LibraryName)]
        public static extern MemoryHandle wasm_memory_new(StoreHandle handle, MemoryTypeHandle memoryType);

        [DllImport(LibraryName)]
        public static extern void wasm_memory_delete(IntPtr memory);

        [DllImport(LibraryName)]
        public static extern IntPtr wasm_memory_type(MemoryHandle memory);

        [DllImport(LibraryName)]
        public static unsafe extern byte* wasm_memory_data(IntPtr memory);

        [DllImport(LibraryName)]
        public static extern UIntPtr wasm_memory_data_size(IntPtr memory);

        [DllImport(LibraryName)]
        public static extern uint wasm_memory_size(MemoryHandle memory);

        [DllImport(LibraryName)]
        public static extern bool wasm_memory_grow(MemoryHandle memory, uint delta);

        // WASI config

        [DllImport(LibraryName)]
        public static extern WasiConfigHandle wasi_config_new();

        [DllImport(LibraryName)]
        public static extern void wasi_config_delete(IntPtr config);

        [DllImport(LibraryName)]
        public unsafe static extern void wasi_config_set_argv(WasiConfigHandle config, int argc, byte*[] argv);

        [DllImport(LibraryName)]
        public static extern void wasi_config_inherit_argv(WasiConfigHandle config);

        [DllImport(LibraryName)]
        public static extern unsafe void wasi_config_set_env(
            WasiConfigHandle config,
            int envc,
            byte*[] names,
            byte*[] values
        );

        [DllImport(LibraryName)]
        public static extern void wasi_config_inherit_env(WasiConfigHandle config);

        [DllImport(LibraryName)]
        public static extern bool wasi_config_set_stdin_file(
            WasiConfigHandle config,
            [MarshalAs(UnmanagedType.LPUTF8Str)] string path
        ); 

        [DllImport(LibraryName)]
        public static extern void wasi_config_inherit_stdin(WasiConfigHandle config);

        [DllImport(LibraryName)]
        public static extern bool wasi_config_set_stdout_file(
            WasiConfigHandle config,
            [MarshalAs(UnmanagedType.LPUTF8Str)] string path
        ); 

        [DllImport(LibraryName)]
        public static extern void wasi_config_inherit_stdout(WasiConfigHandle config);

        [DllImport(LibraryName)]
        public static extern bool wasi_config_set_stderr_file(
            WasiConfigHandle config,
            [MarshalAs(UnmanagedType.LPUTF8Str)] string path
        ); 

        [DllImport(LibraryName)]
        public static extern void wasi_config_inherit_stderr(WasiConfigHandle config);

        [DllImport(LibraryName)]
        public static extern bool wasi_config_preopen_dir(
            WasiConfigHandle config,
            [MarshalAs(UnmanagedType.LPUTF8Str)] string path,
            [MarshalAs(UnmanagedType.LPUTF8Str)] string guestPath
        );

        // WASI instance
        [DllImport(LibraryName)]
        public static extern WasiInstanceHandle wasi_instance_new(
            StoreHandle store,
            WasiConfigHandle config,
            out IntPtr trap
        );

        [DllImport(LibraryName)]
        public static extern void wasi_instance_delete(IntPtr instance);

        [DllImport(LibraryName)]
        public static extern IntPtr wasi_instance_bind_import(WasiInstanceHandle instance, IntPtr importType);
    }
}
