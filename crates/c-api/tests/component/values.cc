#include "utils.h"

#include <gtest/gtest.h>
#include <wasmtime.h>

#include <array>
#include <format>
#include <optional>
#include <span>
#include <variant>

static std::string echo_component(std::string_view type, std::string_view func,
                                  std::string_view host_params) {
  return std::format(
      R"END(
(component
	(type $Foo' {})
	(import "foo" (type $Foo (eq $Foo')))
	(import "do" (func $do (param "a" $Foo) (result $Foo)))
	(core module $libc
		(memory (export "memory") 1)
		{}
	)
	(core instance $libc (instantiate $libc))
	(core func $do_lower (canon lower (func $do) (memory $libc "memory") (realloc (func $libc "realloc"))))

	(core module $doer
		(import "host" "do" (func $do {}))
		(import "libc" "memory" (memory 1))
		(import "libc" "realloc" (func $realloc (param i32 i32 i32 i32) (result i32)))

		(func (export "call")
			{})
	)
	(core instance $doer (instantiate $doer
		(with "host" (instance (export "do" (func $do_lower))))
		(with "libc" (instance $libc))
	))

	(func $call
		(param "a" $Foo)
		(result $Foo)
		(canon lift
			(core func $doer "call")
			(memory $libc "memory")
			(realloc (func $libc "realloc")))
	)

	(export "call" (func $call))
)
		  )END",
      type, REALLOC_AND_FREE, host_params, func);
}

struct Context {
  wasm_engine_t *engine;
  wasmtime_store_t *store;
  wasmtime_context_t *context;
  wasmtime_component_t *component;
  wasmtime_component_instance_t instance;
  wasmtime_component_func_t func;
};

static Context create(std::string_view type, std::string_view body,
                      std::string_view host_params,
                      wasmtime_component_func_callback_t callback) {
  auto component_text = echo_component(type, body, host_params);
  const auto engine = wasm_engine_new();
  EXPECT_NE(engine, nullptr);

  const auto store = wasmtime_store_new(engine, nullptr, nullptr);
  const auto context = wasmtime_store_context(store);

  wasmtime_component_t *component = nullptr;

  auto err = wasmtime_component_new(
      engine, reinterpret_cast<const uint8_t *>(component_text.data()),
      component_text.size(), &component);

  CHECK_ERR(err);

  auto f = wasmtime_component_get_export_index(component, nullptr, "call",
                                               strlen("call"));

  EXPECT_NE(f, nullptr);

  const auto linker = wasmtime_component_linker_new(engine);
  const auto root = wasmtime_component_linker_root(linker);

  wasmtime_component_linker_instance_add_func(root, "do", strlen("do"),
                                              callback, nullptr, nullptr);

  wasmtime_component_linker_instance_delete(root);

  wasmtime_component_instance_t instance = {};
  err = wasmtime_component_linker_instantiate(linker, context, component,
                                              &instance);
  CHECK_ERR(err);

  wasmtime_component_linker_delete(linker);

  wasmtime_component_func_t func = {};
  const auto found =
      wasmtime_component_instance_get_func(&instance, context, f, &func);
  EXPECT_TRUE(found);
  EXPECT_NE(func.store_id, 0);

  wasmtime_component_export_index_delete(f);

  return Context{
      .engine = engine,
      .store = store,
      .context = context,
      .component = component,
      .instance = instance,
      .func = func,
  };
}

static void destroy(Context &ctx) {
  wasmtime_component_delete(ctx.component);
  wasmtime_store_delete(ctx.store);
  wasm_engine_delete(ctx.engine);
}

TEST(component, value_record) {
  static const auto check = [](const wasmtime_component_val_t &val, uint64_t x,
                               uint64_t y) {
    EXPECT_EQ(val.kind, WASMTIME_COMPONENT_RECORD);

    EXPECT_EQ(val.of.record.size, 2);
    const auto entries = val.of.record.data;

    EXPECT_EQ((std::string_view{entries[0].name.data, entries[0].name.size}),
              "x");
    EXPECT_EQ(entries[0].val.kind, WASMTIME_COMPONENT_U64);
    EXPECT_EQ(entries[0].val.of.u64, x);

    EXPECT_EQ((std::string_view{entries[1].name.data, entries[1].name.size}),
              "y");
    EXPECT_EQ(entries[1].val.kind, WASMTIME_COMPONENT_U64);
    EXPECT_EQ(entries[1].val.of.u64, y);
  };

  static const auto make = [](uint64_t x,
                              uint64_t y) -> wasmtime_component_val_t {
    auto ret = wasmtime_component_val_t{
        .kind = WASMTIME_COMPONENT_RECORD,
    };

    wasmtime_component_valrecord_new_uninit(&ret.of.record, 2);

    const auto entries = ret.of.record.data;
    wasm_name_new_from_string(&entries[0].name, "x");
    entries[0].val.kind = WASMTIME_COMPONENT_U64;
    entries[0].val.of.u64 = x;
    wasm_name_new_from_string(&entries[1].name, "y");
    entries[1].val.kind = WASMTIME_COMPONENT_U64;
    entries[1].val.of.u64 = y;

    return ret;
  };

  auto ctx = create(
      R"((record (field "x" u64) (field "y" u64)))", R"(
(param $x i64)
(param $y i64)
(result i32)
(local $res i32)
local.get $x
local.get $y
(call $realloc
	(i32.const 0)
	(i32.const 0)
	(i32.const 4)
	(i32.const 16))
local.tee $res
call $do
local.get $res
	  )",
      "(param i64 i64 i32)",
      +[](void *, wasmtime_context_t *, const wasmtime_component_val_t *args,
          size_t args_len, wasmtime_component_val_t *rets,
          size_t rets_len) -> wasmtime_error_t * {
        EXPECT_EQ(args_len, 1);
        check(args[0], 1, 2);

        EXPECT_EQ(rets_len, 1);
        rets[0] = make(3, 4);

        return nullptr;
      });

  auto arg = make(1, 2);
  auto res = wasmtime_component_val_t{};

  auto err =
      wasmtime_component_func_call(&ctx.func, ctx.context, &arg, 1, &res, 1);
  CHECK_ERR(err);

  err = wasmtime_component_func_post_return(&ctx.func, ctx.context);
  CHECK_ERR(err);

  check(res, 3, 4);

  wasmtime_component_val_delete(&arg);
  wasmtime_component_val_delete(&res);

  destroy(ctx);
}

TEST(component, value_string) {
  static const auto check = [](const wasmtime_component_val_t &val,
                               std::string_view text) {
    EXPECT_EQ(val.kind, WASMTIME_COMPONENT_STRING);
    EXPECT_EQ((std::string_view{val.of.string.data, val.of.string.size}), text);
  };

  static const auto make =
      [](std::string_view text) -> wasmtime_component_val_t {
    auto str = wasm_name_t{};
    wasm_name_new_from_string(&str, text.data());

    return wasmtime_component_val_t{
        .kind = WASMTIME_COMPONENT_STRING,
        .of = {.string = str},
    };
  };

  auto ctx = create(
      R"(string)", R"(
(param $x i32)
(param $y i32)
(result i32)
(local $res i32)
local.get $x
local.get $y
(call $realloc
	(i32.const 0)
	(i32.const 0)
	(i32.const 4)
	(i32.const 8))
local.tee $res
call $do
local.get $res
	  )",
      "(param i32 i32 i32)",
      +[](void *, wasmtime_context_t *, const wasmtime_component_val_t *args,
          size_t args_len, wasmtime_component_val_t *rets,
          size_t rets_len) -> wasmtime_error_t * {
        EXPECT_EQ(args_len, 1);
        check(args[0], "hello from A!");

        EXPECT_EQ(rets_len, 1);
        rets[0] = make("hello from B!");

        return nullptr;
      });

  auto arg = make("hello from A!");
  auto res = wasmtime_component_val_t{};

  auto err =
      wasmtime_component_func_call(&ctx.func, ctx.context, &arg, 1, &res, 1);
  CHECK_ERR(err);

  err = wasmtime_component_func_post_return(&ctx.func, ctx.context);
  CHECK_ERR(err);

  check(res, "hello from B!");

  wasmtime_component_val_delete(&arg);
  wasmtime_component_val_delete(&res);

  destroy(ctx);
}

TEST(component, value_list) {
  static const auto check = [](const wasmtime_component_val_t &val,
                               std::vector<uint32_t> data) {
    EXPECT_EQ(val.kind, WASMTIME_COMPONENT_LIST);
    auto vals = std::span{val.of.list.data, val.of.list.size};
    EXPECT_EQ(vals.size(), data.size());
    for (auto i = 0; i < data.size(); i++) {
      EXPECT_EQ(vals[i].kind, WASMTIME_COMPONENT_U32);
      EXPECT_EQ(vals[i].of.u32, data[i]);
    }
  };

  static const auto make =
      [](std::vector<uint32_t> data) -> wasmtime_component_val_t {
    auto ret = wasmtime_component_val_t{
        .kind = WASMTIME_COMPONENT_LIST,
    };

    wasmtime_component_vallist_new_uninit(&ret.of.list, data.size());

    for (auto i = 0; i < data.size(); i++) {
      ret.of.list.data[i] = wasmtime_component_val_t{
          .kind = WASMTIME_COMPONENT_U32,
          .of = {.u32 = data[i]},
      };
    }

    return ret;
  };

  auto ctx = create(
      R"((list u32))", R"(
(param $x i32)
(param $y i32)
(result i32)
(local $res i32)
local.get $x
local.get $y
(call $realloc
	(i32.const 0)
	(i32.const 0)
	(i32.const 4)
	(i32.const 8))
local.tee $res
call $do
local.get $res
	  )",
      "(param i32 i32 i32)",
      +[](void *, wasmtime_context_t *, const wasmtime_component_val_t *args,
          size_t args_len, wasmtime_component_val_t *rets,
          size_t rets_len) -> wasmtime_error_t * {
        EXPECT_EQ(args_len, 1);
        check(args[0], {1, 2, 3});

        EXPECT_EQ(rets_len, 1);
        rets[0] = make({4, 5, 6, 7});

        return nullptr;
      });

  auto arg = make({1, 2, 3});
  auto res = wasmtime_component_val_t{};

  auto err =
      wasmtime_component_func_call(&ctx.func, ctx.context, &arg, 1, &res, 1);
  CHECK_ERR(err);

  err = wasmtime_component_func_post_return(&ctx.func, ctx.context);
  CHECK_ERR(err);

  check(res, {4, 5, 6, 7});

  wasmtime_component_val_delete(&arg);
  wasmtime_component_val_delete(&res);

  destroy(ctx);
}

