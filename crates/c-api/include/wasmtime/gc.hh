/**
 * \file wasmtime/gc.hh
 *
 * C++ API for WebAssembly GC types: eqref, structref, and arrayref.
 */

#ifndef WASMTIME_GC_HH
#define WASMTIME_GC_HH

#include <vector>
#include <wasmtime/gc.h>
#include <wasmtime/val.hh>

namespace wasmtime {

class StructRef;
class ArrayRef;

/**
 * \brief Representation of a WebAssembly `eqref` value.
 *
 * An `eqref` is a reference to a GC object that supports equality testing.
 * Subtypes include `structref`, `arrayref`, and `i31ref`.
 *
 * Like all GC references, `EqRef` values are rooted in a `Store` and must be
 * unrooted (by destruction or move) to allow garbage collection.
 */
class EqRef {
  friend class Val;
  friend class AnyRef;

  wasmtime_eqref_t val;

public:
  /// Creates a new `EqRef` from its C-API representation.
  explicit EqRef(wasmtime_eqref_t val) : val(val) {}

  /// Copy constructor.
  EqRef(const EqRef &other) { wasmtime_eqref_clone(&other.val, &val); }

  /// Copy assignment.
  EqRef &operator=(const EqRef &other) {
    wasmtime_eqref_unroot(&val);
    wasmtime_eqref_clone(&other.val, &val);
    return *this;
  }

  /// Move constructor.
  EqRef(EqRef &&other) {
    val = other.val;
    wasmtime_eqref_set_null(&other.val);
  }

  /// Move assignment.
  EqRef &operator=(EqRef &&other) {
    wasmtime_eqref_unroot(&val);
    val = other.val;
    wasmtime_eqref_set_null(&other.val);
    return *this;
  }

  ~EqRef() { wasmtime_eqref_unroot(&val); }

  /// Create an `eqref` from an i31 value.
  static EqRef from_i31(Store::Context cx, uint32_t val) {
    wasmtime_eqref_t out;
    wasmtime_eqref_from_i31(cx.capi(), val, &out);
    return EqRef(out);
  }

  /// Returns `true` if this eqref is an i31ref.
  bool is_i31(Store::Context cx) const {
    return wasmtime_eqref_is_i31(cx.capi(), &val);
  }

  /// Get the i31 value as an unsigned 32-bit integer.
  /// Returns `std::nullopt` if this eqref is not an i31ref.
  std::optional<uint32_t> i31_get_u(Store::Context cx) const {
    uint32_t dst;
    if (wasmtime_eqref_i31_get_u(cx.capi(), &val, &dst))
      return dst;
    return std::nullopt;
  }

  /// Get the i31 value as a signed 32-bit integer.
  /// Returns `std::nullopt` if this eqref is not an i31ref.
  std::optional<int32_t> i31_get_s(Store::Context cx) const {
    int32_t dst;
    if (wasmtime_eqref_i31_get_s(cx.capi(), &val, &dst))
      return dst;
    return std::nullopt;
  }

  /// Returns `true` if this eqref is a structref.
  bool is_struct(Store::Context cx) const {
    return wasmtime_eqref_is_struct(cx.capi(), &val);
  }

  /// Returns `true` if this eqref is an arrayref.
  bool is_array(Store::Context cx) const {
    return wasmtime_eqref_is_array(cx.capi(), &val);
  }

  /// Upcast this `eqref` to an `anyref`.
  AnyRef to_anyref() const {
    wasmtime_anyref_t out;
    wasmtime_eqref_to_anyref(&val, &out);
    return AnyRef(out);
  }

  /// Downcast this `eqref` into a `structref`.
  //
  // as_struct() defined after StructRef below.
  inline StructRef as_struct(Store::Context cx) const;

