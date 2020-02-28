using System;
using FluentAssertions;
using Xunit;

namespace Wasmtime.Tests
{
    public class GlobalImportBindingFixture : ModuleFixture
    {
        protected override string ModuleFileName => "GlobalImportBindings.wasm";
    }

    public class GlobalImportBindingTests : IClassFixture<GlobalImportBindingFixture>
    {
        class NoImportsHost : IHost
        {
            public Instance Instance { get; set; }
        }

        class GlobalIsStaticHost : IHost
        {
            public Instance Instance { get; set; }

            [Import("global_i32_mut")]
            public static int x = 0;
        }

        class GlobalIsNotReadOnlyHost : IHost
        {
            public Instance Instance { get; set; }

            [Import("global_i32_mut")]
            public int x = 0;
        }

        class NotAGlobalHost : IHost
        {
            public Instance Instance { get; set; }

            [Import("global_i32_mut")]
            public readonly int x = 0;
        }

        class NotAValidGlobalTypeHost : IHost
        {
            public struct NotAValue
            {
            }

            public Instance Instance { get; set; }

            [Import("global_i32_mut")]
            public readonly MutableGlobal<NotAValue> x = new MutableGlobal<NotAValue>(new NotAValue());
        }

        class TypeMismatchHost : IHost
        {
            public Instance Instance { get; set; }

            [Import("global_i32_mut")]
            public readonly MutableGlobal<long> x = new MutableGlobal<long>(0);
        }

        class NotMutHost : IHost
        {
            public Instance Instance { get; set; }

            [Import("global_i32_mut")]
            public readonly Global<int> Int32Mut = new Global<int>(0);
        }

        class MutHost : IHost
        {
            public Instance Instance { get; set; }

            [Import("global_i32_mut")]
            public readonly MutableGlobal<int> Int32Mut = new MutableGlobal<int>(0);

            [Import("global_i32")]
            public readonly MutableGlobal<int> Int32 = new MutableGlobal<int>(0);
        }

        class ValidHost : IHost
        {
            public Instance Instance { get; set; }

            [Import("global_i32_mut")]
            public readonly MutableGlobal<int> Int32Mut = new MutableGlobal<int>(0);

            [Import("global_i32")]
            public readonly Global<int> Int32 = new Global<int>(1);

            [Import("global_i64_mut")]
            public readonly MutableGlobal<long> Int64Mut = new MutableGlobal<long>(2);

            [Import("global_i64")]
            public readonly Global<long> Int64 = new Global<long>(3);

            [Import("global_f32_mut")]
            public readonly MutableGlobal<float> Float32Mut = new MutableGlobal<float>(4);

            [Import("global_f32")]
            public readonly Global<float> Float32 = new Global<float>(5);

            [Import("global_f64_mut")]
            public readonly MutableGlobal<double> Float64Mut = new MutableGlobal<double>(6);

            [Import("global_f64")]
            public readonly Global<double> Float64 = new Global<double>(7);
        }

        public GlobalImportBindingTests(GlobalImportBindingFixture fixture)
        {
            Fixture = fixture;
        }

        private GlobalImportBindingFixture Fixture { get; set; }

        [Fact]
        public void ItFailsToInstantiateWithMissingImport()
        {
            Action action = () => { using var instance = Fixture.Module.Instantiate(new NoImportsHost()); };

            action
                .Should()
                .Throw<WasmtimeException>()
                .WithMessage("Failed to bind global import 'global_i32_mut': the host does not contain a global field with a matching 'Import' attribute.");
        }

        [Fact]
        public void ItFailsToInstantiateWithStaticField()
        {
            Action action = () => { using var instance = Fixture.Module.Instantiate(new GlobalIsStaticHost()); };

            action
                .Should()
                .Throw<WasmtimeException>()
                .WithMessage("Unable to bind 'GlobalIsStaticHost.x' to WebAssembly import 'global_i32_mut': field cannot be static.");
        }

        [Fact]
        public void ItFailsToInstantiateWithNonReadOnlyField()
        {
            Action action = () => { using var instance = Fixture.Module.Instantiate(new GlobalIsNotReadOnlyHost()); };

            action
                .Should()
                .Throw<WasmtimeException>()
                .WithMessage("Unable to bind 'GlobalIsNotReadOnlyHost.x' to WebAssembly import 'global_i32_mut': field must be readonly.");
        }

