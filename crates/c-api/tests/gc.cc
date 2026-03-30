#include <wasmtime/gc.hh>

#include <gtest/gtest.h>
#include <wasmtime.hh>

using namespace wasmtime;

TEST(EqRef, CopyAndMove) {
  Config config;
  config.wasm_gc(true);
  Engine engine(std::move(config));
  Store store(engine);

  auto cx = store.context();

  // Create an i31 eqref.
  EqRef eq = EqRef::from_i31(cx, 42);
  EXPECT_TRUE(eq.is_i31(cx));

  // Copy constructor
  EqRef eq2(eq);

  // Move constructor
  EqRef eq3(std::move(eq2));

  // Copy assignment
  EqRef eq4 = eq3;

  // Move assignment
  EqRef eq5 = std::move(eq4);

  // Upcast to anyref and check value
  AnyRef upcast = eq.to_anyref();
  auto val = upcast.u31(cx);
  ASSERT_TRUE(val.has_value());
  EXPECT_EQ(*val, 42u);
}

TEST(EqRef, NullEqRef) {
  wasmtime_eqref_t raw;
  wasmtime_eqref_set_null(&raw);
  EXPECT_TRUE(wasmtime_eqref_is_null(&raw));

  // Upcast null eqref to anyref via C API
  wasmtime_anyref_t out;
  wasmtime_eqref_to_anyref(&raw, &out);
  EXPECT_TRUE(wasmtime_anyref_is_null(&out));
}

TEST(I31Ref, CreateAndRead) {
  Config config;
  config.wasm_gc(true);
  Engine engine(std::move(config));
  Store store(engine);
  auto cx = store.context();

  // Create an i31 eqref
  EqRef eq = EqRef::from_i31(cx, 42);
  EXPECT_TRUE(eq.is_i31(cx));

  // Read unsigned
  auto u = eq.i31_get_u(cx);
  ASSERT_TRUE(u.has_value());
  EXPECT_EQ(*u, 42u);

  // Read signed (positive value)
  auto s = eq.i31_get_s(cx);
  ASSERT_TRUE(s.has_value());
  EXPECT_EQ(*s, 42);

  // Upcast to anyref and check is_i31
  AnyRef any = eq.to_anyref();
  EXPECT_TRUE(any.is_i31(cx));
}

TEST(I31Ref, SignedValues) {
  Config config;
  config.wasm_gc(true);
  Engine engine(std::move(config));
  Store store(engine);
  auto cx = store.context();

  // i31 wraps to 31 bits. 0x7FFFFFFF (all 31 bits set) should give -1 signed.
  EqRef eq = EqRef::from_i31(cx, 0x7FFFFFFF);
  auto u = eq.i31_get_u(cx);
  ASSERT_TRUE(u.has_value());
  EXPECT_EQ(*u, 0x7FFFFFFFu);

  auto s = eq.i31_get_s(cx);
  ASSERT_TRUE(s.has_value());
  EXPECT_EQ(*s, -1);
}
