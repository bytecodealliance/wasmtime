#include <wasmtime/val.hh>

#include <gtest/gtest.h>
#include <wasmtime.hh>

using namespace wasmtime;

TEST(Val, Smoke) {
  Val val(1);
  EXPECT_EQ(val.kind(), ValKind::I32);
  EXPECT_EQ(val.i32(), 1);

  val = (int32_t)3;
  EXPECT_EQ(val.kind(), ValKind::I32);
  EXPECT_EQ(val.i32(), 3);

  val = (int64_t)4;
  EXPECT_EQ(val.kind(), ValKind::I64);
  EXPECT_EQ(val.i64(), 4);

  val = (float)5;
  EXPECT_EQ(val.kind(), ValKind::F32);
  EXPECT_EQ(val.f32(), 5);

  val = (double)6;
  EXPECT_EQ(val.kind(), ValKind::F64);
  EXPECT_EQ(val.f64(), 6);

  val = V128();
  EXPECT_EQ(val.kind(), ValKind::V128);
  for (int i = 0; i < 16; i++) {
    EXPECT_EQ(val.v128().v128[i], 0);
  }

  Engine engine;
  Store store(engine);
  val = std::optional<ExternRef>(std::nullopt);
  EXPECT_EQ(val.kind(), ValKind::ExternRef);
  EXPECT_EQ(val.externref(), std::nullopt);

  val = std::optional<ExternRef>(ExternRef(store, 5));
  EXPECT_EQ(val.kind(), ValKind::ExternRef);
  EXPECT_EQ(std::any_cast<int>(val.externref()->data(store)), 5);

  val = ExternRef(store, 5);
  EXPECT_EQ(val.kind(), ValKind::ExternRef);
  EXPECT_EQ(std::any_cast<int>(val.externref()->data(store)), 5);

  val = std::optional<AnyRef>(std::nullopt);
  EXPECT_EQ(val.kind(), ValKind::AnyRef);
  EXPECT_EQ(val.anyref(), std::nullopt);

  val = std::optional<AnyRef>(AnyRef::i31(store, 5));
  EXPECT_EQ(val.kind(), ValKind::AnyRef);
  EXPECT_EQ(val.anyref()->i31(store), 5);
  EXPECT_EQ(val.anyref()->u31(store), 5);

  val = AnyRef::i31(store, -5);
  EXPECT_EQ(val.kind(), ValKind::AnyRef);
  EXPECT_EQ(val.anyref()->i31(store), -5);
  EXPECT_EQ(val.anyref()->u31(store), 0x7ffffffb);

  val = std::optional<Func>(std::nullopt);
  EXPECT_EQ(val.kind(), ValKind::FuncRef);
  EXPECT_EQ(val.funcref(), std::nullopt);

  Func func(store, FuncType({}, {}),
            [](auto caller, auto params, auto results) -> auto {
              return std::monostate();
            });

  val = std::optional<Func>(func);
  EXPECT_EQ(val.kind(), ValKind::FuncRef);

  val = func;
  EXPECT_EQ(val.kind(), ValKind::FuncRef);
}

class SetOnDrop {
  std::shared_ptr<std::atomic<bool>> flag_;

public:
  SetOnDrop() : flag_(std::make_shared<std::atomic<bool>>(false)) {}
  SetOnDrop(const SetOnDrop &) = delete;
  SetOnDrop(SetOnDrop &&obj) : flag_(obj.flag_) { obj.flag_.reset(); }
  ~SetOnDrop() {
    if (flag_)
      flag_->store(true);
  }

  const std::shared_ptr<std::atomic<bool>> &flag() { return this->flag_; }
};

TEST(Val, DropsExternRef) {
  std::shared_ptr<std::atomic<bool>> flag;
  Engine engine;
  Store store(engine);

  // smoke test for `SetOnDrop` itself
  {
    SetOnDrop guard;
    flag = guard.flag();
    EXPECT_FALSE(flag->load());
  }
  EXPECT_TRUE(flag->load());

  // Test that if an `ExternRef` is created and dropped it doesn't leak.
  {
    SetOnDrop guard;
    flag = guard.flag();
    ExternRef r(store, std::make_shared<SetOnDrop>(std::move(guard)));
    EXPECT_FALSE(flag->load());
    store.gc();
    EXPECT_FALSE(flag->load());
  }
  EXPECT_FALSE(flag->load());
  store.gc();
  EXPECT_TRUE(flag->load());

  // Test that if a `Val(ExternRef)` is created and dropped it doesn't leak.
  {
    SetOnDrop guard;
    flag = guard.flag();
    ExternRef r(store, std::make_shared<SetOnDrop>(std::move(guard)));
    Val v(r);
    EXPECT_FALSE(flag->load());
    store.gc();
    EXPECT_FALSE(flag->load());
  }
  EXPECT_FALSE(flag->load());
  store.gc();
  EXPECT_TRUE(flag->load());

  // Similar to above testing a variety of APIs.
  {
    SetOnDrop guard;
    flag = guard.flag();
    ExternRef r(store, std::make_shared<SetOnDrop>(std::move(guard)));
    ExternRef r2 = r;
    ExternRef r3(r2);
    r3 = r2;
    r = std::move(r2);

    Val v(r3);
    Val v2 = v;
    Val v3(v2);
    v3 = v2;
    v = std::move(v2);

    store.gc();
    EXPECT_FALSE(flag->load());
  }
  EXPECT_FALSE(flag->load());
  store.gc();
  EXPECT_TRUE(flag->load());
}
