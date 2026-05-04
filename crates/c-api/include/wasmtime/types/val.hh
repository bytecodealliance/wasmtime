/**
 * \file wasmtime/types/val.hh
 */

#ifndef WASMTIME_TYPES_VAL_HH
#define WASMTIME_TYPES_VAL_HH

#include <memory>
#include <ostream>
#include <wasm.h>
#include <wasmtime/types/_structref_class.hh>
#include <wasmtime/types/_val_class.hh>
#include <wasmtime/types/arrayref.hh>
#include <wasmtime/types/exnref.hh>
#include <wasmtime/types/func.hh>
#include <wasmtime/types/val.h>
#include <wasmtime/val.h>

namespace wasmtime {

/// Representation of a heap type in WebAssembly.
class HeapType {
  wasmtime_heaptype_t ty;

  HeapType(wasmtime_heaptype_kind_t ty) { this->ty.kind = ty; }

public:
  /// Constructor from the raw C API representation.
  HeapType(wasmtime_heaptype_t &ty) : ty(ty) {
    ty.kind = WASMTIME_HEAPTYPE_KIND_NONE;
  }

  /// Copy constructor.
  HeapType(const HeapType &other) { wasmtime_heaptype_clone(&other.ty, &ty); }
  /// Copy assignment operator.
  HeapType &operator=(const HeapType &other) {
    wasmtime_heaptype_delete(&ty);
    wasmtime_heaptype_clone(&other.ty, &ty);
    return *this;
  }
  ~HeapType() {
    if (is_concrete())
      wasmtime_heaptype_delete(&ty);
  }
  /// Move constructor
  HeapType(HeapType &&other) {
    ty = other.ty;
    other.ty.kind = WASMTIME_HEAPTYPE_KIND_NONE;
  }
  /// Move assignment operator
  HeapType &operator=(HeapType &&other) {
    wasmtime_heaptype_delete(&ty);
    ty = other.ty;
    other.ty.kind = WASMTIME_HEAPTYPE_KIND_NONE;
    return *this;
  }

  /// \brief Constructor for the `extern` heap type
  static HeapType extern_() { return HeapType(WASMTIME_HEAPTYPE_KIND_EXTERN); }

  /// \brief Constructor for the `noextern` heap type
  static HeapType noextern() {
    return HeapType(WASMTIME_HEAPTYPE_KIND_NOEXTERN);
  }

  /// \brief Constructor for the `func` heap type
  static HeapType func() { return HeapType(WASMTIME_HEAPTYPE_KIND_FUNC); }

  /// \brief Constructor for a concrete function heap type.
  HeapType(const FuncType &ty) {
    this->ty.kind = WASMTIME_HEAPTYPE_KIND_CONCRETE_FUNC;
    this->ty.of.concrete_func = FuncType(ty).capi_release();
  }

  /// \brief Constructor for the `nofunc` heap type
  static HeapType nofunc() { return HeapType(WASMTIME_HEAPTYPE_KIND_NOFUNC); }

  /// \brief Constructor for the `any` heap type
  static HeapType any() { return HeapType(WASMTIME_HEAPTYPE_KIND_ANY); }

  /// \brief Constructor for the `none` heap type
  static HeapType none() { return HeapType(WASMTIME_HEAPTYPE_KIND_NONE); }

  /// \brief Constructor for the `eq` heap type
  static HeapType eq() { return HeapType(WASMTIME_HEAPTYPE_KIND_EQ); }

  /// \brief Constructor for the `i31` heap type
  static HeapType i31() { return HeapType(WASMTIME_HEAPTYPE_KIND_I31); }

  /// \brief Constructor for the `array` heap type
  static HeapType array() { return HeapType(WASMTIME_HEAPTYPE_KIND_ARRAY); }

  /// \brief Constructor for a concrete array heap type.
  HeapType(const ArrayType &ty) {
    this->ty.kind = WASMTIME_HEAPTYPE_KIND_CONCRETE_ARRAY;
    this->ty.of.concrete_array = ArrayType(ty).capi_release();
  }

  /// \brief Constructor for the `struct` heap type
  static HeapType struct_() { return HeapType(WASMTIME_HEAPTYPE_KIND_STRUCT); }

