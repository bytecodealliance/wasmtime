#include <wasmtime/trap.hh>

#include <gtest/gtest.h>
#include <wasmtime.hh>

using namespace wasmtime;

TEST(Trap, Smoke) {
  Trap t("foo");
  EXPECT_EQ(t.message(), "foo");
  EXPECT_EQ(t.trace().size(), 0);

  Engine engine;
  Module m =
      Module::compile(engine, "(module (func (export \"\") unreachable))")
          .unwrap();
  Store store(engine);
  Instance i = Instance::create(store, m, {}).unwrap();
  auto func = std::get<Func>(*i.get(store, ""));
  auto trap = std::get<Trap>(func.call(store, {}).err().data);
  auto trace = trap.trace();
  EXPECT_EQ(trace.size(), 1);
  auto frame = *trace.begin();
  EXPECT_EQ(frame.func_name(), std::nullopt);
  EXPECT_EQ(frame.module_name(), std::nullopt);
  EXPECT_EQ(frame.func_index(), 0);
  EXPECT_EQ(frame.func_offset(), 1);
  EXPECT_EQ(frame.module_offset(), 29);
  for (auto &frame : trace) {
  }

  auto unreachable_trap = std::get<Trap>(func.call(store, {}).err().data);

  EXPECT_EQ(unreachable_trap.code(),
            WASMTIME_TRAP_CODE_UNREACHABLE_CODE_REACHED);
  EXPECT_TRUE(unreachable_trap.message().find("unreachable") !=
              std::string::npos);
  EXPECT_EQ(func.call(store, {1}).err().message(),
            "expected 0 arguments, got 1");

  Trap out_of_fuel(WASMTIME_TRAP_CODE_OUT_OF_FUEL);
  EXPECT_EQ(out_of_fuel.code(), WASMTIME_TRAP_CODE_OUT_OF_FUEL);
  EXPECT_TRUE(out_of_fuel.message().find("all fuel consumed") !=
              std::string::npos);
}

TEST(Trap, Codes) {
#define TEST_CODE(trapcode)                                                    \
  EXPECT_EQ(Trap(WASMTIME_TRAP_CODE_##trapcode).code(),                        \
            WASMTIME_TRAP_CODE_##trapcode);

  TEST_CODE(STACK_OVERFLOW);
  TEST_CODE(MEMORY_OUT_OF_BOUNDS);
  TEST_CODE(HEAP_MISALIGNED);
  TEST_CODE(TABLE_OUT_OF_BOUNDS);
  TEST_CODE(INDIRECT_CALL_TO_NULL);
  TEST_CODE(BAD_SIGNATURE);
  TEST_CODE(INTEGER_OVERFLOW);
  TEST_CODE(INTEGER_DIVISION_BY_ZERO);
  TEST_CODE(BAD_CONVERSION_TO_INTEGER);
  TEST_CODE(UNREACHABLE_CODE_REACHED);
  TEST_CODE(INTERRUPT);
  TEST_CODE(ALWAYS_TRAP_ADAPTER);
  TEST_CODE(OUT_OF_FUEL);
  TEST_CODE(ATOMIC_WAIT_NON_SHARED_MEMORY);
  TEST_CODE(NULL_REFERENCE);
  TEST_CODE(ARRAY_OUT_OF_BOUNDS);
  TEST_CODE(ALLOCATION_TOO_LARGE);
  TEST_CODE(CAST_FAILURE);
  TEST_CODE(CANNOT_ENTER_COMPONENT);
  TEST_CODE(NO_ASYNC_RESULT);
  TEST_CODE(DISABLED_OPCODE);
#undef TEST_CODE
}