  /// Downcast this `eqref` into an `arrayref`.
  //
  // as_array() defined after ArrayRef below.
  inline ArrayRef as_array(Store::Context cx) const;
};

/**
 * \brief Describes the storage type and mutability of a struct field or array
 * element.
 */
struct FieldType {
  /// The field's storage kind.
  wasmtime_storage_kind_t kind;
  /// Whether the field is mutable or not.
  bool mutable_;

  /// Create a mutable field type.
  static FieldType mut_(wasmtime_storage_kind_t k) { return {k, true}; }
  /// Create an immutable field type.
  static FieldType const_(wasmtime_storage_kind_t k) { return {k, false}; }
};

/**
 * \brief Owned handle to a WebAssembly struct type definition.
 *
 * Create with StructType::create, then use with StructRefPre to allocate
 * instances.
 */
class StructType {
  struct Deleter {
    void operator()(wasmtime_struct_type_t *p) const {
      wasmtime_struct_type_delete(p);
    }
  };
  std::unique_ptr<wasmtime_struct_type_t, Deleter> ptr;

public:
  /// Create a new struct type with the given fields.
  static StructType create(const Engine &engine,
                           const std::vector<FieldType> &fields) {
    static_assert(sizeof(FieldType) == sizeof(wasmtime_field_type_t));
    auto *raw = wasmtime_struct_type_new(
        engine.capi(),
        reinterpret_cast<const wasmtime_field_type_t *>(fields.data()),
        fields.size());
    StructType ty;
    ty.ptr.reset(raw);
    return ty;
  }

  /// Get the underlying C pointer (non-owning).
  const wasmtime_struct_type_t *capi() const { return ptr.get(); }

private:
  StructType() = default;
  friend class StructRefPre;
};

/**
 * \brief Pre-allocated struct layout for fast allocation of struct instances.
 *
 * Created from a StructType and a store context. Reusable for allocating
 * many struct instances of the same type.
 */
class StructRefPre {
  friend class StructRef;
  WASMTIME_OWN_WRAPPER(StructRefPre, wasmtime_struct_ref_pre)

public:
  /// Create a new struct pre-allocator.
  static StructRefPre create(Store::Context cx, const StructType &ty) {
    auto *raw = wasmtime_struct_ref_pre_new(cx.capi(), ty.capi());
    StructRefPre pre(raw);
    return pre;
  }
};

/**
 * \brief Representation of a WebAssembly `structref` value.
 *
 * A `structref` is a reference to a GC struct instance. It is a subtype
 * of `eqref` and `anyref`.
 */
class StructRef {
  friend class EqRef;
  friend class Val;
  friend class AnyRef;

  wasmtime_structref_t val;

public:
  /// Create a `StructRef` from its C-API representation.
  explicit StructRef(wasmtime_structref_t val) : val(val) {}

  /// Clone a `StructRef`.
  StructRef(const StructRef &other) {
    wasmtime_structref_clone(&other.val, &val);
  }

  /// Clone a `StructRef` into this one.
  StructRef &operator=(const StructRef &other) {
    wasmtime_structref_unroot(&val);
    wasmtime_structref_clone(&other.val, &val);
    return *this;
  }

  /// Move a `StructRef`.
  StructRef(StructRef &&other) {
    val = other.val;
    wasmtime_structref_set_null(&other.val);
  }

  /// Move a `StructRef` into this one.
  StructRef &operator=(StructRef &&other) {
    wasmtime_structref_unroot(&val);
    val = other.val;
    wasmtime_structref_set_null(&other.val);
    return *this;
  }

  /// Unroot this `StructRef`.
  ~StructRef() { wasmtime_structref_unroot(&val); }

  /// Allocate a new struct instance.
  static Result<StructRef> create(Store::Context cx, const StructRefPre &pre,
                                  const std::vector<Val> &fields) {
    std::vector<wasmtime_val_t> c_fields;
    c_fields.reserve(fields.size());
    for (auto &f : fields) {
      c_fields.push_back(f.val);
    }

    wasmtime_structref_t out;
    auto *err = wasmtime_structref_new(cx.capi(), pre.capi(), c_fields.data(),
                                       c_fields.size(), &out);
    if (err)
      return Result<StructRef>(Error(err));
    return Result<StructRef>(StructRef(out));
  }