  /// \brief Constructor for a concrete struct heap type.
  HeapType(const StructType &ty) {
    this->ty.kind = WASMTIME_HEAPTYPE_KIND_CONCRETE_STRUCT;
    this->ty.of.concrete_struct = StructType(ty).capi_release();
  }

  /// \brief Constructor for the `exn` heap type
  static HeapType exn() { return HeapType(WASMTIME_HEAPTYPE_KIND_EXN); }

  /// \brief Constructor for a concrete exception heap type.
  HeapType(const ExnType &ty) {
    this->ty.kind = WASMTIME_HEAPTYPE_KIND_CONCRETE_EXN;
    this->ty.of.concrete_exn = ExnType(ty).capi_release();
  }

  /// \brief Constructor for the `noexn` heap type
  static HeapType noexn() { return HeapType(WASMTIME_HEAPTYPE_KIND_NOEXN); }

  /// \brief Is this a concrete heap type?
  bool is_concrete() const {
    switch (ty.kind) {
    case WASMTIME_HEAPTYPE_KIND_CONCRETE_FUNC:
    case WASMTIME_HEAPTYPE_KIND_CONCRETE_ARRAY:
    case WASMTIME_HEAPTYPE_KIND_CONCRETE_STRUCT:
    case WASMTIME_HEAPTYPE_KIND_CONCRETE_EXN:
      return true;
    default:
      return false;
    }
  }

  /// \brief Is this the abstract `extern` heap type?
  bool is_extern() const { return ty.kind == WASMTIME_HEAPTYPE_KIND_EXTERN; }

  /// \brief Is this the abstract `noextern` heap type?
  bool is_noextern() const {
    return ty.kind == WASMTIME_HEAPTYPE_KIND_NOEXTERN;
  }

  /// \brief Is this the abstract `func` heap type?
  bool is_func() const { return ty.kind == WASMTIME_HEAPTYPE_KIND_FUNC; }

  /// \brief If this is a concrete function type, returns the underlying
  /// function type.
  std::optional<FuncType::Ref> as_concrete_func() const {
    if (ty.kind == WASMTIME_HEAPTYPE_KIND_CONCRETE_FUNC) {
      return FuncType::Ref(ty.of.concrete_func);
    }
    return std::nullopt;
  }

  /// \brief Is this the abstract `nofunc` heap type?
  bool is_nofunc() const { return ty.kind == WASMTIME_HEAPTYPE_KIND_NOFUNC; }

  /// \brief Is this the abstract `any` heap type?
  bool is_any() const { return ty.kind == WASMTIME_HEAPTYPE_KIND_ANY; }

  /// \brief Is this the abstract `none` heap type?
  bool is_none() const { return ty.kind == WASMTIME_HEAPTYPE_KIND_NONE; }

  /// \brief Is this the abstract `eq` heap type?
  bool is_eq() const { return ty.kind == WASMTIME_HEAPTYPE_KIND_EQ; }

  /// \brief Is this the abstract `i31` heap type?
  bool is_i31() const { return ty.kind == WASMTIME_HEAPTYPE_KIND_I31; }

  /// \brief Is this the abstract `array` heap type?
  bool is_array() const { return ty.kind == WASMTIME_HEAPTYPE_KIND_ARRAY; }

  /// \brief If this is a concrete array type, returns the underlying
  /// array type.
  const ArrayType *as_concrete_array() const {
    static_assert(sizeof(ArrayType) == sizeof(wasmtime_array_type_t *));
    if (ty.kind == WASMTIME_HEAPTYPE_KIND_CONCRETE_ARRAY) {
      return reinterpret_cast<const ArrayType *>(&ty.of.concrete_array);
    }
    return nullptr;
  }

  /// \brief Is this the abstract `struct` heap type?
  bool is_struct() const { return ty.kind == WASMTIME_HEAPTYPE_KIND_STRUCT; }

  /// \brief If this is a concrete struct type, returns the underlying
  /// struct type.
  const StructType *as_concrete_struct() const {
    static_assert(sizeof(StructType) == sizeof(wasmtime_struct_type_t *));
    if (ty.kind == WASMTIME_HEAPTYPE_KIND_CONCRETE_STRUCT) {
      return reinterpret_cast<const StructType *>(&ty.of.concrete_struct);
    }
    return nullptr;
  }

  /// \brief Is this the abstract `exn` heap type?
  bool is_exn() const { return ty.kind == WASMTIME_HEAPTYPE_KIND_EXN; }

