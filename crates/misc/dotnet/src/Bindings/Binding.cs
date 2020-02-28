using System;
using System.Collections.Generic;
using System.Linq;
using System.Reflection;
using System.Runtime.InteropServices;
using Wasmtime.Imports;

namespace Wasmtime.Bindings
{
    /// <summary>
    /// Represents an abstract host binding.
    /// </summary>
    internal abstract class Binding
    {
        public abstract SafeHandle Bind(Store store, IHost host);

        public static WasmtimeException CreateBindingException(Import import, MemberInfo member, string message)
        {
            return new WasmtimeException($"Unable to bind '{member.DeclaringType.Name}.{member.Name}' to WebAssembly import '{import}': {message}.");
        }

        public static List<Binding> GetImportBindings(Module module, Wasi wasi = null, IHost host = null)
        {
            if (module is null)
            {
                throw new ArgumentNullException(nameof(module));
            }

            var bindings = new List<Binding>();
            var flags = BindingFlags.Public | BindingFlags.Instance | BindingFlags.Static | BindingFlags.DeclaredOnly;
            var type = host?.GetType();
            var methods = type?.GetMethods(flags).Where(m => !m.IsSpecialName && Attribute.IsDefined(m, typeof(ImportAttribute)));
            var fields = type?.GetFields(flags).Where(m => !m.IsSpecialName && Attribute.IsDefined(m, typeof(ImportAttribute)));

            foreach (var import in module.Imports.All)
            {
                var wasiBinding = wasi?.Bind(import);
                if (!(wasiBinding is null))
                {
                    bindings.Add(wasiBinding);
                    continue;
                }

                switch (import)
                {
                    case FunctionImport func:
                        bindings.Add(BindFunction(func, methods));
                        break;

                    case GlobalImport global:
                        bindings.Add(BindGlobal(global, fields));
                        break;

                    case MemoryImport memory:
                        bindings.Add(BindMemory(memory, fields));
                        break;

                    default:
                        throw new NotSupportedException("Unsupported import binding type.");
                }
            }

            return bindings;
        }

        private static FunctionBinding BindFunction(FunctionImport import, IEnumerable<MethodInfo> methods)
        {
            var method = methods?.Where(m =>
                {
                    var attribute = (ImportAttribute)m.GetCustomAttribute(typeof(ImportAttribute));
                    if (attribute is null)
                    {
                        return false;
                    }

                    return attribute.Name == import.Name &&
                            ((string.IsNullOrEmpty(attribute.Module) &&
                            string.IsNullOrEmpty(import.ModuleName)) ||
                            attribute.Module == import.ModuleName);
                }
            ).FirstOrDefault();

            if (method is null)
            {
                throw new WasmtimeException($"Failed to bind function import '{import}': the host does not contain a method with a matching 'Import' attribute.");
            }

            return new FunctionBinding(import, method);
        }

        private static GlobalBinding BindGlobal(GlobalImport import, IEnumerable<FieldInfo> fields)
        {
            var field = fields?.Where(f =>
                {
                    var attribute = (ImportAttribute)f.GetCustomAttribute(typeof(ImportAttribute));
                    return attribute.Name == import.Name &&
                           ((string.IsNullOrEmpty(attribute.Module) &&
                            string.IsNullOrEmpty(import.ModuleName)) ||
                            attribute.Module == import.ModuleName);
                }
            ).FirstOrDefault();

            if (field is null)
            {
                throw new WasmtimeException($"Failed to bind global import '{import}': the host does not contain a global field with a matching 'Import' attribute.");
            }

            return new GlobalBinding(import, field);
        }

        private static MemoryBinding BindMemory(MemoryImport import, IEnumerable<FieldInfo> fields)
        {
            var field = fields?.Where(f =>
                {
                    var attribute = (ImportAttribute)f.GetCustomAttribute(typeof(ImportAttribute));
                    return attribute.Name == import.Name &&
                           ((string.IsNullOrEmpty(attribute.Module) &&
                            string.IsNullOrEmpty(import.ModuleName)) ||
                            attribute.Module == import.ModuleName);
                }
            ).FirstOrDefault();

            if (field is null)
            {
                throw new WasmtimeException($"Failed to bind memory import '{import}': the host does not contain a memory field with a matching 'Import' attribute.");
            }

            return new MemoryBinding(import, field);
        }
    }
}
