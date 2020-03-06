using System;
using System.IO;
using Wasmtime;

namespace Wasmtime.Tests
{
    public abstract class ModuleFixture : IDisposable
    {
        public ModuleFixture()
        {
            Engine = new EngineBuilder()
                .WithMultiValue(true)
                .WithReferenceTypes(true)
                .Build();
            Store = Engine.CreateStore();
            var wat = Path.Combine("Modules", ModuleFileName);
            var wasm = Engine.WatToWasm(File.ReadAllText(wat));
            Module = Store.CreateModule(wat, wasm);
        }

        public void Dispose()
        {
            if (!(Module is null))
            {
                Module.Dispose();
                Module = null;
            }

            if (!(Store is null))
            {
                Store.Dispose();
                Store = null;
            }

            if (!(Engine is null))
            {
                Engine.Dispose();
                Engine = null;
            }
        }

        public Engine Engine { get; set; }
        public Store Store { get; set; }
        public Module Module { get; set; }

        protected abstract string ModuleFileName { get; }
    }
}