  /// \brief If this is a concrete exception type, returns the underlying
  /// exception type.
  const ExnType *as_concrete_exn() const {
    static_assert(sizeof(ExnType) == sizeof(wasmtime_exn_type_t *));
    if (ty.kind == WASMTIME_HEAPTYPE_KIND_CONCRETE_EXN) {
      return reinterpret_cast<const ExnType *>(&ty.of.concrete_exn);
    }
    return nullptr;
  }

  /// \brief Is this the abstract `noexn` heap type?
  bool is_noexn() const { return ty.kind == WASMTIME_HEAPTYPE_KIND_NOEXN; }

  /// \brief Returns the underlying C API heap type.
  const wasmtime_heaptype_t *capi() const { return &ty; }
};

/// Representation of a reference type in WebAssembly.
class RefType {
  wasmtime_reftype_t ty;

public:
  /// Copy constructor.
  RefType(const RefType &other) { wasmtime_reftype_clone(&other.ty, &ty); }
  /// Copy assignment operator.
  RefType &operator=(const RefType &other) {
    wasmtime_reftype_delete(&ty);
    wasmtime_reftype_clone(&other.ty, &ty);
    return *this;
  }
  ~RefType() { wasmtime_reftype_delete(&ty); }
  /// Move constructor
  RefType(RefType &&other) {
    ty = other.ty;
    other.ty.heaptype.kind = WASMTIME_HEAPTYPE_KIND_NONE;
  }
  /// Move assignment operator
  RefType &operator=(RefType &&other) {
    wasmtime_reftype_delete(&ty);
    ty = other.ty;
    other.ty.heaptype.kind = WASMTIME_HEAPTYPE_KIND_NONE;
    return *this;
  }

  /// \brief Constructs a reference type with the given nullability and heap
  /// type.
  RefType(bool nullable, const HeapType &heaptype) {
    ty.nullable = nullable;
    wasmtime_heaptype_clone(heaptype.capi(), &ty.heaptype);
  }

  /// \brief Returns whether this reference type is nullable.
  bool nullable() const { return ty.nullable; }

  /// \brief Returns the heap type of this reference type.
  const HeapType &heaptype() const {
    static_assert(sizeof(HeapType) == sizeof(wasmtime_heaptype_t));
    return *reinterpret_cast<const HeapType *>(&ty.heaptype);
  }

  /// \brief Convenience constructor for the wasm `externref` type.
  static RefType externref() { return RefType(true, HeapType::extern_()); }

  /// \brief Convenience constructor for the wasm `nullexternref` type.
  static RefType nullexternref() { return RefType(true, HeapType::noextern()); }

  /// \brief Convenience constructor for the wasm `funcref` type.
  static RefType funcref() { return RefType(true, HeapType::func()); }

  /// \brief Convenience constructor for the wasm `nullfuncref` type.
  static RefType nullfuncref() { return RefType(true, HeapType::nofunc()); }

  /// \brief Convenience constructor for the wasm `anyref` type.
  static RefType anyref() { return RefType(true, HeapType::any()); }

  /// \brief Convenience constructor for the wasm `eqref` type.
  static RefType eqref() { return RefType(true, HeapType::eq()); }

  /// \brief Convenience constructor for the wasm `i31ref` type.
  static RefType i31ref() { return RefType(true, HeapType::i31()); }

  /// \brief Convenience constructor for the wasm `arrayref` type.
  static RefType arrayref() { return RefType(true, HeapType::array()); }

  /// \brief Convenience constructor for the wasm `structref` type.
  static RefType structref() { return RefType(true, HeapType::struct_()); }

  /// \brief Convenience constructor for the wasm `nullref` type.
  static RefType nullref() { return RefType(true, HeapType::none()); }

  /// \brief Convenience constructor for the wasm `exnref` type.
  static RefType exnref() { return RefType(true, HeapType::exn()); }

  /// \brief Convenience constructor for the wasm `nullexnref` type.
  static RefType nullexnref() { return RefType(true, HeapType::noexn()); }

