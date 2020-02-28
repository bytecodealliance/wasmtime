using System;
using Wasmtime;

namespace HelloExample
{
    class Host : IHost
    {
        public Instance Instance { get; set; }

        [Import("hello")]
        public void SayHello()
        {
            Console.WriteLine("Hello from C#, WebAssembly!");
        }
    }

    class Program
    {
        static void Main(string[] args)
        {
            using var engine = new Engine();
            using var store = engine.CreateStore();
            using var module = store.CreateModule("hello.wasm");
            using dynamic instance = module.Instantiate(new Host());

            instance.run();
        }
    }
}