TEST(component, value_tuple) {
  static const auto check = [](const wasmtime_component_val_t &val,
                               std::vector<uint32_t> data) {
    EXPECT_EQ(val.kind, WASMTIME_COMPONENT_TUPLE);
    auto vals = std::span{val.of.tuple.data, val.of.tuple.size};
    EXPECT_EQ(vals.size(), data.size());
    for (auto i = 0; i < data.size(); i++) {
      EXPECT_EQ(vals[i].kind, WASMTIME_COMPONENT_U32);
      EXPECT_EQ(vals[i].of.u32, data[i]);
    }
  };

  static const auto make =
      [](std::vector<uint32_t> data) -> wasmtime_component_val_t {
    auto ret = wasmtime_component_val_t{
        .kind = WASMTIME_COMPONENT_TUPLE,
    };

    wasmtime_component_valtuple_new_uninit(&ret.of.tuple, data.size());

    for (auto i = 0; i < data.size(); i++) {
      ret.of.list.data[i] = wasmtime_component_val_t{
          .kind = WASMTIME_COMPONENT_U32,
          .of = {.u32 = data[i]},
      };
    }

    return ret;
  };

  auto ctx = create(
      R"((tuple u32 u32 u32))", R"(
(param $x i32)
(param $y i32)
(param $z i32)
(result i32)
(local $res i32)
local.get $x
local.get $y
local.get $z
(call $realloc
	(i32.const 0)
	(i32.const 0)
	(i32.const 4)
	(i32.const 12))
local.tee $res
call $do
local.get $res
	  )",
      "(param i32 i32 i32 i32)",
      +[](void *, wasmtime_context_t *, const wasmtime_component_val_t *args,
          size_t args_len, wasmtime_component_val_t *rets,
          size_t rets_len) -> wasmtime_error_t * {
        EXPECT_EQ(args_len, 1);
        check(args[0], {1, 2, 3});

        EXPECT_EQ(rets_len, 1);
        rets[0] = make({4, 5, 6});

        return nullptr;
      });

  auto arg = make({1, 2, 3});
  auto res = wasmtime_component_val_t{};

  auto err =
      wasmtime_component_func_call(&ctx.func, ctx.context, &arg, 1, &res, 1);
  CHECK_ERR(err);

  err = wasmtime_component_func_post_return(&ctx.func, ctx.context);
  CHECK_ERR(err);

  check(res, {4, 5, 6});

  wasmtime_component_val_delete(&arg);
  wasmtime_component_val_delete(&res);

  destroy(ctx);
}

