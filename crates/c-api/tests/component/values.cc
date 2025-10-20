#include "utils.h"

#include <gtest/gtest.h>
#include <wasmtime.h>
#include <wasmtime/component.hh>
#include <wasmtime/store.hh>

#include <array>
#include <format>
#include <optional>
#include <span>
#include <variant>

using namespace wasmtime::component;
using wasmtime::Engine;
using wasmtime::Result;
using wasmtime::Span;
using wasmtime::Store;

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
  Engine engine;
  Store store;
  Store::Context context;
  Component component;
  Instance instance;
  Func func;
};

typedef Result<std::monostate> (*host_func_t)(Store::Context, Span<const Val>,
                                              Span<Val>);

static Context create(std::string_view type, std::string_view body,
                      std::string_view host_params, host_func_t callback) {
  auto component_text = echo_component(type, body, host_params);
  Engine engine;
  Store store(engine);
  const auto context = store.context();
  Component component = Component::compile(engine, component_text).unwrap();

  auto f = component.export_index(nullptr, "call");

  EXPECT_TRUE(f);

  Linker linker(engine);
  linker.root().add_func("do", callback).unwrap();

  auto instance = linker.instantiate(context, component).unwrap();
  auto func = *instance.get_func(context, *f);

  return Context{
      .engine = engine,
      .store = std::move(store),
      .context = context,
      .component = component,
      .instance = instance,
      .func = func,
  };
}

TEST(component, value_record) {
  static const auto check = [](const Val &v, uint64_t x, uint64_t y) {
    EXPECT_TRUE(v.is_record());
    const Record &r = v.get_record();
    EXPECT_EQ(r.size(), 2);

    const auto &x_field = *r.begin();
    EXPECT_EQ(x_field.name(), "x");
    const auto &x_field_val = x_field.value();
    EXPECT_TRUE(x_field_val.is_u64());
    EXPECT_EQ(x_field_val.get_u64(), x);

    const auto &y_field = *(r.begin() + 1);
    EXPECT_EQ(y_field.name(), "y");
    const auto &y_field_val = y_field.value();
    EXPECT_TRUE(y_field_val.is_u64());
    EXPECT_EQ(y_field_val.get_u64(), y);
  };

  static const auto make = [](uint64_t x, uint64_t y) -> Val {
    return Record({
        {"x", Val(x)},
        {"y", Val(y)},
    });
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
      +[](Store::Context, Span<const Val> args,
          Span<Val> rets) -> Result<std::monostate> {
        EXPECT_EQ(args.size(), 1);
        check(args[0], 1, 2);

        EXPECT_EQ(rets.size(), 1);
        rets[0] = make(3, 4);

        return std::monostate();
      });

  auto arg = make(1, 2);
  auto res = Val(false);

  ctx.func.call(ctx.context, Span<const Val>(&arg, 1), Span<Val>(&res, 1))
      .unwrap();
  ctx.func.post_return(ctx.context).unwrap();

  check(res, 3, 4);
}

TEST(component, value_string) {
  static const auto check = [](const Val &v, std::string_view text) {
    EXPECT_TRUE(v.is_string());
    EXPECT_EQ(v.get_string(), text);
  };

  static const auto make = [](std::string_view text) -> Val {
    return Val::string(text);
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
      +[](Store::Context, Span<const Val> args,
          Span<Val> rets) -> Result<std::monostate> {
        EXPECT_EQ(args.size(), 1);
        check(args[0], "hello from A!");

        EXPECT_EQ(rets.size(), 1);
        rets[0] = make("hello from B!");

        return std::monostate();
      });

  auto arg = make("hello from A!");
  auto res = Val(false);

  ctx.func.call(ctx.context, Span<const Val>(&arg, 1), Span<Val>(&res, 1))
      .unwrap();
  ctx.func.post_return(ctx.context).unwrap();

  check(res, "hello from B!");
}

