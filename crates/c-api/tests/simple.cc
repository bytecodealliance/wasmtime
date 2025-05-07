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

TEST(Caller, Smoke) {
  Engine engine;
  Store store(engine);
  Func f(store, FuncType({}, {}),
         [](auto caller, auto params, auto results) -> auto {
           EXPECT_FALSE(caller.get_export("foo"));
           return std::monostate();
         });
  unwrap(f.call(store, {}));

  Module m = unwrap(Module::compile(engine, "(module "
                                            "(import \"\" \"\" (func))"
                                            "(memory (export \"m\") 1)"
                                            "(func (export \"f\") call 0)"
                                            ")"));
  Func f2(store, FuncType({}, {}),
          [](auto caller, auto params, auto results) -> auto {
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
  Func f(store, FuncType({}, {}),
         [](auto caller, auto params, auto results) -> auto {
           return std::monostate();
         });
  unwrap(f.call(store, {}));

  Func f2(store, FuncType({}, {}),
          [](auto caller, auto params, auto results) -> auto {
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
