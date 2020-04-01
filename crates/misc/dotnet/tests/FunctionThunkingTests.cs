using FluentAssertions;
using System;
using System.Linq;
using Xunit;

namespace Wasmtime.Tests
{
    public class FunctionThunkingFixture : ModuleFixture
    {
        protected override string ModuleFileName => "FunctionThunking.wat";
    }

    public class FunctionThunkingTests : IClassFixture<FunctionThunkingFixture>
    {
        const string THROW_MESSAGE = "Test error message for wasmtime dotnet unit tests.";

        public FunctionThunkingTests(FunctionThunkingFixture fixture)
        {
            Fixture = fixture;

            Fixture.Host.DefineFunction("env", "add", (int x, int y) => x + y);
            Fixture.Host.DefineFunction("env", "swap", (int x, int y) => (y, x));
            Fixture.Host.DefineFunction("env", "do_throw", () => throw new Exception(THROW_MESSAGE));
            Fixture.Host.DefineFunction("env", "check_string", (Caller caller, int address, int length) => {
                caller.GetMemory("mem").ReadString(address, length).Should().Be("Hello World");
            });
        }

        private FunctionThunkingFixture Fixture { get; }

        [Fact]
        public void ItBindsImportMethodsAndCallsThemCorrectly()
        {
            using dynamic instance = Fixture.Host.Instantiate(Fixture.Module);

            int x = instance.add(40, 2);
            x.Should().Be(42);
            x = instance.add(22, 5);
            x.Should().Be(27);

            object[] results = instance.swap(10, 100);
            results.Should().Equal(new object[] { 100, 10 });

            instance.check_string();

            // Collect garbage to make sure delegate function pointers pasted to wasmtime are rooted.
            GC.Collect();
            GC.WaitForPendingFinalizers();

            x = instance.add(1970, 50);
            x.Should().Be(2020);

            results = instance.swap(2020, 1970);
            results.Should().Equal(new object[] { 1970, 2020 });

            instance.check_string();
        }

        [Fact]
        public void ItPropagatesExceptionsToCallersViaTraps()
        {
            using dynamic instance = Fixture.Host.Instantiate(Fixture.Module);

            Action action = () => instance.do_throw();

            action
                .Should()
                .Throw<TrapException>()
                .WithMessage(THROW_MESSAGE);
        }
    }
}
