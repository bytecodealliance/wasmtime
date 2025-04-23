#include <gtest/gtest.h>
#include <wasmtime.hh>

using namespace wasmtime;

template <typename T, typename E> T unwrap(Result<T, E> result) {
  if (result) {
    return result.ok();
  }
  std::cerr << "error: " << result.err().message() << "\n";
  std::abort();
}

TEST(Engine, Smoke) {
  Engine engine;
  Config config;
  engine = Engine(std::move(config));
}

TEST(wat2wasm, Smoke) {
  wat2wasm("(module)").ok();
  wat2wasm("xxx").err();
}

TEST(Module, Smoke) {
  Engine engine;
  Module::compile(engine, "(module)").ok();
  Module::compile(engine, "wat").err();

  auto wasm = wat2wasm("(module)").ok();
  Module::compile(engine, wasm).ok();
  std::vector<uint8_t> emptyWasm;
  Module::compile(engine, emptyWasm).err();

  Module::validate(engine, wasm).ok();
  Module::validate(engine, emptyWasm).err();

  Module m2 = unwrap(Module::compile(engine, "(module)"));
  Module m3 = m2;
  Module m4(m3);
  m4 = m2;
  Module m5(std::move(m3));
  m4 = std::move(m5);
}

TEST(Module, Serialize) {
  Engine engine;
  Module m = unwrap(Module::compile(engine, "(module)"));
  auto bytes = unwrap(m.serialize());
  m = unwrap(Module::deserialize(engine, bytes));
  std::string path("test_deserialize_file.cwasm");
  auto fh = ::fopen(path.c_str(), "wb");
  ::fwrite(bytes.data(), sizeof(uint8_t), bytes.size(), fh);
  ::fclose(fh);
  m = unwrap(Module::deserialize_file(engine, path));
  ::remove(path.c_str());
}

TEST(ExternRef, Smoke) {
  Engine engine;
  Store store(engine);
  ExternRef a(store, "foo");
  ExternRef b(store, 3);
  EXPECT_STREQ(std::any_cast<const char *>(a.data(store)), "foo");
  EXPECT_EQ(std::any_cast<int>(b.data(store)), 3);
  a.unroot(store);
  a = b;
}

TEST(Global, Smoke) {
  Engine engine;
  Store store(engine);
  Global::create(store, GlobalType(ValKind::I32, true), 3.0).err();
  unwrap(Global::create(store, GlobalType(ValKind::I32, true), 3));
  unwrap(Global::create(store, GlobalType(ValKind::I32, false), 3));

  Global g = unwrap(Global::create(store, GlobalType(ValKind::I32, true), 4));
  EXPECT_EQ(g.get(store).i32(), 4);
  unwrap(g.set(store, 10));
  EXPECT_EQ(g.get(store).i32(), 10);
  g.set(store, 10.23).err();
  EXPECT_EQ(g.get(store).i32(), 10);

  EXPECT_EQ(g.type(store)->content().kind(), ValKind::I32);
  EXPECT_TRUE(g.type(store)->is_mutable());
}

TEST(Table, Smoke) {
  Engine engine;
  Store store(engine);
  Table::create(store, TableType(ValKind::FuncRef, 1), 3.0).err();

  Val null = std::optional<Func>();
  Table t = unwrap(Table::create(store, TableType(ValKind::FuncRef, 1), null));
  EXPECT_FALSE(t.get(store, 1));
  EXPECT_TRUE(t.get(store, 0));
  Val val = *t.get(store, 0);
  EXPECT_EQ(val.kind(), ValKind::FuncRef);
  EXPECT_FALSE(val.funcref());
  EXPECT_EQ(unwrap(t.grow(store, 4, null)), 1);
  unwrap(t.set(store, 3, null));
  t.set(store, 3, 3).err();
  EXPECT_EQ(t.size(store), 5);
  EXPECT_EQ(t.type(store)->element().kind(), ValKind::FuncRef);
}

TEST(Memory, Smoke) {
  Engine engine;
  Store store(engine);
  Memory m = unwrap(Memory::create(store, MemoryType(1)));
  EXPECT_EQ(m.size(store), 1);
  EXPECT_EQ(unwrap(m.grow(store, 1)), 1);
  EXPECT_EQ(m.data(store).size(), 2 << 16);
  EXPECT_EQ(m.type(store)->min(), 1);
}