  /// Read a field from the struct.
  Result<Val> field(Store::Context cx, size_t index) const {
    wasmtime_val_t out;
    auto *err = wasmtime_structref_field(cx.capi(), &val, index, &out);
    if (err)
      return Result<Val>(Error(err));
    return Result<Val>(Val(out));
  }

  /// Set a field of the struct.
  Result<std::monostate> set_field(Store::Context cx, size_t index,
                                   const Val &value) const {
    auto *err =
        wasmtime_structref_set_field(cx.capi(), &val, index, &value.val);
    if (err)
      return Result<std::monostate>(Error(err));
    return Result<std::monostate>(std::monostate{});
  }

  /// Upcast to anyref.
  AnyRef to_anyref() const {
    wasmtime_anyref_t out;
    wasmtime_structref_to_anyref(&val, &out);
    return AnyRef(out);
  }

  /// Upcast to eqref.
  EqRef to_eqref() const {
    wasmtime_eqref_t out;
    wasmtime_structref_to_eqref(&val, &out);
    return EqRef(out);
  }
};

inline StructRef EqRef::as_struct(Store::Context cx) const {
  wasmtime_structref_t out;
  wasmtime_eqref_as_struct(cx.capi(), &val, &out);
  return StructRef(out);
}

/**
 * \brief Owned handle to a WebAssembly array type definition.
 */
class ArrayType {
  struct Deleter {
    void operator()(wasmtime_array_type_t *p) const {
      wasmtime_array_type_delete(p);
    }
  };
  std::unique_ptr<wasmtime_array_type_t, Deleter> ptr;

public:
  /// Create a new array type with the given element type.
  static ArrayType create(const Engine &engine, const FieldType &field) {
    static_assert(sizeof(FieldType) == sizeof(wasmtime_field_type_t));
    auto *raw = wasmtime_array_type_new(
        engine.capi(), reinterpret_cast<const wasmtime_field_type_t *>(&field));
    ArrayType ty;
    ty.ptr.reset(raw);
    return ty;
  }

  /// Get the underlying C pointer (non-owning).
  const wasmtime_array_type_t *capi() const { return ptr.get(); }

private:
  ArrayType() = default;
  friend class ArrayRefPre;
};

/**
 * \brief Pre-allocated array layout for fast allocation of array instances.
 *
 * Created from a ArrayType and a store context. Reusable for allocating
 * many array instances of the same type.
 */
class ArrayRefPre {
  friend class ArrayRef;
  WASMTIME_OWN_WRAPPER(ArrayRefPre, wasmtime_array_ref_pre)

public:
  /// Create a new array pre-allocator.
  static ArrayRefPre create(Store::Context cx, const ArrayType &ty) {
    auto *raw = wasmtime_array_ref_pre_new(cx.capi(), ty.capi());
    ArrayRefPre pre(raw);
    return pre;
  }
};

/**
 * \brief Representation of a WebAssembly `arrayref` value.
 *
 * An `arrayref` is a reference to a GC array instance. It is a subtype
 * of `eqref` and `anyref`.
 */
class ArrayRef {
  friend class EqRef;
  friend class Val;
  friend class AnyRef;

  wasmtime_arrayref_t val;

public:
  /// Create a `ArrayRef` from its C-API representation.
  explicit ArrayRef(wasmtime_arrayref_t val) : val(val) {}

  /// Clone a `ArrayRef`.
  ArrayRef(const ArrayRef &other) { wasmtime_arrayref_clone(&other.val, &val); }

  /// Clone a `ArrayRef` into this one.
  ArrayRef &operator=(const ArrayRef &other) {
    wasmtime_arrayref_unroot(&val);
    wasmtime_arrayref_clone(&other.val, &val);
    return *this;
  }