TEST(component, value_list) {
  static const auto check = [](const Val &v, std::vector<uint32_t> data) {
    EXPECT_TRUE(v.is_list());
    const List &l = v.get_list();
    EXPECT_EQ(l.size(), data.size());

    for (auto i = 0; i < data.size(); i++) {
      const auto &elem = l.begin()[i];
      EXPECT_TRUE(elem.is_u32());
      EXPECT_EQ(elem.get_u32(), data[i]);
    }
  };

  static const auto make = [](std::vector<Val> data) -> Val {
    return List(data);
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
      +[](Store::Context, Span<const Val> args,
          Span<Val> rets) -> Result<std::monostate> {
        EXPECT_EQ(args.size(), 1);
        check(args[0], {1, 2, 3});

        EXPECT_EQ(rets.size(), 1);
        rets[0] = make({uint32_t(4), uint32_t(5), uint32_t(6), uint32_t(7)});

        return std::monostate();
      });

  auto arg = make({uint32_t(1), uint32_t(2), uint32_t(3)});
  auto res = Val(false);

  ctx.func.call(ctx.context, Span<const Val>(&arg, 1), Span<Val>(&res, 1))
      .unwrap();
  ctx.func.post_return(ctx.context).unwrap();

  check(res, {4, 5, 6, 7});
}

TEST(component, value_tuple) {
  static const auto check = [](const Val &v, std::vector<uint32_t> data) {
    EXPECT_TRUE(v.is_tuple());
    const Tuple &t = v.get_tuple();
    EXPECT_EQ(t.size(), data.size());
    for (auto i = 0; i < data.size(); i++) {
      const auto &elem = t.begin()[i];
      EXPECT_TRUE(elem.is_u32());
      EXPECT_EQ(elem.get_u32(), data[i]);
    }
  };

  static const auto make = [](std::vector<Val> data) -> Val {
    return Tuple(data);
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
      +[](Store::Context, Span<const Val> args,
          Span<Val> rets) -> Result<std::monostate> {
        EXPECT_EQ(args.size(), 1);
        check(args[0], {1, 2, 3});

        EXPECT_EQ(rets.size(), 1);
        rets[0] = make({uint32_t(4), uint32_t(5), uint32_t(6)});

        return std::monostate();
      });

  auto arg = make({uint32_t(1), uint32_t(2), uint32_t(3)});
  auto res = Val(false);

  ctx.func.call(ctx.context, Span<const Val>(&arg, 1), Span<Val>(&res, 1))
      .unwrap();
  ctx.func.post_return(ctx.context).unwrap();

  check(res, {4, 5, 6});
}

TEST(component, value_variant) {
  static const auto check_aa = [](const Val &v, uint32_t value) {
    EXPECT_TRUE(v.is_variant());
    const Variant &var = v.get_variant();
    EXPECT_EQ(var.discriminant(), "aa");
    EXPECT_NE(var.value(), nullptr);
    EXPECT_TRUE(var.value()->is_u32());
    EXPECT_EQ(var.value()->get_u32(), value);
  };

  static const auto check_bb = [](const Val &v, std::string_view value) {
    EXPECT_TRUE(v.is_variant());
    const Variant &var = v.get_variant();
    EXPECT_EQ(var.discriminant(), "bb");
    EXPECT_NE(var.value(), nullptr);
    EXPECT_TRUE(var.value()->is_string());
    EXPECT_EQ(var.value()->get_string(), value);
  };

  static const auto make_aa = [](uint32_t value) -> Val {
    return Variant("aa", Val(value));
  };

  static const auto make_bb = [](std::string_view value) -> Val {
    return Variant("bb", Val::string(value));
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
      +[](Store::Context, Span<const Val> args,
          Span<Val> rets) -> Result<std::monostate> {
        EXPECT_EQ(args.size(), 1);
        check_aa(args[0], 123);

        EXPECT_EQ(rets.size(), 1);
        rets[0] = make_bb("textt");

        return std::monostate();
      });

  auto arg = make_aa(123);
  auto res = Val(false);

  ctx.func.call(ctx.context, Span<const Val>(&arg, 1), Span<Val>(&res, 1))
      .unwrap();
  ctx.func.post_return(ctx.context).unwrap();

  check_bb(res, "textt");
}