TEST(Instance, Smoke) {
  Engine engine;
  Store store(engine);
  Memory m = unwrap(Memory::create(store, MemoryType(1)));
  Global g = unwrap(Global::create(store, GlobalType(ValKind::I32, false), 1));
  Table t = unwrap(Table::create(store, TableType(ValKind::FuncRef, 1),
                                 std::optional<Func>()));
  Func f(
      store, FuncType({}, {}),
      [](auto caller, auto params, auto results) -> auto{
        return std::monostate();
      });

  Module mod =
      unwrap(Module::compile(engine, "(module"
                                     "(import \"\" \"\" (func))"
                                     "(import \"\" \"\" (global i32))"
                                     "(import \"\" \"\" (table 1 funcref))"
                                     "(import \"\" \"\" (memory 1))"

                                     "(func (export \"f\"))"
                                     "(global (export \"g\") i32 (i32.const 0))"
                                     "(export \"m\" (memory 0))"
                                     "(export \"t\" (table 0))"
                                     ")"));
  Instance::create(store, mod, {}).err();
  Instance i = unwrap(Instance::create(store, mod, {f, g, t, m}));
  EXPECT_FALSE(i.get(store, "not-present"));
  f = std::get<Func>(*i.get(store, "f"));
  m = std::get<Memory>(*i.get(store, "m"));
  t = std::get<Table>(*i.get(store, "t"));
  g = std::get<Global>(*i.get(store, "g"));

  EXPECT_TRUE(i.get(store, 0));
  EXPECT_TRUE(i.get(store, 1));
  EXPECT_TRUE(i.get(store, 2));
  EXPECT_TRUE(i.get(store, 3));
  EXPECT_FALSE(i.get(store, 4));
  auto [name, func] = *i.get(store, 0);
  EXPECT_EQ(name, "f");
}

TEST(Linker, Smoke) {
  Engine engine;
  Linker linker(engine);
  Store store(engine);
  linker.allow_shadowing(false);
  Global g = unwrap(Global::create(store, GlobalType(ValKind::I32, false), 1));
  unwrap(linker.define(store, "a", "g", g));
  unwrap(linker.define_wasi());
  unwrap(linker.func_new(
      "a", "f", FuncType({}, {}),
      [](auto caller, auto params, auto results) -> auto{
        return std::monostate();
      }));
  unwrap(linker.func_wrap("a", "f2", []() {}));
  unwrap(linker.func_wrap("a", "f3", [](Caller arg) {}));
  unwrap(linker.func_wrap("a", "f4", [](Caller arg, int32_t a) {}));
  Module mod = unwrap(Module::compile(engine, "(module)"));
  Instance i = unwrap(Instance::create(store, mod, {}));
  unwrap(linker.define_instance(store, "x", i));
  unwrap(linker.instantiate(store, mod));
  unwrap(linker.module(store, "y", mod));
  EXPECT_TRUE(linker.get(store, "a", "g"));
  unwrap(linker.get_default(store, "g"));
  EXPECT_TRUE(linker.get(store, "a", "f"));
  EXPECT_TRUE(std::holds_alternative<Func>(*linker.get(store, "a", "f")));
}

TEST(Linker, CallableMove) {
  Engine engine;
  Linker linker(engine);
  Store store(engine);
  linker.allow_shadowing(false);

  struct CallableFunc {
    CallableFunc() = default;
    CallableFunc(const CallableFunc&) = delete;
    CallableFunc(CallableFunc&&) = default;

    Result<std::monostate, Trap> operator()(Caller caller, Span<const Val> params, Span<Val> results) {
      return std::monostate();
    }
  };

  CallableFunc cf;
  unwrap(linker.func_new("a", "f", FuncType({}, {}), std::move(cf)));
}

TEST(Linker, CallableCopy) {
  Engine engine;
  Linker linker(engine);
  Store store(engine);
  linker.allow_shadowing(false);

  struct CallableFunc {
    CallableFunc() = default;
    CallableFunc(const CallableFunc&) = default;
    CallableFunc(CallableFunc&&) = default;

    Result<std::monostate, Trap> operator()(Caller caller, Span<const Val> params, Span<Val> results) {
      return std::monostate();
    }
  };

  CallableFunc cf;
  unwrap(linker.func_new("a", "f", FuncType({}, {}), cf));
}

