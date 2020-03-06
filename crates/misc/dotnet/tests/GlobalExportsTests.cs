using System;
using System.Collections.Generic;
using System.IO;
using System.Linq;
using FluentAssertions;
using Wasmtime;
using Xunit;

namespace Wasmtime.Tests
{
    public class GlobalExportsFixture : ModuleFixture
    {
        protected override string ModuleFileName => "GlobalExports.wat";
    }

    public class GlobalExportsTests : IClassFixture<GlobalExportsFixture>
    {
        public class Host : IHost
        {
            public Instance Instance { get; set; }
        }

        public GlobalExportsTests(GlobalExportsFixture fixture)
        {
            Fixture = fixture;
        }

        private GlobalExportsFixture Fixture { get; set; }

        [Theory]
        [MemberData(nameof(GetGlobalExports))]
        public void ItHasTheExpectedGlobalExports(string exportName, ValueKind expectedKind, bool expectedMutable)
        {
            var export = Fixture.Module.Exports.Globals.Where(f => f.Name == exportName).FirstOrDefault();
            export.Should().NotBeNull();
            export.Kind.Should().Be(expectedKind);
            export.IsMutable.Should().Be(expectedMutable);
        }

        [Fact]
        public void ItHasTheExpectedNumberOfExportedGlobals()
        {
            GetGlobalExports().Count().Should().Be(Fixture.Module.Exports.Globals.Count);
        }

        [Fact]
        public void ItCreatesExternsForTheGlobals()
        {
            using var instance = Fixture.Module.Instantiate(new Host());

            dynamic dyn = instance;
            var globals = instance.Externs.Globals;
            globals.Count.Should().Be(8);

            var i32 = globals[0];
            i32.Name.Should().Be("global_i32");
            i32.Kind.Should().Be(ValueKind.Int32);
            i32.IsMutable.Should().Be(false);
            i32.Value.Should().Be(0);

            var i32Mut = globals[1];
            i32Mut.Name.Should().Be("global_i32_mut");
            i32Mut.Kind.Should().Be(ValueKind.Int32);
            i32Mut.IsMutable.Should().Be(true);
            i32Mut.Value.Should().Be(1);
            i32Mut.Value = 11;
            i32Mut.Value.Should().Be(11);
            dyn.global_i32_mut = 12;
            ((int)dyn.global_i32_mut).Should().Be(12);
            i32Mut.Value.Should().Be(12);

            var i64 = globals[2];
            i64.Name.Should().Be("global_i64");
            i64.Kind.Should().Be(ValueKind.Int64);
            i64.IsMutable.Should().Be(false);
            i64.Value.Should().Be(2);

            var i64Mut = globals[3];
            i64Mut.Name.Should().Be("global_i64_mut");
            i64Mut.Kind.Should().Be(ValueKind.Int64);
            i64Mut.IsMutable.Should().Be(true);
            i64Mut.Value.Should().Be(3);
            i64Mut.Value = 13;
            i64Mut.Value.Should().Be(13);
            dyn.global_i64_mut = 14;
            ((long)dyn.global_i64_mut).Should().Be(14);
            i64Mut.Value.Should().Be(14);

            var f32 = globals[4];
            f32.Name.Should().Be("global_f32");
            f32.Kind.Should().Be(ValueKind.Float32);
            f32.IsMutable.Should().Be(false);
            f32.Value.Should().Be(4);

            var f32Mut = globals[5];
            f32Mut.Name.Should().Be("global_f32_mut");
            f32Mut.Kind.Should().Be(ValueKind.Float32);
            f32Mut.IsMutable.Should().Be(true);
            f32Mut.Value.Should().Be(5);
            f32Mut.Value = 15;
            f32Mut.Value.Should().Be(15);
            dyn.global_f32_mut = 16;
            ((float)dyn.global_f32_mut).Should().Be(16);
            f32Mut.Value.Should().Be(16);

            var f64 = globals[6];
            f64.Name.Should().Be("global_f64");
            f64.Kind.Should().Be(ValueKind.Float64);
            f64.IsMutable.Should().Be(false);
            f64.Value.Should().Be(6);

            var f64Mut = globals[7];
            f64Mut.Name.Should().Be("global_f64_mut");
            f64Mut.Kind.Should().Be(ValueKind.Float64);
            f64Mut.IsMutable.Should().Be(true);
            f64Mut.Value.Should().Be(7);
            f64Mut.Value = 17;
            f64Mut.Value.Should().Be(17);
            dyn.global_f64_mut = 17;
            ((double)dyn.global_f64_mut).Should().Be(17);
            f64Mut.Value.Should().Be(17);

            Action action = () => i32.Value = 0;
            action
                .Should()
                .Throw<InvalidOperationException>()
                .WithMessage("The value of global 'global_i32' cannot be modified.");
            action = () => dyn.global_i32 = 0;
            action
                .Should()
                .Throw<InvalidOperationException>()
                .WithMessage("The value of global 'global_i32' cannot be modified.");

            action = () => i64.Value = 0;
            action
                .Should()
                .Throw<InvalidOperationException>()
                .WithMessage("The value of global 'global_i64' cannot be modified.");
            action = () => dyn.global_i64 = 0;
            action
                .Should()
                .Throw<InvalidOperationException>()
                .WithMessage("The value of global 'global_i64' cannot be modified.");

            action = () => f32.Value = 0;
            action
                .Should()
                .Throw<InvalidOperationException>()
                .WithMessage("The value of global 'global_f32' cannot be modified.");
            action = () => dyn.global_f32 = 0;
            action
                .Should()
                .Throw<InvalidOperationException>()
                .WithMessage("The value of global 'global_f32' cannot be modified.");

            action = () => f64.Value = 0;
            action
                .Should()
                .Throw<InvalidOperationException>()
                .WithMessage("The value of global 'global_f64' cannot be modified.");
            action = () => dyn.global_f64 = 0;
            action
                .Should()
                .Throw<InvalidOperationException>()
                .WithMessage("The value of global 'global_f64' cannot be modified.");
        }

        public static IEnumerable<object[]> GetGlobalExports()
        {
            yield return new object[] {
                "global_i32",
                ValueKind.Int32,
                false
            };

            yield return new object[] {
                "global_i32_mut",
                ValueKind.Int32,
                true
            };

            yield return new object[] {
                "global_i64",
                ValueKind.Int64,
                false
            };

            yield return new object[] {
                "global_i64_mut",
                ValueKind.Int64,
                true
            };

            yield return new object[] {
                "global_f32",
                ValueKind.Float32,
                false
            };

            yield return new object[] {
                "global_f32_mut",
                ValueKind.Float32,
                true
            };

            yield return new object[] {
                "global_f64",
                ValueKind.Float64,
                false
            };

            yield return new object[] {
                "global_f64_mut",
                ValueKind.Float64,
                true
            };
        }
    }
}
