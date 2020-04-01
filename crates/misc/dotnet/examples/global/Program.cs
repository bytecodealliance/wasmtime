using System;
using Wasmtime;

namespace HelloExample
{
    class Program
    {
        static void Main(string[] args)
        {
            using var host = new Host();

            var global = host.DefineMutableGlobal("", "global", 1);

            host.DefineFunction(
                "",
                "print_global",
                () => {
                    Console.WriteLine($"The value of the global is: {global.Value}.");
                }
            );

            using var module = host.LoadModule("global.wasm");

            using dynamic instance = host.Instantiate(module);
            instance.run(20);
        }
    }
}