TEST(Caller, Smoke) {
  Engine engine;
  Store store(engine);
  Func f(
      store, FuncType({}, {}),
      [](auto caller, auto params, auto results) -> auto{
        EXPECT_FALSE(caller.get_export("foo"));
        return std::monostate();
      });
  unwrap(f.call(store, {}));

  Module m = unwrap(Module::compile(engine, "(module "
                                            "(import \"\" \"\" (func))"
                                            "(memory (export \"m\") 1)"
                                            "(func (export \"f\") call 0)"
                                            ")"));
  Func f2(
      store, FuncType({}, {}),
      [](auto caller, auto params, auto results) -> auto{
        EXPECT_FALSE(caller.get_export("foo"));
        EXPECT_TRUE(caller.get_export("m"));
        EXPECT_TRUE(caller.get_export("f"));
        Memory m = std::get<Memory>(*caller.get_export("m"));
        EXPECT_EQ(m.type(caller)->min(), 1);
        return std::monostate();
      });
  Instance i = unwrap(Instance::create(store, m, {f2}));
  f = std::get<Func>(*i.get(store, "f"));
  unwrap(f.call(store, {}));
}

TEST(Func, Smoke) {
  Engine engine;
  Store store(engine);
  Func f(
      store, FuncType({}, {}),
      [](auto caller, auto params, auto results) -> auto{
        return std::monostate();
      });
  unwrap(f.call(store, {}));

  Func f2(
      store, FuncType({}, {}),
      [](auto caller, auto params, auto results) -> auto{
        return Trap("message");
      });
  EXPECT_EQ(f2.call(store, {}).err().message(), "message");
}

TEST(Data, Smoke) {

  Engine engine;
  Store store(engine);
  store.context().set_data(10);
  Func f0(store, FuncType({}, {}),
          [](auto caller, auto params,
             auto results) -> Result<std::monostate, Trap> {
            auto data = std::any_cast<int>(caller.context().get_data());
            if (data != 10) {
              return Trap("message");
            }
            return std::monostate();
          });
  unwrap(f0.call(store, {}));

  store.context().set_data(std::make_pair<int, int>(10, -3));
  Func f1(store, FuncType({}, {}),
          [](auto caller, auto params,
             auto results) -> Result<std::monostate, Trap> {
            auto data =
                std::any_cast<std::pair<int, int>>(caller.context().get_data());
            if (data.first != 10 || data.second != -3) {
              return Trap("message");
            }
            return std::monostate();
          });
  unwrap(f1.call(store, {}));

  store.context().set_data(std::string("hello world"));
  Func f2(store, FuncType({}, {}),
          [](auto caller, auto params,
             auto results) -> Result<std::monostate, Trap> {
            auto data = std::any_cast<std::string>(caller.context().get_data());
            if (data != "hello world") {
              return Trap("message");
            }
            return std::monostate();
          });
  unwrap(f2.call(store, {}));

  struct test_object {
    test_object() : v(nullptr) {}
    test_object(int i) : v(new int(i)) {}
    test_object(const test_object &other)
        : v((other.v) ? new int(*other.v) : nullptr) {}
    test_object(test_object &&other) : v(other.v) { other.v = nullptr; }
    ~test_object() {
      if (v) {
        delete v;
        v = nullptr;
      }
    }
    int *v;
  };

  test_object data(7);
  store.context().set_data(&data); // by pointer
  Func f3(store, FuncType({}, {}),
          [](auto caller, auto params,
             auto results) -> Result<std::monostate, Trap> {
            auto data =
                std::any_cast<test_object *>(caller.context().get_data());
            if (*data->v != 7) {
              return Trap("message");
            }
            return std::monostate();
          });
  unwrap(f3.call(store, {}));
  EXPECT_EQ(*data.v, 7);

  store.context().set_data(data); // by copy
  Func f4(store, FuncType({}, {}),
          [](auto caller, auto params,
             auto results) -> Result<std::monostate, Trap> {
            auto data =
                std::any_cast<test_object &>(caller.context().get_data());
            if (*data.v != 7) {
              return Trap("message");
            }
            return std::monostate();
          });
  unwrap(f4.call(store, {}));
  EXPECT_EQ(*data.v, 7);

  store.context().set_data(std::move(data)); // by move
  Func f5(store, FuncType({}, {}),
          [](auto caller, auto params,
             auto results) -> Result<std::monostate, Trap> {
            auto data =
                std::any_cast<test_object &>(caller.context().get_data());
            if (*data.v != 7) {
              return Trap("message");
            }
            return std::monostate();
          });
  unwrap(f5.call(store, {}));
  EXPECT_EQ(data.v, nullptr);
}