TEST(component, value_enum) {
  static const auto check = [](const Val &v, std::string_view text) {
    EXPECT_TRUE(v.is_enum());
    EXPECT_EQ(v.get_enum(), text);
  };

  static const auto make = [](std::string_view text) -> Val {
    return Val::enum_(text);
  };

  auto ctx = create(
      R"((enum "aa" "bb"))", R"(
(param $x i32)
(result i32)
local.get $x
call $do
	  )",
      "(param i32) (result i32)",
      +[](Store::Context, Span<const Val> args,
          Span<Val> rets) -> Result<std::monostate> {
        EXPECT_EQ(args.size(), 1);
        check(args[0], "aa");

        EXPECT_EQ(rets.size(), 1);
        rets[0] = make("bb");

        return std::monostate();
      });

  auto arg = make("aa");
  auto res = Val(false);

  ctx.func.call(ctx.context, Span<const Val>(&arg, 1), Span<Val>(&res, 1))
      .unwrap();
  ctx.func.post_return(ctx.context).unwrap();

  check(res, "bb");
}

TEST(component, value_option) {
  static const auto check = [](const Val &v, std::optional<uint32_t> value) {
    EXPECT_TRUE(v.is_option());
    const WitOption &o = v.get_option();
    if (value.has_value()) {
      EXPECT_NE(o.value(), nullptr);
      EXPECT_TRUE(o.value()->is_u32());
      EXPECT_EQ(o.value()->get_u32(), *value);
    } else {
      EXPECT_EQ(o.value(), nullptr);
    }
  };

  static const auto make = [](std::optional<uint32_t> value) -> Val {
    if (value) {
      return WitOption(Val(*value));
    }
    return WitOption(std::nullopt);
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
      +[](Store::Context, Span<const Val> args,
          Span<Val> rets) -> Result<std::monostate> {
        EXPECT_EQ(args.size(), 1);
        check(args[0], 123);

        EXPECT_EQ(rets.size(), 1);
        rets[0] = make({});

        return std::monostate();
      });

  auto arg = make(123);
  auto res = Val(false);

  ctx.func.call(ctx.context, Span<const Val>(&arg, 1), Span<Val>(&res, 1))
      .unwrap();
  ctx.func.post_return(ctx.context).unwrap();

  check(res, {});
}

TEST(component, value_result) {
  static const auto check = [](const Val &v, bool expected_is_ok,
                               uint32_t expected_value) {
    EXPECT_TRUE(v.is_result());
    const WitResult &r = v.get_result();
    EXPECT_EQ(r.is_ok(), expected_is_ok);
    EXPECT_NE(r.payload(), nullptr);
    EXPECT_TRUE(r.payload()->is_u32());
    EXPECT_EQ(r.payload()->get_u32(), expected_value);
  };

  static const auto make = [](bool is_ok, uint32_t value) -> Val {
    if (is_ok) {
      return WitResult::ok(Val(value));
    }
    return WitResult::err(Val(value));
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
      +[](Store::Context, Span<const Val> args,
          Span<Val> rets) -> Result<std::monostate> {
        EXPECT_EQ(args.size(), 1);
        check(args[0], true, 123);

        EXPECT_EQ(rets.size(), 1);
        rets[0] = make(false, 456);

        return std::monostate();
      });

  auto arg = make(true, 123);
  auto res = Val(false);

  ctx.func.call(ctx.context, Span<const Val>(&arg, 1), Span<Val>(&res, 1))
      .unwrap();
  ctx.func.post_return(ctx.context).unwrap();

  check(res, false, 456);
}