  /// Move a `ArrayRef`.
  ArrayRef(ArrayRef &&other) {
    val = other.val;
    wasmtime_arrayref_set_null(&other.val);
  }

  /// Move a `ArrayRef` into this one.
  ArrayRef &operator=(ArrayRef &&other) {
    wasmtime_arrayref_unroot(&val);
    val = other.val;
    wasmtime_arrayref_set_null(&other.val);
    return *this;
  }

  /// Unroot this `ArrayRef`.
  ~ArrayRef() { wasmtime_arrayref_unroot(&val); }

  /// Allocate a new array with all elements set to the same value.
  static Result<ArrayRef> create(Store::Context cx, const ArrayRefPre &pre,
                                 const Val &elem, uint32_t len) {
    wasmtime_arrayref_t out;
    auto *err =
        wasmtime_arrayref_new(cx.capi(), pre.capi(), &elem.val, len, &out);
    if (err)
      return Result<ArrayRef>(Error(err));
    return Result<ArrayRef>(ArrayRef(out));
  }

  /// Get the length of the array.
  Result<uint32_t> len(Store::Context cx) const {
    uint32_t out;
    auto *err = wasmtime_arrayref_len(cx.capi(), &val, &out);
    if (err)
      return Result<uint32_t>(Error(err));
    return Result<uint32_t>(out);
  }

  /// Read an element from the array.
  Result<Val> get(Store::Context cx, uint32_t index) const {
    wasmtime_val_t out;
    auto *err = wasmtime_arrayref_get(cx.capi(), &val, index, &out);
    if (err)
      return Result<Val>(Error(err));
    return Result<Val>(Val(out));
  }

  /// Set an element of the array.
  Result<std::monostate> set(Store::Context cx, uint32_t index,
                             const Val &value) const {
    auto *err = wasmtime_arrayref_set(cx.capi(), &val, index, &value.val);
    if (err)
      return Result<std::monostate>(Error(err));
    return Result<std::monostate>(std::monostate{});
  }

  /// Upcast to anyref.
  AnyRef to_anyref() const {
    wasmtime_anyref_t out;
    wasmtime_arrayref_to_anyref(&val, &out);
    return AnyRef(out);
  }

  /// Upcast to eqref.
  EqRef to_eqref() const {
    wasmtime_eqref_t out;
    wasmtime_arrayref_to_eqref(&val, &out);
    return EqRef(out);
  }
};

inline ArrayRef EqRef::as_array(Store::Context cx) const {
  wasmtime_arrayref_t out;
  wasmtime_eqref_as_array(cx.capi(), &val, &out);
  return ArrayRef(out);
}

// AnyRef downcast method definitions (declared in val.hh)
inline bool AnyRef::is_eqref(Store::Context cx) const {
  return wasmtime_anyref_is_eqref(cx.capi(), &val);
}
inline bool AnyRef::is_struct(Store::Context cx) const {
  return wasmtime_anyref_is_struct(cx.capi(), &val);
}
inline bool AnyRef::is_array(Store::Context cx) const {
  return wasmtime_anyref_is_array(cx.capi(), &val);
}
inline std::optional<EqRef> AnyRef::as_eqref(Store::Context cx) const {
  wasmtime_eqref_t out;
  if (wasmtime_anyref_as_eqref(cx.capi(), &val, &out))
    return EqRef(out);
  return std::nullopt;
}
inline std::optional<StructRef> AnyRef::as_struct(Store::Context cx) const {
  wasmtime_structref_t out;
  if (wasmtime_anyref_as_struct(cx.capi(), &val, &out))
    return StructRef(out);
  return std::nullopt;
}
inline std::optional<ArrayRef> AnyRef::as_array(Store::Context cx) const {
  wasmtime_arrayref_t out;
  if (wasmtime_anyref_as_array(cx.capi(), &val, &out))
    return ArrayRef(out);
  return std::nullopt;
}

} // namespace wasmtime

#endif // WASMTIME_GC_HH
