#include <wasmtime/func.hh>

#include <gtest/gtest.h>
#include <wasmtime.hh>

using namespace wasmtime;
using empty_t = std::tuple<>;

TEST(TypedFunc, Smoke) {
  Engine engine;
  Store store(engine);
  Func thunk(
      store, FuncType({}, {}),
      [](auto caller, auto params, auto results) { return std::monostate(); });

  EXPECT_FALSE((thunk.typed<int, int>(store)));
  EXPECT_FALSE((thunk.typed<float, std::tuple<int32_t, uint32_t>>(store)));
  EXPECT_FALSE((thunk.typed<float, empty_t>(store)));
  EXPECT_TRUE((thunk.typed<empty_t, empty_t>(store)));

  Func pi32(
      store, FuncType({ValKind::I32}, {}),
      [](auto caller, auto params, auto results) { return std::monostate(); });

  EXPECT_FALSE((pi32.typed<float, empty_t>(store)));
  EXPECT_TRUE((pi32.typed<std::tuple<int32_t>, empty_t>(store)));
  EXPECT_TRUE((pi32.typed<int32_t, empty_t>(store)));
  EXPECT_TRUE((pi32.typed<std::tuple<uint32_t>, empty_t>(store)));
  EXPECT_TRUE((pi32.typed<uint32_t, empty_t>(store)));

  Func rets(
      store, FuncType({}, {ValKind::F32, ValKind::F64}),
      [](auto caller, auto params, auto results) { return std::monostate(); });

  EXPECT_FALSE((rets.typed<empty_t, std::tuple<int, int>>(store)));
  EXPECT_FALSE((rets.typed<empty_t, empty_t>(store)));
  EXPECT_TRUE((rets.typed<empty_t, std::tuple<float, double>>(store)));
}

TEST(TypedFunc, Call) {
  Engine engine;
  Store store(engine);

  {
    Func thunk(store, FuncType({}, {}),
               [](auto caller, auto params, auto results) {
                 return std::monostate();
               });
    auto func = thunk.typed<empty_t, empty_t>(store).unwrap();
    empty_t result = func.call(store, empty_t()).unwrap();
  }

  {
    Func f(store, FuncType({ValKind::I32}, {}),
           [](auto caller, auto params, auto results) {
             EXPECT_EQ(params[0].i32(), 1);
             return std::monostate();
           });

    f.typed<int32_t, empty_t>(store).unwrap().call(store, 1).unwrap();
    f.typed<std::tuple<int32_t>, empty_t>(store)
        .unwrap()
        .call(store, {1})
        .unwrap();
  }
  {
    Func f(store,
           FuncType({ValKind::F32, ValKind::I64}, {ValKind::I32, ValKind::F64}),
           [](auto caller, auto params, auto results) {
             EXPECT_EQ(params[0].f32(), 1);
             EXPECT_EQ(params[1].i64(), 2);
             results[0] = int32_t(3);
             results[1] = double(4);
             return std::monostate();
           });

    auto func =
        f.typed<std::tuple<float, uint64_t>, std::tuple<uint32_t, double>>(
             store)
            .unwrap();
    auto result = func.call(store, {1, 2}).unwrap();
    EXPECT_EQ(std::get<0>(result), 3);
    EXPECT_EQ(std::get<1>(result), 4);
  }

  {
    FuncType ty({ValKind::ExternRef, ValKind::ExternRef},
                {ValKind::ExternRef, ValKind::ExternRef});
    Func f(store, ty, [](auto caller, auto params, auto results) {
      caller.context().gc();
      EXPECT_TRUE(params[0].externref(caller));
      EXPECT_EQ(std::any_cast<int>(params[0].externref(caller)->data(caller)),
                100);
      EXPECT_FALSE(params[1].externref(caller));
      results[0] = ExternRef(caller, int(3));
      results[1] = std::optional<ExternRef>(std::nullopt);
      caller.context().gc();
      return std::monostate();
    });

    using ExternRefPair =
        std::tuple<std::optional<ExternRef>, std::optional<ExternRef>>;
    auto func = f.typed<ExternRefPair, ExternRefPair>(store).unwrap();
    auto result =
        func.call(store, {ExternRef(store, int(100)), std::nullopt}).unwrap();
    store.context().gc();
    EXPECT_EQ(std::any_cast<int>(std::get<0>(result)->data(store)), 3);
    EXPECT_EQ(std::get<1>(result), std::nullopt);
  }

  {
    Func f2(store, FuncType({}, {}),
            [](auto caller, auto params, auto results) {
              return std::monostate();
            });

    FuncType ty({ValKind::FuncRef, ValKind::FuncRef},
                {ValKind::FuncRef, ValKind::FuncRef});

    Func f(store, ty, [&](auto caller, auto params, auto results) {
      EXPECT_TRUE(params[0].funcref());
      Func param = *params[0].funcref();
      param.typed<empty_t, empty_t>(caller)
          .unwrap()
          .call(caller, empty_t())
          .unwrap();
      EXPECT_FALSE(params[1].funcref());
      results[0] = f2;
      results[1] = std::optional<Func>(std::nullopt);
      return std::monostate();
    });

    using FuncPair = std::tuple<std::optional<Func>, std::optional<Func>>;
    auto func = f.typed<FuncPair, FuncPair>(store).unwrap();
    auto result = func.call(store, {f2, std::nullopt}).unwrap();
    /* EXPECT_EQ(std::any_cast<int>(std::get<0>(result)->data()), 3); */
    Func result_f = *std::get<0>(result);
    result_f.typed<empty_t, empty_t>(store)
        .unwrap()
        .call(store, empty_t())
        .unwrap();
    EXPECT_EQ(std::get<1>(result), std::nullopt);
  }

  {
    FuncType ty({ValKind::V128}, {ValKind::V128});

    Func f(store, ty, [&](auto caller, auto params, auto results) {
      V128 ret;
      for (int i = 0; i < 16; i++) {
        EXPECT_EQ(params[0].v128().v128[i], 1);
        ret.v128[i] = 2;
      }
      results[0] = ret;
      return std::monostate();
    });

    V128 param;
    for (int i = 0; i < 16; i++) {
      param.v128[i] = 1;
    }
    auto func = f.typed<V128, V128>(store).unwrap();
    auto result = func.call(store, {param}).unwrap();
    for (int i = 0; i < 16; i++) {
      EXPECT_EQ(result.v128[i], 2);
    }
  }
}

