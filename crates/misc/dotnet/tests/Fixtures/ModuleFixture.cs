using System;
using System.IO;
using Wasmtime;

namespace Wasmtime.Tests
{
    public abstract class ModuleFixture : IDisposable
    {
        public ModuleFixture()
        {
            Host = new HostBuilder()
                .WithMultiValue(true)
                .WithReferenceTypes(true)
                .Build();

            Module = Host.LoadModuleText(Path.Combine("Modules", ModuleFileName));
        }

        public void Dispose()
        {
            if (!(Module is null))
            {
                Module.Dispose();
                Module = null;
            }

            if (!(Host is null))
            {
                Host.Dispose();
                Host = null;
            }
        }

        public Host Host { get; set; }
        public Module Module { get; set; }

        protected abstract string ModuleFileName { get; }
    }
}