TEST(component, value_variant) {
  static const auto check_aa = [](const wasmtime_component_val_t &val,
                                  uint32_t value) {
    EXPECT_EQ(val.kind, WASMTIME_COMPONENT_VARIANT);
    EXPECT_EQ((std::string_view{val.of.variant.discriminant.data,
                                val.of.variant.discriminant.size}),
              "aa");

    EXPECT_NE(val.of.variant.val, nullptr);

    EXPECT_EQ(val.of.variant.val->kind, WASMTIME_COMPONENT_U32);
    EXPECT_EQ(val.of.variant.val->of.u32, value);
  };

  static const auto check_bb = [](const wasmtime_component_val_t &val,
                                  std::string_view value) {
    EXPECT_EQ(val.kind, WASMTIME_COMPONENT_VARIANT);
    EXPECT_EQ((std::string_view{val.of.variant.discriminant.data,
                                val.of.variant.discriminant.size}),
              "bb");

    EXPECT_NE(val.of.variant.val, nullptr);

    EXPECT_EQ(val.of.variant.val->kind, WASMTIME_COMPONENT_STRING);
    EXPECT_EQ((std::string_view{val.of.variant.val->of.string.data,
                                val.of.variant.val->of.string.size}),
              value);
  };

  static const auto make_aa = [](uint32_t value) -> wasmtime_component_val_t {
    auto ret = wasmtime_component_val_t{
        .kind = WASMTIME_COMPONENT_VARIANT,
    };

    wasm_name_new_from_string(&ret.of.variant.discriminant, "aa");

    ret.of.variant.val = wasmtime_component_val_new();
    ret.of.variant.val->kind = WASMTIME_COMPONENT_U32;
    ret.of.variant.val->of.u32 = value;

    return ret;
  };

  static const auto make_bb =
      [](std::string_view value) -> wasmtime_component_val_t {
    auto ret = wasmtime_component_val_t{
        .kind = WASMTIME_COMPONENT_VARIANT,
    };

    wasm_name_new_from_string(&ret.of.variant.discriminant, "bb");

    ret.of.variant.val = wasmtime_component_val_new();
    ret.of.variant.val->kind = WASMTIME_COMPONENT_STRING;
    wasm_name_new(&ret.of.variant.val->of.string, value.size(), value.data());

    return ret;
  };

  auto ctx = create(
      R"(
(variant
	(case "aa" u32)
	(case "bb" string)
)
	  )",
      R"(
(param $x i32)
(param $y i32)
(param $z i32)
(result i32)
(local $res i32)
local.get $x
local.get $y
local.get $z
(call $realloc
	(i32.const 0)
	(i32.const 0)
	(i32.const 4)
	(i32.const 12))
local.tee $res
call $do
local.get $res
	  )",
      "(param i32 i32 i32 i32)",
      +[](void *, wasmtime_context_t *, const wasmtime_component_val_t *args,
          size_t args_len, wasmtime_component_val_t *rets,
          size_t rets_len) -> wasmtime_error_t * {
        EXPECT_EQ(args_len, 1);
        check_aa(args[0], 123);

        EXPECT_EQ(rets_len, 1);
        rets[0] = make_bb("textt");

        return nullptr;
      });

  auto arg = make_aa(123);
  auto res = wasmtime_component_val_t{};

  auto err =
      wasmtime_component_func_call(&ctx.func, ctx.context, &arg, 1, &res, 1);
  CHECK_ERR(err);

  err = wasmtime_component_func_post_return(&ctx.func, ctx.context);
  CHECK_ERR(err);

  check_bb(res, "textt");

  wasmtime_component_val_delete(&arg);
  wasmtime_component_val_delete(&res);

  destroy(ctx);
}

