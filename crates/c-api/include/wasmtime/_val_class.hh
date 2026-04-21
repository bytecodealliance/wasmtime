#ifndef WASMTIME_VAL_CLASS_HH
#define WASMTIME_VAL_CLASS_HH

#include <optional>
#include <wasmtime/_anyref_class.hh>
#include <wasmtime/_arrayref_class.hh>
#include <wasmtime/_eqref_class.hh>
#include <wasmtime/_externref_class.hh>
#include <wasmtime/_func_class.hh>
#include <wasmtime/_store_class.hh>
#include <wasmtime/_structref_class.hh>
#include <wasmtime/types/val.hh>
#include <wasmtime/val.h>

namespace wasmtime {

/// \brief Container for the `v128` WebAssembly type.
struct V128 {
  /// \brief The little-endian bytes of the `v128` value.
  wasmtime_v128 v128;

  /// \brief Creates a new zero-value `v128`.
  V128() : v128{} { memset(&v128[0], 0, sizeof(wasmtime_v128)); }

  /// \brief Creates a new `V128` from its C API representation.
  V128(const wasmtime_v128 &v) : v128{} {
    memcpy(&v128[0], &v[0], sizeof(wasmtime_v128));
  }
};

/**
 * \brief Representation of a generic WebAssembly value.
 *
 * This is roughly equivalent to a tagged union of all possible WebAssembly
 * values. This is later used as an argument with functions, globals, tables,
 * etc.
 *
 * Note that a `Val` can represent owned GC pointers. In this case the `unroot`
 * method must be used to ensure that they can later be garbage-collected.
 */
class Val {
  friend class Global;
  friend class Table;
  friend class Func;

  wasmtime_val_t val;

  Val() : val{} {
    val.kind = WASMTIME_I32;
    val.of.i32 = 0;
  }

public:
  /// Creates a new value from the raw C API representation.
  Val(wasmtime_val_t val) : val(val) {}

  /// Creates a new `i32` WebAssembly value.
  Val(int32_t i32) : val{} {
    val.kind = WASMTIME_I32;
    val.of.i32 = i32;
  }
  /// Creates a new `i64` WebAssembly value.
  Val(int64_t i64) : val{} {
    val.kind = WASMTIME_I64;
    val.of.i64 = i64;
  }
  /// Creates a new `f32` WebAssembly value.
  Val(float f32) : val{} {
    val.kind = WASMTIME_F32;
    val.of.f32 = f32;
  }
  /// Creates a new `f64` WebAssembly value.
  Val(double f64) : val{} {
    val.kind = WASMTIME_F64;
    val.of.f64 = f64;
  }
  /// Creates a new `v128` WebAssembly value.
  Val(const V128 &v128) : val{} {
    val.kind = WASMTIME_V128;
    memcpy(&val.of.v128[0], &v128.v128[0], sizeof(wasmtime_v128));
  }
  /// Creates a new `funcref` WebAssembly value.
  Val(std::optional<Func> func);
  /// Creates a new `funcref` WebAssembly value which is not `ref.null func`.
  Val(Func func);
#ifdef WASMTIME_FEATURE_GC
  /// Creates a new `externref` value.
  Val(std::optional<ExternRef> ptr);
  /// Creates a new `anyref` value.
  Val(std::optional<AnyRef> ptr);
  /// Creates a new `externref` WebAssembly value which is not `ref.null
  /// extern`.
  Val(ExternRef ptr);
  /// Creates a new `anyref` WebAssembly value which is not `ref.null
  /// any`.
  Val(AnyRef ptr);
#endif

  /// Copy constructor to clone `other`.
  Val(const Val &other) { wasmtime_val_clone(&other.val, &val); }

  /// Copy assignment to clone from `other`.
  Val &operator=(const Val &other) {
    wasmtime_val_unroot(&val);
    wasmtime_val_clone(&other.val, &val);
    return *this;
  }

  /// Move constructor to move the contents of `other`.
  Val(Val &&other) {
    val = other.val;
    other.val.kind = WASMTIME_I32;
    other.val.of.i32 = 0;
  }

  /// Move assignment to move the contents of `other`.
  Val &operator=(Val &&other) {
    wasmtime_val_unroot(&val);
    val = other.val;
    other.val.kind = WASMTIME_I32;
    other.val.of.i32 = 0;
    return *this;
  }

  /// Unroots the values in `val`, if any.
  ~Val() { wasmtime_val_unroot(&val); }

  /// Returns the kind of value that this value has.
  ValKind kind() const {
    switch (val.kind) {
    case WASMTIME_I32:
      return ValKind::I32;
    case WASMTIME_I64:
      return ValKind::I64;
    case WASMTIME_F32:
      return ValKind::F32;
    case WASMTIME_F64:
      return ValKind::F64;
    case WASMTIME_FUNCREF:
      return ValKind::FuncRef;
    case WASMTIME_EXTERNREF:
      return ValKind::ExternRef;
    case WASMTIME_ANYREF:
      return ValKind::AnyRef;
    case WASMTIME_EXNREF:
      return ValKind::ExnRef;
    case WASMTIME_V128:
      return ValKind::V128;
    }
    std::abort();
  }

  /// Returns the underlying `i32`, requires `kind() == KindI32` or aborts the
  /// process.
  int32_t i32() const {
    if (val.kind != WASMTIME_I32) {
      std::abort();
    }
    return val.of.i32;
  }

  /// Returns the underlying `i64`, requires `kind() == KindI64` or aborts the
  /// process.
  int64_t i64() const {
    if (val.kind != WASMTIME_I64) {
      std::abort();
    }
    return val.of.i64;
  }

  /// Returns the underlying `f32`, requires `kind() == KindF32` or aborts the
  /// process.
  float f32() const {
    if (val.kind != WASMTIME_F32) {
      std::abort();
    }
    return val.of.f32;
  }

  /// Returns the underlying `f64`, requires `kind() == KindF64` or aborts the
  /// process.
  double f64() const {
    if (val.kind != WASMTIME_F64) {
      std::abort();
    }
    return val.of.f64;
  }

  /// Returns the underlying `v128`, requires `kind() == KindV128` or aborts
  /// the process.
  V128 v128() const {
    if (val.kind != WASMTIME_V128) {
      std::abort();
    }
    return val.of.v128;
  }

#ifdef WASMTIME_FEATURE_GC
  /// Returns the underlying `externref`, requires `kind() == KindExternRef` or
  /// aborts the process.
  ///
  /// Note that `externref` is a nullable reference, hence the `optional` return
  /// value.
  std::optional<ExternRef> externref() const;

  /// Returns the underlying `anyref`, requires `kind() == KindAnyRef` or
  /// aborts the process.
  ///
  /// Note that `anyref` is a nullable reference, hence the `optional` return
  /// value.
  std::optional<AnyRef> anyref() const;
#endif

  /// Returns the underlying `funcref`, requires `kind() == KindFuncRef` or
  /// aborts the process.
  ///
  /// Note that `funcref` is a nullable reference, hence the `optional` return
  /// value.
  std::optional<Func> funcref() const;

  /// Raw C-API representation.
  using Raw = wasmtime_val_t;

  /**
   * \brief Returns the underlying C API pointer.
   */
  const Raw *capi() const { return &val; }

  /**
   * \brief Returns the underlying C API pointer.
   */
  Raw *capi() { return &val; }
};

} // namespace wasmtime

#endif // WASMTIME_VAL_CLASS_HH
