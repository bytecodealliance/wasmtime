#include <gtest/gtest.h>
#include <wasmtime.hh>

using namespace wasmtime;

static Engine make_engine() {
  Config config;
  config.wasm_exceptions(true);
  return Engine(std::move(config));
}

TEST(Exception, ConstructAndExamine) {
  auto engine = make_engine();
  Store store(engine);
  auto cx = store.context();

  // Create a tag type with (i32, i64) payload.
  FuncType ft({ValKind::I32, ValKind::I64}, {});
  TagType tt(ft);

  // Create a tag instance.
  auto tag = Tag::create(cx, tt).unwrap();

  // Create an exception with payload (42, 100).
  std::vector<Val> fields = {Val(int32_t(42)), Val(int64_t(100))};
  auto exn = ExnRef::create(cx, tag, fields).unwrap();

  // Read back the tag and verify identity.
  auto exn_tag = exn.tag(cx).unwrap();
  EXPECT_TRUE(tag.eq(cx, exn_tag));

  // Read field count.
  EXPECT_EQ(exn.field_count(cx), 2u);

  // Read fields.
  auto f0 = exn.field(cx, 0).unwrap();
  EXPECT_EQ(f0.kind(), ValKind::I32);
  EXPECT_EQ(f0.i32(), 42);

  auto f1 = exn.field(cx, 1).unwrap();
  EXPECT_EQ(f1.kind(), ValKind::I64);
  EXPECT_EQ(f1.i64(), 100);
}

TEST(Exception, TagFromModule) {
  auto engine = make_engine();
  Store store(engine);
  auto cx = store.context();

  Module module = Module::compile(engine, "(module"
                                          "  (tag $t (param i32))"
                                          "  (export \"t\" (tag $t))"
                                          ")")
                      .unwrap();

  auto instance = Instance::create(cx, module, {}).unwrap();
  auto ext = instance.get(cx, "t");
  ASSERT_TRUE(ext.has_value());
  auto tag = std::get<Tag>(*ext);

  auto tt = tag.type(cx);
  auto func = tt->functype();
  EXPECT_EQ(func->params().size(), 1u);
  EXPECT_EQ(func->params().begin()->kind(), ValKind::I32);
}

TEST(Exception, HostThrowWasmCatch) {
  auto engine = make_engine();
  Store store(engine);
  auto cx = store.context();

  FuncType tag_ft({ValKind::I32}, {});
  TagType tt(tag_ft);
  auto tag = Tag::create(cx, tt).unwrap();

  Module module =
      Module::compile(engine, "(module"
                              "  (import \"host\" \"throw\" (func $throw))"
                              "  (import \"host\" \"tag\" (tag $t (param i32)))"
                              "  (func (export \"run\") (result i32)"
                              "    (block $done (result i32)"
                              "      (try_table (catch 0 $done)"
                              "        (call $throw)"
                              "      )"
                              "      (unreachable)"
                              "    )"
                              "  )"
                              ")")
          .unwrap();

  FuncType throw_ft({}, {});
  Func throw_fn(cx, throw_ft,
                [&](Caller caller, Span<const Val> args,
                    Span<Val> results) -> Result<std::monostate, Trap> {
                  (void)args;
                  (void)results;
                  auto cx2 = caller.context();

                  std::vector<Val> fields = {Val(int32_t(99))};
                  auto exn = ExnRef::create(cx2, tag, fields).unwrap();
                  return cx2.throw_exception(exn);
                });

  std::vector<Extern> imports = {throw_fn, tag};
  auto instance = Instance::create(cx, module, imports).unwrap();

  auto run_ext = instance.get(cx, "run");
  ASSERT_TRUE(run_ext.has_value());
  auto run_fn = std::get<Func>(*run_ext);
  auto result = run_fn.call(cx, {});
  ASSERT_TRUE(result);
  ASSERT_EQ(result.ok().size(), 1u);
  EXPECT_EQ(result.ok()[0].i32(), 99);
}

TEST(Exception, ExnRefRoundTripThroughVal) {
  auto engine = make_engine();
  Store store(engine);
  auto cx = store.context();

  // A module that throws an exception and catches it as an exnref.
  Module module =
      Module::compile(engine, "(module"
                              "  (tag $t (param i32))"
                              "  (export \"tag\" (tag $t))"
                              "  (func (export \"make_exnref\") (result exnref)"
                              "    (block $done (result exnref)"
                              "      (try_table (catch_all_ref $done)"
                              "        (throw $t (i32.const 55))"
                              "      )"
                              "      (unreachable)"
                              "    )"
                              "  )"
                              ")")
          .unwrap();

  auto instance = Instance::create(cx, module, {}).unwrap();

  auto tag_ext = instance.get(cx, "tag");
  ASSERT_TRUE(tag_ext.has_value());
  auto tag = std::get<Tag>(*tag_ext);

  auto make_fn = std::get<Func>(*instance.get(cx, "make_exnref"));
  auto result = make_fn.call(cx, {});
  ASSERT_TRUE(result);
  ASSERT_EQ(result.ok().size(), 1u);

  // The returned value should be an exnref.
  auto &exnref_val = result.ok()[0];
  EXPECT_EQ(exnref_val.kind(), ValKind::ExnRef);

  // Pass the exnref back into a wasm function that extracts the i32 payload.
  Module module2 =
      Module::compile(engine,
                      "(module"
                      "  (import \"host\" \"tag\" (tag $t (param i32)))"
                      "  (func (export \"read\") (param exnref) (result i32)"
                      "    (block $done (result i32)"
                      "      (try_table (catch 0 $done)"
                      "        (throw_ref (local.get 0))"
                      "      )"
                      "      (unreachable)"
                      "    )"
                      "  )"
                      ")")
          .unwrap();

  std::vector<Extern> imports2 = {tag};
  auto instance2 = Instance::create(cx, module2, imports2).unwrap();
  auto read_fn = std::get<Func>(*instance2.get(cx, "read"));

  // Call with the exnref we got back from the first module.
  std::vector<Val> args = {std::move(exnref_val)};
  auto result2 = read_fn.call(cx, args);
  ASSERT_TRUE(result2);
  ASSERT_EQ(result2.ok().size(), 1u);
  EXPECT_EQ(result2.ok()[0].i32(), 55);
}

TEST(Exception, WasmThrowHostCatch) {
  auto engine = make_engine();
  Store store(engine);
  auto cx = store.context();

  Module module = Module::compile(engine, "(module"
                                          "  (tag $t (param i32))"
                                          "  (export \"tag\" (tag $t))"
                                          "  (func (export \"throw\")"
                                          "    (throw $t (i32.const 77))"
                                          "  )"
                                          ")")
                      .unwrap();

  auto instance = Instance::create(cx, module, {}).unwrap();

  auto tag_ext = instance.get(cx, "tag");
  ASSERT_TRUE(tag_ext.has_value());
  auto tag = std::get<Tag>(*tag_ext);

  auto throw_ext = instance.get(cx, "throw");
  ASSERT_TRUE(throw_ext.has_value());
  auto throw_fn = std::get<Func>(*throw_ext);

  auto result = throw_fn.call(cx, {});
  ASSERT_FALSE(result);

  ASSERT_TRUE(cx.has_exception());
  auto exn = cx.take_exception();
  ASSERT_TRUE(exn.has_value());

  auto exn_tag = exn->tag(cx).unwrap();
  EXPECT_TRUE(tag.eq(cx, exn_tag));

  EXPECT_EQ(exn->field_count(cx), 1u);
  auto f0 = exn->field(cx, 0).unwrap();
  EXPECT_EQ(f0.i32(), 77);
}