void assert_types_eq(ValType::ListRef actual,
                     std::initializer_list<ValKind> expected) {
  EXPECT_EQ(expected.size(), actual.size());
  std::vector<ValKind> actual_vec;
  for (auto ty : actual) {
    actual_vec.push_back(ty.kind());
  }
  std::vector<ValKind> expected_vec(expected);
  EXPECT_EQ(actual_vec, expected_vec);
}

void assert_func_type(FuncType actual, std::initializer_list<ValKind> params,
                      std::initializer_list<ValKind> results) {
  assert_types_eq(actual->params(), params);
  assert_types_eq(actual->results(), results);
}

TEST(TypedFunc, WrapAndTypes) {
  Engine engine;
  Store store(engine);
  Func f = Func::wrap(store, []() {});
  assert_func_type(f.type(store), {}, {});
  f = Func::wrap(store, []() { return int32_t(1); });
  assert_func_type(f.type(store), {}, {ValKind::I32});
  f = Func::wrap(store, []() { return int64_t(1); });
  assert_func_type(f.type(store), {}, {ValKind::I64});
  f = Func::wrap(store, []() { return float(1); });
  assert_func_type(f.type(store), {}, {ValKind::F32});
  f = Func::wrap(store, []() { return double(1); });
  assert_func_type(f.type(store), {}, {ValKind::F64});
  f = Func::wrap(store, []() { return V128(); });
  assert_func_type(f.type(store), {}, {ValKind::V128});
  f = Func::wrap(store,
                 []() { return std::make_tuple(int32_t(1), int32_t(2)); });
  assert_func_type(f.type(store), {}, {ValKind::I32, ValKind::I32});
  f = Func::wrap(store, []() { return std::optional<Func>(std::nullopt); });
  assert_func_type(f.type(store), {}, {ValKind::FuncRef});
  f = Func::wrap(store,
                 []() { return std::optional<ExternRef>(std::nullopt); });
  assert_func_type(f.type(store), {}, {ValKind::ExternRef});
  f = Func::wrap(
      store, []() { return Result<std::monostate, Trap>(std::monostate()); });
  assert_func_type(f.type(store), {}, {});
  f = Func::wrap(store, []() { return Result<int32_t, Trap>(1); });
  assert_func_type(f.type(store), {}, {ValKind::I32});
  f = Func::wrap(store, []() { return Result<float, Trap>(1); });
  assert_func_type(f.type(store), {}, {ValKind::F32});
  f = Func::wrap(store, []() {
    return Result<std::tuple<int32_t, int32_t>, Trap>({1, 2});
  });
  assert_func_type(f.type(store), {}, {ValKind::I32, ValKind::I32});

  f = Func::wrap(store, [](int32_t a) {});
  assert_func_type(f.type(store), {ValKind::I32}, {});
  f = Func::wrap(store, [](int64_t a) {});
  assert_func_type(f.type(store), {ValKind::I64}, {});
  f = Func::wrap(store, [](float a) {});
  assert_func_type(f.type(store), {ValKind::F32}, {});
  f = Func::wrap(store, [](double a) {});
  assert_func_type(f.type(store), {ValKind::F64}, {});
  f = Func::wrap(store, [](V128 a) {});
  assert_func_type(f.type(store), {ValKind::V128}, {});
  f = Func::wrap(store, [](std::optional<Func> a) {});
  assert_func_type(f.type(store), {ValKind::FuncRef}, {});
  f = Func::wrap(store, [](std::optional<ExternRef> a) {});
  assert_func_type(f.type(store), {ValKind::ExternRef}, {});
  f = Func::wrap(store, [](Caller a) {});
  assert_func_type(f.type(store), {}, {});
  f = Func::wrap(store, [](Caller a, int32_t b) {});
  assert_func_type(f.type(store), {ValKind::I32}, {});
}

TEST(TypedFunc, WrapRuntime) {
  Engine engine;
  Store store(engine);
  Func f = Func::wrap(store, []() {});
  f.typed<empty_t, empty_t>(store).unwrap().call(store, empty_t()).unwrap();

  f = Func::wrap(store, []() { return int32_t(1); });
  int32_t i =
      f.typed<empty_t, int32_t>(store).unwrap().call(store, empty_t()).unwrap();
  EXPECT_EQ(i, 1);

  f = Func::wrap(store, [](Caller cx, int32_t i) { EXPECT_EQ(i, 2); });
  f.typed<int32_t, empty_t>(store).unwrap().call(store, 2).unwrap();

  f = Func::wrap(store, [](Caller cx, int32_t i, int32_t j) { return i + j; });
  auto ret = f.typed<std::tuple<int32_t, int32_t>, int32_t>(store)
                 .unwrap()
                 .call(store, {1, 2})
                 .unwrap();
  EXPECT_EQ(ret, 3);
}