  /// \brief Returns the underlying C API reference type.
  const wasmtime_reftype_t *capi() const { return &ty; }
};

inline const RefType *ValType::as_ref() const {
  static_assert(sizeof(RefType) == sizeof(wasmtime_reftype_t));
  if (wasmtime_ty.kind == WASMTIME_VALTYPE_KIND_REF) {
    return reinterpret_cast<const RefType *>(&wasmtime_ty.reftype);
  }
  return nullptr;
}

inline ValType::ValType(const Engine &engine, const RefType &ty)
    : ref(nullptr) {
  wasmtime_ty.kind = WASMTIME_VALTYPE_KIND_REF;
  wasmtime_reftype_clone(ty.capi(), &wasmtime_ty.reftype);
  ptr.reset(wasmtime_valtype_to_wasm(engine.capi(), &wasmtime_ty));
  ref = ptr.get();
}

inline ValType ValType::anyref() {
  wasmtime_valtype_t ty;
  ty.kind = WASMTIME_VALTYPE_KIND_REF;
  ty.reftype.nullable = true;
  ty.reftype.heaptype.kind = WASMTIME_HEAPTYPE_KIND_ANY;
  return ValType(&ty);
}

inline ValType ValType::exnref() {
  wasmtime_valtype_t ty;
  ty.kind = WASMTIME_VALTYPE_KIND_REF;
  ty.reftype.nullable = true;
  ty.reftype.heaptype.kind = WASMTIME_HEAPTYPE_KIND_EXN;
  return ValType(&ty);
}

/// \brief Used to print a HeapType.
inline std::ostream &operator<<(std::ostream &os, const HeapType &e) {
  const wasmtime_heaptype_t *ty = e.capi();
  switch (ty->kind) {
  case WASMTIME_HEAPTYPE_KIND_EXTERN:
    os << "extern";
    break;
  case WASMTIME_HEAPTYPE_KIND_NOEXTERN:
    os << "noextern";
    break;
  case WASMTIME_HEAPTYPE_KIND_FUNC:
    os << "func";
    break;
  case WASMTIME_HEAPTYPE_KIND_CONCRETE_FUNC:
    os << "$func";
    break;
  case WASMTIME_HEAPTYPE_KIND_NOFUNC:
    os << "nofunc";
    break;
  case WASMTIME_HEAPTYPE_KIND_ANY:
    os << "any";
    break;
  case WASMTIME_HEAPTYPE_KIND_NONE:
    os << "none";
    break;
  case WASMTIME_HEAPTYPE_KIND_EQ:
    os << "eq";
    break;
  case WASMTIME_HEAPTYPE_KIND_I31:
    os << "i31";
    break;
  case WASMTIME_HEAPTYPE_KIND_ARRAY:
    os << "array";
    break;
  case WASMTIME_HEAPTYPE_KIND_CONCRETE_ARRAY:
    os << "$array";
    break;
  case WASMTIME_HEAPTYPE_KIND_STRUCT:
    os << "struct";
    break;
  case WASMTIME_HEAPTYPE_KIND_CONCRETE_STRUCT:
    os << "$struct";
    break;
  case WASMTIME_HEAPTYPE_KIND_EXN:
    os << "exn";
    break;
  case WASMTIME_HEAPTYPE_KIND_NOEXN:
    os << "noexn";
    break;
  default:
    os << "unknown";
    break;
  }
  return os;
}

/// \brief Used to print a RefType.
inline std::ostream &operator<<(std::ostream &os, const RefType &e) {
  os << "(ref ";
  if (e.nullable())
    os << "null ";
  os << e.heaptype();
  os << ")";
  return os;
}

/// \brief Used to print a ValType.
inline std::ostream &operator<<(std::ostream &os, const ValType &e) {
  const wasmtime_valtype_t *ty = e.wasmtime_capi();
  switch (ty->kind) {
  case WASMTIME_VALTYPE_KIND_I32:
    os << "i32";
    break;
  case WASMTIME_VALTYPE_KIND_I64:
    os << "i64";
    break;
  case WASMTIME_VALTYPE_KIND_F32:
    os << "f32";
    break;
  case WASMTIME_VALTYPE_KIND_F64:
    os << "f64";
    break;
  case WASMTIME_VALTYPE_KIND_V128:
    os << "v128";
    break;
  case WASMTIME_VALTYPE_KIND_REF:
    os << *e.as_ref();
    break;
  default:
    os << "unknown";
    break;
  }
  return os;
}

}; // namespace wasmtime

#endif // WASMTIME_TYPES_VAL_HH