        [Fact]
        public void ItFailsToInstantiateWithInvalidType()
        {
            Action action = () => { using var instance = Fixture.Module.Instantiate(new NotAGlobalHost()); };

            action
                .Should()
                .Throw<WasmtimeException>()
                .WithMessage("Unable to bind 'NotAGlobalHost.x' to WebAssembly import 'global_i32_mut': field is expected to be of type 'Global<T>'.");
        }

        [Fact]
        public void ItFailsToInstantiateWithInvalidGlobalType()
        {
            Action action = () => { using var instance = Fixture.Module.Instantiate(new NotAValidGlobalTypeHost()); };

            action
                .Should()
                .Throw<NotSupportedException>()
                .WithMessage("Type 'Wasmtime.Tests.GlobalImportBindingTests+NotAValidGlobalTypeHost+NotAValue' is not a supported WebAssembly value type.");
        }

        [Fact]
        public void ItFailsToInstantiateWithGlobalTypeMismatch()
        {
            Action action = () => { using var instance = Fixture.Module.Instantiate(new TypeMismatchHost()); };

            action
                .Should()
                .Throw<WasmtimeException>()
                .WithMessage("Unable to bind 'TypeMismatchHost.x' to WebAssembly import 'global_i32_mut': global type argument is expected to be of type 'int'.");
        }

        [Fact]
        public void ItFailsToInstantiateWhenGlobalIsNotMut()
        {
            Action action = () => { using var instance = Fixture.Module.Instantiate(new NotMutHost()); };

            action
                .Should()
                .Throw<WasmtimeException>()
                .WithMessage("Unable to bind 'NotMutHost.Int32Mut' to WebAssembly import 'global_i32_mut': the import is mutable (use the 'MutableGlobal' type).");
        }

        [Fact]
        public void ItFailsToInstantiateWhenGlobalIsMut()
        {
            Action action = () => { using var instance = Fixture.Module.Instantiate(new MutHost()); };

            action
                .Should()
                .Throw<WasmtimeException>()
                .WithMessage("Unable to bind 'MutHost.Int32' to WebAssembly import 'global_i32': the import is constant (use the 'Global' type).");
        }

        [Fact]
        public void ItBindsTheGlobalsCorrectly()
        {
            var host = new ValidHost();
            using dynamic instance = Fixture.Module.Instantiate(host);

            host.Int32Mut.Value.Should().Be(0);
            ((int)instance.get_global_i32_mut()).Should().Be(0);
            host.Int32.Value.Should().Be(1);
            ((int)instance.get_global_i32()).Should().Be(1);
            host.Int64Mut.Value.Should().Be(2);
            ((long)instance.get_global_i64_mut()).Should().Be(2);
            host.Int64.Value.Should().Be(3);
            ((long)instance.get_global_i64()).Should().Be(3);
            host.Float32Mut.Value.Should().Be(4);
            ((float)instance.get_global_f32_mut()).Should().Be(4);
            host.Float32.Value.Should().Be(5);
            ((float)instance.get_global_f32()).Should().Be(5);
            host.Float64Mut.Value.Should().Be(6);
            ((double)instance.get_global_f64_mut()).Should().Be(6);
            host.Float64.Value.Should().Be(7);
            ((double)instance.get_global_f64()).Should().Be(7);

            host.Int32Mut.Value = 10;
            host.Int32Mut.Value.Should().Be(10);
            ((int)instance.get_global_i32_mut()).Should().Be(10);
            instance.set_global_i32_mut(11);
            host.Int32Mut.Value.Should().Be(11);
            ((int)instance.get_global_i32_mut()).Should().Be(11);

            host.Int64Mut.Value = 12;
            host.Int64Mut.Value.Should().Be(12);
            ((long)instance.get_global_i64_mut()).Should().Be(12);
            instance.set_global_i64_mut(13);
            host.Int64Mut.Value.Should().Be(13);
            ((long)instance.get_global_i64_mut()).Should().Be(13);

            host.Float32Mut.Value = 14;
            host.Float32Mut.Value.Should().Be(14);
            ((float)instance.get_global_f32_mut()).Should().Be(14);
            instance.set_global_f32_mut(15);
            host.Float32Mut.Value.Should().Be(15);
            ((float)instance.get_global_f32_mut()).Should().Be(15);

            host.Float64Mut.Value = 16;
            host.Float64Mut.Value.Should().Be(16);
            ((double)instance.get_global_f64_mut()).Should().Be(16);
            instance.set_global_f64_mut(17);
            host.Float64Mut.Value.Should().Be(17);
            ((double)instance.get_global_f64_mut()).Should().Be(17);
        }
    }
}
