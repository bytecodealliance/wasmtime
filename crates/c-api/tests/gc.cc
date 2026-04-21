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

TEST(StructRef, CreateAndReadFields) {
  Config config;
  config.wasm_gc(true);
  Engine engine(std::move(config));
  Store store(engine);
  auto cx = store.context();

  // Create a struct type with two mutable i32 fields.
  auto ty = StructType::create(engine, {
                                           FieldType::mut_(WASMTIME_I32),
                                           FieldType::mut_(WASMTIME_I32),
                                       });
  auto pre = StructRefPre::create(cx, ty);

  // Allocate a struct with field values 10 and 20.
  auto result =
      StructRef::create(cx, pre, {Val(int32_t(10)), Val(int32_t(20))});
  ASSERT_TRUE(result);
  StructRef s = result.ok();

  // Read fields back.
  auto v0 = s.field(cx, 0);
  ASSERT_TRUE(v0);
  EXPECT_EQ(v0.ok().i32(), 10);

  auto v1 = s.field(cx, 1);
  ASSERT_TRUE(v1);
  EXPECT_EQ(v1.ok().i32(), 20);

  // Write field 0 to 42 and read back.
  auto set_result = s.set_field(cx, 0, Val(int32_t(42)));
  ASSERT_TRUE(set_result);

  auto v0b = s.field(cx, 0);
  ASSERT_TRUE(v0b);
  EXPECT_EQ(v0b.ok().i32(), 42);
}

TEST(StructRef, UpcastAndDowncast) {
  Config config;
  config.wasm_gc(true);
  Engine engine(std::move(config));
  Store store(engine);
  auto cx = store.context();

  auto ty = StructType::create(engine, {
                                           FieldType::const_(WASMTIME_I32),
                                       });
  auto pre = StructRefPre::create(cx, ty);

  auto result = StructRef::create(cx, pre, {Val(int32_t(99))});
  ASSERT_TRUE(result);
  StructRef s = result.ok();

  // Upcast to eqref.
  EqRef eq = s.to_eqref();
  EXPECT_TRUE(eq.is_struct(cx));
  EXPECT_FALSE(eq.is_i31(cx));

  // Downcast back to structref.
  StructRef s2 = eq.as_struct(cx);
  auto v = s2.field(cx, 0);
  ASSERT_TRUE(v);
  EXPECT_EQ(v.ok().i32(), 99);

  // Upcast to anyref.
  AnyRef any = s.to_anyref();
  EXPECT_FALSE(any.is_i31(cx));
}

TEST(ArrayRef, CreateAndReadElements) {
  Config config;
  config.wasm_gc(true);
  Engine engine(std::move(config));
  Store store(engine);
  auto cx = store.context();

  // Create an array type with mutable i32 elements.
  auto ty = ArrayType::create(engine, FieldType::mut_(WASMTIME_I32));
  auto pre = ArrayRefPre::create(cx, ty);

  // Allocate an array of 5 elements, all initialized to 7.
  auto result = ArrayRef::create(cx, pre, Val(int32_t(7)), 5);
  ASSERT_TRUE(result);
  ArrayRef arr = result.ok();

  // Check length.
  auto len_result = arr.len(cx);
  ASSERT_TRUE(len_result);
  EXPECT_EQ(len_result.ok(), 5u);

  // Read elements.
  for (uint32_t i = 0; i < 5; i++) {
    auto v = arr.get(cx, i);
    ASSERT_TRUE(v);
    EXPECT_EQ(v.ok().i32(), 7);
  }

  // Write element 2 to 42, read back.
  auto set_result = arr.set(cx, 2, Val(int32_t(42)));
  ASSERT_TRUE(set_result);

  auto v2 = arr.get(cx, 2);
  ASSERT_TRUE(v2);
  EXPECT_EQ(v2.ok().i32(), 42);
}

