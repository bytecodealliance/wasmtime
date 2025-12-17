#include <wasmtime/store.hh>

#include <gtest/gtest.h>
#include <wasmtime/config.hh>
#include <wasmtime/error.hh>
#include <wasmtime/func.hh>
#include <wasmtime/instance.hh>
#include <wasmtime/module.hh>

using namespace wasmtime;

template <typename T, typename E> T unwrap(Result<T, E> result) {
  if (result) {
    return result.ok();
  }
  std::cerr << "error: " << result.err().message() << "\n";
  std::abort();
}

TEST(Store, Smoke) {
  Engine engine;
  Store store(engine);
  Store store2 = std::move(store);
  Store store3(std::move(store2));

  store = Store(engine);
  store.limiter(-1, -1, -1, -1, -1);
  store.context().gc();
  store.context().get_fuel().err();
  store.context().set_fuel(1).err();
  store.context().set_epoch_deadline(1);
}

TEST(Store, EpochDeadlineCallback) {
  Config config;
  config.epoch_interruption(true);
  Engine engine(std::move(config));

  size_t num_calls = 0;
  Store store(engine);
  store.epoch_deadline_callback(
      [&num_calls](wasmtime::Store::Context /* context */,
                   uint64_t &epoch_deadline_delta)
          -> wasmtime::Result<wasmtime::DeadlineKind> {
        epoch_deadline_delta += 1;
        num_calls += 1;
        return wasmtime::DeadlineKind::Continue;
      });

  store.context().set_epoch_deadline(1);

  Module m = unwrap(Module::compile(engine, "(module (func (export \"f\")))"));
  Instance i = unwrap(Instance::create(store, m, {}));

  auto f = std::get<Func>(*i.get(store, "f"));

  unwrap(f.call(store, {}));
  ASSERT_EQ(num_calls, 0);

  engine.increment_epoch();
  unwrap(f.call(store, {}));
  ASSERT_EQ(num_calls, 1);

  /// epoch_deadline_delta increased by 1 in the callback
  unwrap(f.call(store, {}));
  ASSERT_EQ(num_calls, 1);

  engine.increment_epoch();
  unwrap(f.call(store, {}));
  ASSERT_EQ(num_calls, 2);

  store.epoch_deadline_callback(
      [](wasmtime::Store::Context /* context */, uint64_t &epoch_deadline_delta)
          -> wasmtime::Result<wasmtime::DeadlineKind> {
        return Error("error from callback");
      });

  engine.increment_epoch();
  auto result = f.call(store, {});
  EXPECT_FALSE(result);
  EXPECT_TRUE(result.err().message().find("error from callback") !=
              std::string::npos);
}