TEST(component, value_flags) {
  static const auto check = [](const Val &v, std::vector<std::string> data) {
    EXPECT_TRUE(v.is_flags());
    const Flags &f = v.get_flags();

    EXPECT_EQ(f.size(), data.size());
    for (auto i = 0; i < data.size(); i++) {
      EXPECT_EQ(f.begin()[i].name(), data[i]);
    }
  };

  static const auto make = [](std::vector<Flag> data) -> Val {
    return Flags(data);
  };

  auto ctx = create(
      R"((flags "aa" "bb"))", R"(
(param $x i32)
(result i32)
local.get $x
call $do
	  )",
      "(param i32) (result i32)",
      +[](Store::Context, Span<const Val> args,
          Span<Val> rets) -> Result<std::monostate> {
        EXPECT_EQ(args.size(), 1);
        check(args[0], {"aa"});

        EXPECT_EQ(rets.size(), 1);
        rets[0] = make({Flag("aa"), Flag("bb")});

        return std::monostate();
      });

  auto arg = make({Flag("aa")});
  auto res = Val(false);

  ctx.func.call(ctx.context, Span<const Val>(&arg, 1), Span<Val>(&res, 1))
      .unwrap();
  ctx.func.post_return(ctx.context).unwrap();

  check(res, {"aa", "bb"});
}