TEST(component, value_enum) {
  static const auto check = [](const wasmtime_component_val_t &val,
                               std::string_view text) {
    EXPECT_EQ(val.kind, WASMTIME_COMPONENT_ENUM);
    EXPECT_EQ(
        (std::string_view{val.of.enumeration.data, val.of.enumeration.size}),
        text);
  };

  static const auto make =
      [](std::string_view text) -> wasmtime_component_val_t {
    auto ret = wasmtime_component_val_t{
        .kind = WASMTIME_COMPONENT_ENUM,
    };

    wasm_name_new(&ret.of.enumeration, text.size(), text.data());

    return ret;
  };

  auto ctx = create(
      R"((enum "aa" "bb"))", R"(
(param $x i32)
(result i32)
local.get $x
call $do
	  )",
      "(param i32) (result i32)",
      +[](void *, wasmtime_context_t *, const wasmtime_component_val_t *args,
          size_t args_len, wasmtime_component_val_t *rets,
          size_t rets_len) -> wasmtime_error_t * {
        EXPECT_EQ(args_len, 1);
        check(args[0], "aa");

        EXPECT_EQ(rets_len, 1);
        rets[0] = make("bb");

        return nullptr;
      });

  auto arg = make("aa");
  auto res = wasmtime_component_val_t{};

  auto err =
      wasmtime_component_func_call(&ctx.func, ctx.context, &arg, 1, &res, 1);
  CHECK_ERR(err);

  err = wasmtime_component_func_post_return(&ctx.func, ctx.context);
  CHECK_ERR(err);

  check(res, "bb");

  wasmtime_component_val_delete(&arg);
  wasmtime_component_val_delete(&res);

  destroy(ctx);
}