TEST(ArrayRef, UpcastAndDowncast) {
  Config config;
  config.wasm_gc(true);
  Engine engine(std::move(config));
  Store store(engine);
  auto cx = store.context();

  auto ty = ArrayType::create(engine, FieldType::const_(WASMTIME_I32));
  auto pre = ArrayRefPre::create(cx, ty);

  auto result = ArrayRef::create(cx, pre, Val(int32_t(99)), 3);
  ASSERT_TRUE(result);
  ArrayRef arr = result.ok();

  // Upcast to eqref.
  EqRef eq = arr.to_eqref();
  EXPECT_TRUE(eq.is_array(cx));
  EXPECT_FALSE(eq.is_struct(cx));
  EXPECT_FALSE(eq.is_i31(cx));

  // Downcast back to arrayref.
  ArrayRef arr2 = eq.as_array(cx);
  auto len = arr2.len(cx);
  ASSERT_TRUE(len);
  EXPECT_EQ(len.ok(), 3u);

  auto v = arr2.get(cx, 0);
  ASSERT_TRUE(v);
  EXPECT_EQ(v.ok().i32(), 99);

  // Upcast to anyref.
  AnyRef any = arr.to_anyref();
  EXPECT_FALSE(any.is_i31(cx));
}

TEST(AnyRef, DowncastI31) {
  Config config;
  config.wasm_gc(true);
  Engine engine(std::move(config));
  Store store(engine);
  auto cx = store.context();

  AnyRef any = AnyRef::i31(cx, 42);
  EXPECT_TRUE(any.is_i31(cx));
  EXPECT_TRUE(any.is_eqref(cx));
  EXPECT_FALSE(any.is_struct(cx));
  EXPECT_FALSE(any.is_array(cx));

  // Downcast to eqref.
  auto opt_eq = any.as_eqref(cx);
  EXPECT_TRUE(opt_eq);
  EqRef eq = *opt_eq;
  EXPECT_TRUE(eq.is_i31(cx));
  auto val = eq.i31_get_u(cx);
  ASSERT_TRUE(val.has_value());
  EXPECT_EQ(*val, 42u);
}

TEST(AnyRef, DowncastStruct) {
  Config config;
  config.wasm_gc(true);
  Engine engine(std::move(config));
  Store store(engine);
  auto cx = store.context();

  auto ty = StructType::create(engine, {FieldType::const_(WASMTIME_I32)});
  auto pre = StructRefPre::create(cx, ty);
  auto result = StructRef::create(cx, pre, {Val(int32_t(77))});
  ASSERT_TRUE(result);
  StructRef s = result.ok();

  AnyRef any = s.to_anyref();
  EXPECT_TRUE(any.is_eqref(cx));
  EXPECT_TRUE(any.is_struct(cx));
  EXPECT_FALSE(any.is_array(cx));
  EXPECT_FALSE(any.is_i31(cx));

  // Downcast back to struct.
  auto opt_s2 = any.as_struct(cx);
  EXPECT_TRUE(opt_s2);
  StructRef s2 = *opt_s2;
  auto v = s2.field(cx, 0);
  ASSERT_TRUE(v);
  EXPECT_EQ(v.ok().i32(), 77);
}

TEST(AnyRef, DowncastArray) {
  Config config;
  config.wasm_gc(true);
  Engine engine(std::move(config));
  Store store(engine);
  auto cx = store.context();

  auto ty = ArrayType::create(engine, FieldType::const_(WASMTIME_I32));
  auto pre = ArrayRefPre::create(cx, ty);
  auto result = ArrayRef::create(cx, pre, Val(int32_t(55)), 2);
  ASSERT_TRUE(result);
  ArrayRef arr = result.ok();

  AnyRef any = arr.to_anyref();
  EXPECT_TRUE(any.is_eqref(cx));
  EXPECT_FALSE(any.is_struct(cx));
  EXPECT_TRUE(any.is_array(cx));
  EXPECT_FALSE(any.is_i31(cx));

  // Downcast back to array.
  auto opt_arr2 = any.as_array(cx);
  EXPECT_TRUE(opt_arr2);
  ArrayRef arr2 = *opt_arr2;
  auto len = arr2.len(cx);
  ASSERT_TRUE(len);
  EXPECT_EQ(len.ok(), 2u);
}
