#include <wasmtime/store.hh>

#include <gtest/gtest.h>

using namespace wasmtime;

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