TEST(component, value_option) {
  static const auto check = [](const wasmtime_component_val_t &val,
                               std::optional<uint32_t> value) {
    EXPECT_EQ(val.kind, WASMTIME_COMPONENT_OPTION);

    if (value.has_value()) {
      EXPECT_NE(val.of.option, nullptr);
      EXPECT_EQ(val.of.option->kind, WASMTIME_COMPONENT_U32);
      EXPECT_EQ(val.of.option->of.u32, *value);
    } else {
      EXPECT_EQ(val.of.option, nullptr);
    }
  };

  static const auto make =
      [](std::optional<uint32_t> value) -> wasmtime_component_val_t {
    auto ret = wasmtime_component_val_t{
        .kind = WASMTIME_COMPONENT_OPTION,
        .of = {.option = nullptr},
    };

    if (value.has_value()) {
      ret.of.option = wasmtime_component_val_new();
      *ret.of.option = wasmtime_component_val_t{
          .kind = WASMTIME_COMPONENT_U32,
          .of = {.u32 = *value},
      };
    }

    return ret;
  };

  auto ctx = create(
      R"((option u32))", R"(
(param $x i32)
(param $y i32)
(result i32)
(local $res i32)
local.get $x
local.get $y
(call $realloc
	(i32.const 0)
	(i32.const 0)
	(i32.const 4)
	(i32.const 8))
local.tee $res
call $do
local.get $res
	  )",
      "(param i32 i32 i32)",
      +[](void *, wasmtime_context_t *, const wasmtime_component_val_t *args,
          size_t args_len, wasmtime_component_val_t *rets,
          size_t rets_len) -> wasmtime_error_t * {
        EXPECT_EQ(args_len, 1);
        check(args[0], 123);

        EXPECT_EQ(rets_len, 1);
        rets[0] = make({});

        return nullptr;
      });

  auto arg = make(123);
  auto res = wasmtime_component_val_t{};

  auto err =
      wasmtime_component_func_call(&ctx.func, ctx.context, &arg, 1, &res, 1);
  CHECK_ERR(err);

  err = wasmtime_component_func_post_return(&ctx.func, ctx.context);
  CHECK_ERR(err);

  check(res, {});

  wasmtime_component_val_delete(&arg);
  wasmtime_component_val_delete(&res);

  destroy(ctx);
}

