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
      Module::compile(engine, "(module (func (export \"\") unreachable))").unwrap();
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

  EXPECT_TRUE(func.call(store, {}).err().message().find("unreachable") !=
              std::string::npos);
  EXPECT_EQ(func.call(store, {1}).err().message(),
            "expected 0 arguments, got 1");
}