TEST(component, value_list_inner) {
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

TEST(component, records) {
  Record r({{"x", uint64_t(1)}, {"y", uint64_t(2)}});
  EXPECT_EQ(r.size(), 2);

  for (auto &field : r) {
    if (field.name() == "x") {
      EXPECT_EQ(field.value().get_u64(), 1);
    } else if (field.name() == "y") {
      EXPECT_EQ(field.value().get_u64(), 2);
    } else {
      FAIL() << "unexpected field name: " << field.name();
    }
  }

  Record r2({{"x", r}, {"y", uint64_t(2)}});
  EXPECT_EQ(r2.size(), 2);
  EXPECT_EQ(r.size(), 2);

  for (auto &field : r2) {
    if (field.name() == "x") {
      auto inner = field.value().get_record();
      EXPECT_EQ(inner.size(), 2);
      for (auto &inner_field : inner) {
        if (inner_field.name() == "x") {
          EXPECT_EQ(inner_field.value().get_u64(), 1);
        } else if (inner_field.name() == "y") {
          EXPECT_EQ(inner_field.value().get_u64(), 2);
        } else {
          FAIL() << "unexpected inner field name: " << inner_field.name();
        }
      }
    } else if (field.name() == "y") {
      EXPECT_EQ(field.value().get_u64(), 2);
    } else {
      FAIL() << "unexpected field name: " << field.name();
    }
  }

  Val record = r2;
  EXPECT_TRUE(record.is_record());
  EXPECT_EQ(r2.size(), 2);
  Val record2 = std::move(r2);
  EXPECT_TRUE(record2.is_record());
  EXPECT_EQ(r2.size(), 0);
}

TEST(component, lists) {
  List l({uint32_t(1), uint32_t(2), uint32_t(3)});
  EXPECT_EQ(l.size(), 3);
  uint32_t expected = 1;
  for (auto &val : l) {
    EXPECT_EQ(val.get_u32(), expected);
    expected++;
  }

  List l2 = l;
  EXPECT_EQ(l.size(), 3);
  EXPECT_EQ(l2.size(), 3);

  List l3 = std::move(l);
  EXPECT_EQ(l.size(), 0);
  EXPECT_EQ(l3.size(), 3);

  Val value(l3);
  value.get_list();
}

TEST(component, tuples) {
  Tuple l({uint32_t(1), uint64_t(2), uint8_t(3)});
  EXPECT_EQ(l.size(), 3);

  Val value(l);
  EXPECT_TRUE(value.is_tuple());
  EXPECT_EQ(l.size(), 3);

  for (auto &val : l) {
    if (val.is_u32()) {
      EXPECT_EQ(val.get_u32(), 1);
    } else if (val.is_u64()) {
      EXPECT_EQ(val.get_u64(), 2);
    } else if (val.is_u8()) {
      EXPECT_EQ(val.get_u8(), 3);
    } else {
      FAIL() << "unexpected tuple value type";
    }
  }
}

TEST(component, variants) {
  Variant v("hello", uint32_t(42));
  EXPECT_EQ(v.discriminant(), "hello");
  EXPECT_TRUE(v.value()->is_u32());
  EXPECT_EQ(v.value()->get_u32(), 42);

  Variant v2("another", v);
  EXPECT_EQ(v.discriminant(), "hello");
  EXPECT_TRUE(v.value()->is_u32());
  EXPECT_EQ(v.value()->get_u32(), 42);
  EXPECT_EQ(v2.discriminant(), "another");
  EXPECT_TRUE(v2.value()->is_variant());
  auto inner = v2.value()->get_variant();
  EXPECT_EQ(inner.discriminant(), "hello");
  EXPECT_TRUE(inner.value()->is_u32());
  EXPECT_EQ(inner.value()->get_u32(), 42);

  Val value = v;
  EXPECT_TRUE(value.is_variant());
  auto v3 = value.get_variant();
  EXPECT_EQ(v3.discriminant(), "hello");
  EXPECT_TRUE(v3.value()->is_u32());
  EXPECT_EQ(v3.value()->get_u32(), 42);
}

TEST(component, strings) {
  Val v = Val::string("hi");
  EXPECT_TRUE(v.is_string());
  EXPECT_EQ(v.get_string(), "hi");

  v = Val::string("another");
  EXPECT_TRUE(v.is_string());
  EXPECT_EQ(v.get_string(), "another");
}

TEST(component, results) {
  WitResult r = WitResult::ok(uint32_t(42));
  EXPECT_TRUE(r.is_ok());
  EXPECT_EQ(r.payload()->get_u32(), 42);

  r = WitResult::ok(std::nullopt);
  EXPECT_TRUE(r.is_ok());
  EXPECT_EQ(r.payload(), nullptr);

  r = WitResult::err(std::nullopt);
  EXPECT_FALSE(r.is_ok());
  EXPECT_EQ(r.payload(), nullptr);

  Val v = r;
  EXPECT_TRUE(v.is_result());
  auto r2 = v.get_result();
  EXPECT_FALSE(r2.is_ok());
  EXPECT_EQ(r2.payload(), nullptr);

  r = WitResult::ok(uint32_t(99));
  v = r;
  EXPECT_TRUE(r.is_ok());
  EXPECT_NE(r.payload(), nullptr);
  EXPECT_EQ(r.payload()->get_u32(), 99);
}

TEST(component, enums) {
  Val v = Val::enum_("hi");
  EXPECT_TRUE(v.is_enum());
  EXPECT_EQ(v.get_enum(), "hi");

  v = Val::enum_("another");
  EXPECT_TRUE(v.is_enum());
  EXPECT_EQ(v.get_enum(), "another");
}

TEST(component, options) {
  WitOption o(Val(uint32_t(42)));
  EXPECT_NE(o.value(), nullptr);
  EXPECT_TRUE(o.value()->is_u32());
  EXPECT_EQ(o.value()->get_u32(), 42);

  Val v(o);
  WitOption o2(v);
  EXPECT_NE(o.value(), nullptr);
  EXPECT_TRUE(o2.value()->is_option());
  auto inner = o2.value()->get_option();
  auto value = inner.value();
  EXPECT_NE(value, nullptr);
  EXPECT_TRUE(value->is_u32());
  EXPECT_EQ(value->get_u32(), 42);

  EXPECT_NE(o.value(), nullptr);
  EXPECT_TRUE(o.value()->is_u32());
  EXPECT_EQ(o.value()->get_u32(), 42);

  WitOption o3(std::nullopt);
  EXPECT_EQ(o3.value(), nullptr);
}

TEST(component, flags) {
  std::vector<Flag> flags = {
      Flag("a"),
      Flag("b"),
      Flag("c"),
  };
  Flags f(flags);
  EXPECT_EQ(f.size(), 3);
  for (auto i = 0; i < f.size(); i++) {
    EXPECT_EQ(f.begin()[i].name(), flags[i].name());
  }

  flags.clear();
  Flags f2(flags);
  EXPECT_EQ(f2.size(), 0);
  EXPECT_EQ(f.size(), 3);

  Val v = f;
  EXPECT_TRUE(v.is_flags());
  Flags f3 = v.get_flags();
  EXPECT_EQ(f3.size(), 3);
  EXPECT_EQ(f.size(), 3);
}