TEST(component, value_result) {
  static const auto check = [](const wasmtime_component_val_t &val,
                               bool expected_is_ok, uint32_t expected_value) {
    EXPECT_EQ(val.kind, WASMTIME_COMPONENT_RESULT);

    EXPECT_EQ(val.of.result.is_ok, expected_is_ok);
    EXPECT_NE(val.of.result.val, nullptr);

    EXPECT_EQ(val.of.result.val->kind, WASMTIME_COMPONENT_U32);
    EXPECT_EQ(val.of.result.val->of.u32, expected_value);
  };

  static const auto make = [](bool is_ok,
                              uint32_t value) -> wasmtime_component_val_t {
    auto ret = wasmtime_component_val_t{
        .kind = WASMTIME_COMPONENT_RESULT,
    };

    const auto inner = wasmtime_component_val_new();
    inner->kind = WASMTIME_COMPONENT_U32;
    inner->of.u32 = value;

    ret.of.result = {
        .is_ok = is_ok,
        .val = inner,
    };

    return ret;
  };

  auto ctx = create(
      R"((result u32 (error u32)))", R"(
(param $x i32)
(param $y i32)
(result i32)
(local $res i32)
local.get $x
local.get $y
(call $realloc
	(i32.const 0)
	(i32.const 0)
	(i32.const 4)
	(i32.const 8))
local.tee $res
call $do
local.get $res
	  )",
      "(param i32 i32 i32)",
      +[](void *, wasmtime_context_t *, const wasmtime_component_val_t *args,
          size_t args_len, wasmtime_component_val_t *rets,
          size_t rets_len) -> wasmtime_error_t * {
        EXPECT_EQ(args_len, 1);
        check(args[0], true, 123);

        EXPECT_EQ(rets_len, 1);
        rets[0] = make(false, 456);

        return nullptr;
      });

  auto arg = make(true, 123);
  auto res = wasmtime_component_val_t{};

  auto err =
      wasmtime_component_func_call(&ctx.func, ctx.context, &arg, 1, &res, 1);
  CHECK_ERR(err);

  err = wasmtime_component_func_post_return(&ctx.func, ctx.context);
  CHECK_ERR(err);

  check(res, false, 456);

  wasmtime_component_val_delete(&arg);
  wasmtime_component_val_delete(&res);

  destroy(ctx);
}

