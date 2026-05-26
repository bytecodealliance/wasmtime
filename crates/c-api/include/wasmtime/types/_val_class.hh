#ifndef WASMTIME_TYPES_VAL_CLASS_HH
#define WASMTIME_TYPES_VAL_CLASS_HH

#include <memory>
#include <wasm.h>
#include <wasmtime/engine.hh>
#include <wasmtime/types/val.h>
#include <wasmtime/val.h>

namespace wasmtime {

class RefType;

/**
 * \brief Type information about a WebAssembly value.
 *
 * Currently mostly just contains the `ValKind`.
 */
class ValType {
  friend class TableType;
  friend class GlobalType;
  friend class FuncType;
  struct deleter {
    void operator()(wasm_valtype_t *p) const { wasm_valtype_delete(p); }
  };

  std::unique_ptr<wasm_valtype_t, deleter> ptr;

public:
  /// \brief Non-owning reference to a `ValType`, must not be used after the
  /// original `ValType` is deleted.
  class Ref {
    friend class ValType;

    const wasm_valtype_t *ptr;

  public:
    /// \brief Instantiates from the raw C API representation.
    Ref(const wasm_valtype_t *ptr) : ptr(ptr) {}
    /// Copy constructor
    Ref(const ValType &ty) : Ref(ty.ptr.get()) {}

    /// \brief Tests if this type is equal to another.
    bool operator==(const Ref &other) const {
      return wasmtime_wasm_valtype_equal(ptr, other.ptr);
    }
  };

  /// \brief Non-owning reference to a list of `ValType` instances. Must not be
  /// used after the original owner is deleted.
  class ListRef {
    const wasm_valtype_vec_t *list;

  public:
    /// Creates a list from the raw underlying C API.
    ListRef(const wasm_valtype_vec_t *list) : list(list) {}

    /// This list iterates over a list of `ValType::Ref` instances.
    typedef const Ref *iterator;

    /// Pointer to the beginning of iteration
    iterator begin() const {
      return reinterpret_cast<Ref *>(&list->data[0]); // NOLINT
    }

    /// Pointer to the end of iteration
    iterator end() const {
      return reinterpret_cast<Ref *>(&list->data[list->size]); // NOLINT
    }

    /// Returns how many types are in this list.
    size_t size() const { return list->size; }
  };

private:
  Ref ref;
  wasmtime_valtype_t wasmtime_ty;
  ValType(wasm_valtype_t *ptr) : ptr(ptr), ref(ptr) {
    wasmtime_valtype_new(ptr, &wasmtime_ty);
  }
  ValType(wasmtime_valtype_t *ty)
      : ptr(wasmtime_valtype_to_wasm(nullptr, ty)), ref(nullptr),
        wasmtime_ty(*ty) {
    ref = ptr.get();
  }

public:
  /// Copies a `Ref` to a new owned value.
  ValType(Ref other) : ValType(wasm_valtype_copy(other.ptr)) {}
  /// Copies one type to a new one.
  ValType(const ValType &other) : ValType(wasm_valtype_copy(other.ptr.get())) {}
  /// Copies the contents of another type into this one.
  ValType &operator=(const ValType &other) {
    ptr.reset(wasm_valtype_copy(other.ptr.get()));
    ref = ptr.get();
    wasmtime_valtype_delete(&wasmtime_ty);
    wasmtime_valtype_clone(&other.wasmtime_ty, &wasmtime_ty);
    return *this;
  }
  ~ValType() { wasmtime_valtype_delete(&wasmtime_ty); }
  /// Moves the memory owned by another value type into this one.
  ValType(ValType &&other) : ptr(std::move(other.ptr)), ref(ptr.get()) {
    wasmtime_ty = other.wasmtime_ty;
    other.ref = nullptr;
    other.wasmtime_ty.kind = WASMTIME_VALTYPE_KIND_I32;
  }
  /// Moves the memory owned by another value type into this one.
  ValType &operator=(ValType &&other) {
    ptr = std::move(other.ptr);
    ref = ptr.get();
    wasmtime_ty = other.wasmtime_ty;
    other.ref = nullptr;
    other.wasmtime_ty.kind = WASMTIME_VALTYPE_KIND_I32;
    return *this;
  }

  /// Convenience constructor for the `i32` value type.
  static ValType i32() { return ValType(wasm_valtype_new(WASM_I32)); }

  /// Convenience constructor for the `i64` value type.
  static ValType i64() { return ValType(wasm_valtype_new(WASM_I64)); }

  /// Convenience constructor for the `f32` value type.
  static ValType f32() { return ValType(wasm_valtype_new(WASM_F32)); }

  /// Convenience constructor for the `f64` value type.
  static ValType f64() { return ValType(wasm_valtype_new(WASM_F64)); }

  /// Convenience constructor for the `v128` value type.
  static ValType v128() {
    wasmtime_valtype_t ty;
    ty.kind = WASMTIME_VALTYPE_KIND_V128;
    return ValType(&ty);
  }

  /// Convenience constructor for the `funcref` value type.
  static ValType funcref() { return ValType(wasm_valtype_new(WASM_FUNCREF)); }

  /// Convenience constructor for the `externref` value type.
  static ValType externref() {
    return ValType(wasm_valtype_new(WASM_EXTERNREF));
  }

  /// Convenience constructor for the `anyref` value type.
  static ValType anyref();

  /// Convenience constructor for the `exnref` value type.
  static ValType exnref();

  /// Convenience constructor for reference types.
  ValType(const Engine &engine, const RefType &ref_type);

  /// \brief Returns the underlying `Ref`, a non-owning reference pointing to
  /// this instance.
  Ref *operator->() { return &ref; }
  /// \brief Returns the underlying `Ref`, a non-owning reference pointing to
  /// this instance.
  Ref *operator*() { return &ref; }

  /// \brief Equality operator, compares the underlying types for equality.
  bool operator==(const Ref &other) const {
    return wasmtime_wasm_valtype_equal(ptr.get(), other.ptr);
  }

  /// \brief Equality operator, compares the underlying types for equality.
  bool operator==(const ValType &other) const {
    return wasmtime_wasm_valtype_equal(ptr.get(), other.ptr.get());
  }

  /// \brief Returns the underlying C API representation of this type.
  const wasmtime_valtype_t *wasmtime_capi() const { return &wasmtime_ty; }

  /// \brief Returns the underlying C API representation of this type.
  const wasm_valtype_t *capi() const { return ptr.get(); }

  /// \brief Returns if this is the `i32` wasm type.
  bool is_i32() const { return wasmtime_ty.kind == WASMTIME_VALTYPE_KIND_I32; }

  /// \brief Returns if this is the `i64` wasm type.
  bool is_i64() const { return wasmtime_ty.kind == WASMTIME_VALTYPE_KIND_I64; }

  /// \brief Returns if this is the `f32` wasm type.
  bool is_f32() const { return wasmtime_ty.kind == WASMTIME_VALTYPE_KIND_F32; }

  /// \brief Returns if this is the `f64` wasm type.
  bool is_f64() const { return wasmtime_ty.kind == WASMTIME_VALTYPE_KIND_F64; }

  /// \brief Returns if this is the `v128` wasm type.
  bool is_v128() const {
    return wasmtime_ty.kind == WASMTIME_VALTYPE_KIND_V128;
  }

  /// \brief Returns if this is a reference type.
  const RefType *as_ref() const;
};

}; // namespace wasmtime

#endif // WASMTIME_TYPES_VAL_CLASS_HH
