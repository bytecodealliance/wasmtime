using System;
using System.Collections.Generic;
using System.Linq;
using FluentAssertions;
using Xunit;

namespace Wasmtime.Tests
{
    public class GlobalImportsFixture : ModuleFixture
    {
        protected override string ModuleFileName => "GlobalImports.wasm";
    }

    public class GlobalImportsTests : IClassFixture<GlobalImportsFixture>
    {
        public GlobalImportsTests(GlobalImportsFixture fixture)
        {
            Fixture = fixture;
        }

        private GlobalImportsFixture Fixture { get; set; }

        [Theory]
        [MemberData(nameof(GetGlobalImports))]
        public void ItHasTheExpectedGlobalImports(string importModule, string importName, ValueKind expectedKind, bool expectedMutable)
        {
            var import = Fixture.Module.Imports.Globals.Where(f => f.ModuleName == importModule && f.Name == importName).FirstOrDefault();
            import.Should().NotBeNull();
            import.Kind.Should().Be(expectedKind);
            import.IsMutable.Should().Be(expectedMutable);
        }

        [Fact]
        public void ItHasTheExpectedNumberOfExportedGlobals()
        {
            GetGlobalImports().Count().Should().Be(Fixture.Module.Imports.Globals.Count);
        }

        public static IEnumerable<object[]> GetGlobalImports()
        {
            yield return new object[] {
                "",
                "global_i32",
                ValueKind.Int32,
                false
            };

            yield return new object[] {
                "",
                "global_i32_mut",
                ValueKind.Int32,
                true
            };

            yield return new object[] {
                "",
                "global_i64",
                ValueKind.Int64,
                false
            };

            yield return new object[] {
                "",
                "global_i64_mut",
                ValueKind.Int64,
                true
            };

            yield return new object[] {
                "",
                "global_f32",
                ValueKind.Float32,
                false
            };

            yield return new object[] {
                "",
                "global_f32_mut",
                ValueKind.Float32,
                true
            };

            yield return new object[] {
                "",
                "global_f64",
                ValueKind.Float64,
                false
            };

            yield return new object[] {
                "",
                "global_f64_mut",
                ValueKind.Float64,
                true
            };

            yield return new object[] {
                "other",
                "global_from_module",
                ValueKind.Int32,
                false
            };
        }
    }
}