TEST(component, value_flags) {
  static const auto check = [](const wasmtime_component_val_t &val,
                               std::vector<std::string> data) {
    EXPECT_EQ(val.kind, WASMTIME_COMPONENT_FLAGS);
    auto flags = std::span{val.of.flags.data, val.of.flags.size};
    EXPECT_EQ(flags.size(), data.size());
    for (auto i = 0; i < data.size(); i++) {
      EXPECT_EQ((std::string_view{flags[i].data, flags[i].size}), data[i]);
    }
  };

  static const auto make =
      [](std::vector<std::string> data) -> wasmtime_component_val_t {
    auto ret = wasmtime_component_val_t{
        .kind = WASMTIME_COMPONENT_FLAGS,
    };

    wasmtime_component_valflags_new_uninit(&ret.of.flags, data.size());

    for (auto i = 0; i < data.size(); i++) {
      wasm_name_new(&ret.of.flags.data[i], data[i].size(), data[i].data());
    }

    return ret;
  };

  auto ctx = create(
      R"((flags "aa" "bb"))", R"(
(param $x i32)
(result i32)
local.get $x
call $do
	  )",
      "(param i32) (result i32)",
      +[](void *, wasmtime_context_t *, const wasmtime_component_val_t *args,
          size_t args_len, wasmtime_component_val_t *rets,
          size_t rets_len) -> wasmtime_error_t * {
        EXPECT_EQ(args_len, 1);
        check(args[0], {"aa"});

        EXPECT_EQ(rets_len, 1);
        rets[0] = make({"aa", "bb"});

        return nullptr;
      });

  auto arg = make({"aa"});
  auto res = wasmtime_component_val_t{};

  auto err =
      wasmtime_component_func_call(&ctx.func, ctx.context, &arg, 1, &res, 1);
  CHECK_ERR(err);

  err = wasmtime_component_func_post_return(&ctx.func, ctx.context);
  CHECK_ERR(err);

  check(res, {"aa", "bb"});

  wasmtime_component_val_delete(&arg);
  wasmtime_component_val_delete(&res);

  destroy(ctx);
}

TEST(component, value_list_inner) {
  {
    auto x = wasmtime_component_val_t{
        .kind = WASMTIME_COMPONENT_LIST,
    };
    wasmtime_component_vallist_new_empty(&x.of.list);
    EXPECT_EQ(x.of.list.data, nullptr);
    EXPECT_EQ(x.of.list.size, 0);

    wasmtime_component_vallist_new_uninit(&x.of.list, 1);
    EXPECT_NE(x.of.list.data, nullptr);
    EXPECT_EQ(x.of.list.size, 1);

    wasmtime_component_vallist_delete(&x.of.list);

    auto items = std::array{
        wasmtime_component_val_t{
            .kind = WASMTIME_COMPONENT_U32,
            .of = {.u32 = 123},
        },
    };

    wasmtime_component_vallist_new(&x.of.list, items.size(), items.data());
    EXPECT_NE(x.of.list.data, nullptr);
    EXPECT_EQ(x.of.list.size, 1);

    EXPECT_EQ(x.of.list.data[0].kind, WASMTIME_COMPONENT_U32);
    EXPECT_EQ(x.of.list.data[0].of.u32, 123);

    auto clone = wasmtime_component_val_t{
        .kind = WASMTIME_COMPONENT_LIST,
    };

    wasmtime_component_vallist_copy(&clone.of.list, &x.of.list);
    wasmtime_component_vallist_delete(&x.of.list);

    EXPECT_NE(clone.of.list.data, nullptr);
    EXPECT_EQ(clone.of.list.size, 1);

    EXPECT_EQ(clone.of.list.data[0].kind, WASMTIME_COMPONENT_U32);
    EXPECT_EQ(clone.of.list.data[0].of.u32, 123);

    wasmtime_component_vallist_delete(&clone.of.list);
  }
}
