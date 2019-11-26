using System;
using Wasmtime;

namespace HelloExample
{
    class Host : IHost
    {
        public Instance Instance { get; set; }

        [Import("log")]
        public void Log(int address, int length)
        {
            var message = Instance.Externs.Memories[0].ReadString(address, length);
            Console.WriteLine($"Message from WebAssembly: {message}");
        }
    }

    class Program
    {
        static void Main(string[] args)
        {
            using (var engine = new Engine())
            using (var store = engine.CreateStore())
            using (var module = store.CreateModule("memory.wasm"))
            using (dynamic instance = module.Instantiate(new Host()))
            {
                instance.run();
            }
        }
    }
}
