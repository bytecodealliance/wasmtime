using System;
using Wasmtime;

namespace HelloExample
{
    class Host : IHost
    {
        public Instance Instance { get; set; }

        [Import("print_global")]
        public void PrintGlobal()
        {
            Console.WriteLine($"The value of the global is: {Global.Value}.");
        }

        [Import("global")]
        public readonly MutableGlobal<int> Global = new MutableGlobal<int>(1);
    }

    class Program
    {
        static void Main(string[] args)
        {
            using var engine = new Engine();
            using var store = engine.CreateStore();
            using var module = store.CreateModule("global.wasm");
            using dynamic instance = module.Instantiate(new Host());

            instance.run(20);
        }
    }
}
