using FluentAssertions;
using System;
using System.Linq;
using Xunit;

namespace Wasmtime.Tests
{
    public class FunctionThunkingFixture : ModuleFixture
    {
        protected override string ModuleFileName => "FunctionThunking.wasm";
    }

    public class FunctionThunkingTests : IClassFixture<FunctionThunkingFixture>
    {
        const string THROW_MESSAGE = "Test error messages for wasmtime dotnet bidnings unit tests.";

        class MyHost : IHost
        {
            public Instance Instance { get; set; }

            [Import("add", Module = "env")]
            public int Add(int x, int y) => x + y;

            [Import("do_throw", Module = "env")]
            public void Throw() => throw new Exception(THROW_MESSAGE);
        }

        public FunctionThunkingTests(FunctionThunkingFixture fixture)
        {
            Fixture = fixture;
        }

        private FunctionThunkingFixture Fixture { get; }

        [Fact]
        public void ItBindsImportMethodsAndCallsThemCorrectly()
        {
            var host = new MyHost();
            using (var instance = Fixture.Module.Instantiate(host))
            {
                var add_func = instance.Externs.Functions.Where(f => f.Name == "add_wrapper").Single();
                int invoke_add(int x, int y) => (int)add_func.Invoke(new object[] { x, y });

                invoke_add(40, 2).Should().Be(42);
                invoke_add(22, 5).Should().Be(27);

                //Collect garbage to make sure delegate function pointers pasted to wasmtime are rooted.
                GC.Collect();
                GC.WaitForPendingFinalizers();

                invoke_add(1970, 50).Should().Be(2020);
            }
        }

        [Fact]
        public void ItPropegatesExceptionsToCallersViaTraps()
        {
            var host = new MyHost();
            using (var instance = Fixture.Module.Instantiate(host))
            {
                var throw_func = instance.Externs.Functions.Where(f => f.Name == "do_throw_wrapper").Single();
                Action action = () => throw_func.Invoke();

                action
                    .Should()
                    .Throw<TrapException>()
                    .WithMessage(THROW_MESSAGE);
            }
        }
    }
}
