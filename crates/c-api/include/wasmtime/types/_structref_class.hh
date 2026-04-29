#ifndef WASMTIME_TYPES_STRUCTREF_CLASS_HH
#define WASMTIME_TYPES_STRUCTREF_CLASS_HH

#include <wasmtime/conf.h>

#ifdef WASMTIME_FEATURE_GC

#include <memory>
#include <vector>
#include <wasmtime/engine.hh>
#include <wasmtime/types/_val_class.hh>
#include <wasmtime/types/structref.h>

namespace wasmtime {

/// \brief Storage type for a struct field or array element.
class StorageType {
  wasmtime_storage_type_t ty;

  StorageType(wasmtime_storage_type_kind_t kind) { ty.kind = kind; }

public:
  /// Copy constructor.
  StorageType(const StorageType &other) {
    wasmtime_storage_type_clone(&other.ty, &ty);
  }
  /// Copy assignment operator.
  StorageType &operator=(const StorageType &other) {
    wasmtime_storage_type_delete(&ty);
    wasmtime_storage_type_clone(&other.ty, &ty);
    return *this;
  }
  ~StorageType() { wasmtime_storage_type_delete(&ty); }
  /// Move constructor
  StorageType(StorageType &&other) {
    ty = other.ty;
    other.ty.kind = WASMTIME_STORAGE_TYPE_KIND_I8;
  }
  /// Move assignment operator
  StorageType &operator=(StorageType &&other) {
    wasmtime_storage_type_delete(&ty);
    ty = other.ty;
    other.ty.kind = WASMTIME_STORAGE_TYPE_KIND_I8;
    return *this;
  }

  /// \brief Constructs a storage type from a value type.
  StorageType(const ValType &ty);

  /// \brief Constructs a storage type for an 8-bit integer.
  static StorageType i8() { return StorageType(WASMTIME_STORAGE_TYPE_KIND_I8); }

  /// \brief Constructs a storage type for a 16-bit integer.
  static StorageType i16() {
    return StorageType(WASMTIME_STORAGE_TYPE_KIND_I16);
  }

  /// \brief Returns the underlying C API storage type.
  const wasmtime_storage_type_t *capi() const { return &ty; }

  /// \brief Returns whether this storage type is an 8-bit integer.
  bool is_i8() const { return ty.kind == WASMTIME_STORAGE_TYPE_KIND_I8; }

  /// \brief Returns whether this storage type is a 16-bit integer.
  bool is_i16() const { return ty.kind == WASMTIME_STORAGE_TYPE_KIND_I16; }

  /// \brief If this storage type is a value type, returns the underlying value
  /// type.
  std::optional<ValType::Ref> as_valtype() const {
    if (ty.kind == WASMTIME_STORAGE_TYPE_KIND_VALTYPE)
      return ty.valtype;
    return std::nullopt;
  }
};

/**
 * \brief Describes the storage type and mutability of a struct field or array
 * element.
 */
class FieldType {
  wasmtime_field_type_t ty;

public:
  /// \brief Constructs a field type from a C API field type.
  FieldType(wasmtime_field_type ty) : ty(ty) {}

  /// Copy constructor.
  FieldType(const FieldType &other) {
    wasmtime_field_type_clone(&other.ty, &ty);
  }
  /// Copy assignment operator.
  FieldType &operator=(const FieldType &other) {
    wasmtime_field_type_delete(&ty);
    wasmtime_field_type_clone(&other.ty, &ty);
    return *this;
  }
  ~FieldType() { wasmtime_field_type_delete(&ty); }
  /// Move constructor
  FieldType(FieldType &&other) {
    ty = other.ty;
    other.ty.storage.kind = WASMTIME_STORAGE_TYPE_KIND_I8;
  }
  /// Move assignment operator
  FieldType &operator=(FieldType &&other) {
    wasmtime_field_type_delete(&ty);
    ty = other.ty;
    other.ty.storage.kind = WASMTIME_STORAGE_TYPE_KIND_I8;
    return *this;
  }

  /// \brief Constructs a field type with the given mutability and storage type.
  FieldType(bool is_mutable, const StorageType &ty) {
    this->ty.mutable_ = is_mutable;
    wasmtime_storage_type_clone(ty.capi(), &this->ty.storage);
  }

  /// \brief Constructs a mutable field type with the given storage type.
  static FieldType mut_(const StorageType &ty) { return FieldType(true, ty); }

  /// \brief Constructs an immutable field type with the given storage type.
  static FieldType const_(const StorageType &ty) {
    return FieldType(false, ty);
  }

  /// \brief Returns whether this field type is mutable.
  bool is_mutable() const { return ty.mutable_; }

  /// \brief Returns the storage type of this field type.
  const StorageType &storage_type() const {
    static_assert(sizeof(StorageType) == sizeof(wasmtime_storage_type_t));
    return *reinterpret_cast<const StorageType *>(&ty.storage);
  }

  /// \brief Returns the underlying C API field type.
  const wasmtime_field_type_t *capi() const { return &ty; }
};

/**
 * \brief Owned handle to a WebAssembly struct type definition.
 *
 * Create with StructType::create, then use with StructRefPre to allocate
 * instances.
 */
class StructType {
#define wasmtime_struct_type_clone wasmtime_struct_type_copy
  WASMTIME_CLONE_WRAPPER(StructType, wasmtime_struct_type)
#undef wasmtime_struct_type_clone

  /// Create a new struct type with the given fields.
  StructType(const Engine &engine, const std::vector<FieldType> &fields)
      : ptr(wasmtime_struct_type_new(
            engine.capi(),
            reinterpret_cast<const wasmtime_field_type_t *>(fields.data()),
            fields.size()))

  {
    static_assert(sizeof(FieldType) == sizeof(wasmtime_field_type_t));
  }

  size_t num_fields() const {
    return wasmtime_struct_type_num_fields(ptr.get());
  }

  std::optional<FieldType> field(size_t index) const {
    if (index >= wasmtime_struct_type_num_fields(ptr.get()))
      return std::nullopt;
    wasmtime_field_type_t ty;
    wasmtime_struct_type_field(ptr.get(), index, &ty);
    return FieldType(ty);
  }
};

} // namespace wasmtime

#endif // WASMTIME_FEATURE_GC

#endif // WASMTIME_TYPES_STRUCTREF_CLASS_HH
