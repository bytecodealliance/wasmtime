/**
 * \file wasmtime/component/types/val.hh
 */

#ifndef WASMTIME_COMPONENT_TYPES_VAL_HH
#define WASMTIME_COMPONENT_TYPES_VAL_HH

#include <wasmtime/conf.h>

#ifdef WASMTIME_FEATURE_COMPONENT_MODEL

#include <memory>
#include <optional>
#include <string_view>
#include <wasmtime/component/types/val.h>
#include <wasmtime/helpers.hh>

namespace wasmtime {
namespace component {

class ValType;

/**
 * \brief Represents a component list type.
 */
class ListType {
  WASMTIME_CLONE_EQUAL_WRAPPER(ListType, wasmtime_component_list_type);

  /// Returns the element type of this list type.
  ValType element() const;
};

/**
 * \brief Represents a component record type.
 */
class RecordType {
  WASMTIME_CLONE_EQUAL_WRAPPER(RecordType, wasmtime_component_record_type);

  /// Returns the number of fields in this record type.
  size_t field_count() const {
    return wasmtime_component_record_type_field_count(ptr.get());
  }

  /// Retrieves the nth field.
  std::optional<std::pair<std::string_view, ValType>>
  field_nth(size_t nth) const;
};

/**
 * \brief Represents a component tuple type.
 */
class TupleType {
  WASMTIME_CLONE_EQUAL_WRAPPER(TupleType, wasmtime_component_tuple_type);

  /// Returns the number of types in this tuple type.
  size_t types_count() const {
    return wasmtime_component_tuple_type_types_count(ptr.get());
  }

  /// Retrieves the nth type.
  std::optional<ValType> types_nth(size_t nth) const;
};

/**
 * \brief Represents a component variant type.
 */
class VariantType {
  WASMTIME_CLONE_EQUAL_WRAPPER(VariantType, wasmtime_component_variant_type);

  /// Returns the number of cases in this variant type.
  size_t case_count() const {
    return wasmtime_component_variant_type_case_count(ptr.get());
  }

  /// Retrieves the nth case.
  std::optional<std::pair<std::string_view, std::optional<ValType>>>
  case_nth(size_t nth) const;
};

/**
 * \brief Represents a component enum type.
 */
class EnumType {
  WASMTIME_CLONE_EQUAL_WRAPPER(EnumType, wasmtime_component_enum_type);

  /// Returns the number of names in this enum type.
  size_t names_count() const {
    return wasmtime_component_enum_type_names_count(ptr.get());
  }

  /// Retrieves the nth name.
  std::optional<std::string_view> names_nth(size_t nth) const {
    const char *name_ptr = nullptr;
    size_t name_len = 0;
    if (wasmtime_component_enum_type_names_nth(ptr.get(), nth, &name_ptr,
                                               &name_len)) {
      return std::string_view(name_ptr, name_len);
    }
    return std::nullopt;
  }
};

/**
 * \brief Represents a component option type.
 */
class OptionType {
  WASMTIME_CLONE_EQUAL_WRAPPER(OptionType, wasmtime_component_option_type);

  /// Returns the inner type of this option type.
  ValType ty() const;
};

/**
 * \brief Represents a component result type.
 */
class ResultType {
  WASMTIME_CLONE_EQUAL_WRAPPER(ResultType, wasmtime_component_result_type);

  /// Returns the ok type of this result type, if any.
  std::optional<ValType> ok() const;

  /// Returns the err type of this result type, if any.
  std::optional<ValType> err() const;
};

/**
 * \brief Represents a component flags type.
 */
class FlagsType {
  WASMTIME_CLONE_EQUAL_WRAPPER(FlagsType, wasmtime_component_flags_type);

  /// Returns the number of names in this flags type.
  size_t names_count() const {
    return wasmtime_component_flags_type_names_count(ptr.get());
  }

  /// Retrieves the nth name.
  std::optional<std::string_view> names_nth(size_t nth) const {
    const char *name_ptr = nullptr;
    size_t name_len = 0;
    if (wasmtime_component_flags_type_names_nth(ptr.get(), nth, &name_ptr,
                                                &name_len)) {
      return std::string_view(name_ptr, name_len);
    }
    return std::nullopt;
  }
};

/// Class representing a component model `resource` value which is either a
/// guest or host-defined resource.
class ResourceType {
  WASMTIME_CLONE_EQUAL_WRAPPER(ResourceType, wasmtime_component_resource_type);

public:
  /// \brief Creates a new host resource type with the specified `ty`
  /// identifier.
  explicit ResourceType(uint32_t ty)
      : ptr(wasmtime_component_resource_type_new_host(ty)) {}
};

/**
 * \brief Represents a component future type.
 */
class FutureType {
  WASMTIME_CLONE_EQUAL_WRAPPER(FutureType, wasmtime_component_future_type);

  /// Returns the inner type of this future type, if any.
  std::optional<ValType> ty() const;
};

/**
 * \brief Represents a component stream type.
 */
class StreamType {
  WASMTIME_CLONE_EQUAL_WRAPPER(StreamType, wasmtime_component_stream_type);

  /// Returns the inner type of this stream type, if any.
  std::optional<ValType> ty() const;
};

/**
 * \brief Represents a component value type.
 */
class ValType {
  wasmtime_component_valtype_t ty;

  static ValType new_kind(wasmtime_component_valtype_kind_t kind) {
    wasmtime_component_valtype_t ty;
    ty.kind = kind;
    return ValType(std::move(ty));
  }

public:
  /// Creates a component value type from the raw C API representation.
  explicit ValType(wasmtime_component_valtype_t &&ty) {
    this->ty = ty;
    ty.kind = WASMTIME_COMPONENT_VALTYPE_BOOL;
  }

  /// Copies another type into this one.
  ValType(const ValType &other) {
    wasmtime_component_valtype_clone(&other.ty, &ty);
  }

  /// Copies another type into this one.
  ValType &operator=(const ValType &other) {
    wasmtime_component_valtype_delete(&ty);
    wasmtime_component_valtype_clone(&other.ty, &ty);
    return *this;
  }

  /// Moves another type into this one.
  ValType(ValType &&other) : ty(other.ty) {
    other.ty.kind = WASMTIME_COMPONENT_VALTYPE_BOOL;
  }

  /// Moves another type into this one.
  ValType &operator=(ValType &&other) {
    wasmtime_component_valtype_delete(&ty);
    ty = other.ty;
    other.ty.kind = WASMTIME_COMPONENT_VALTYPE_BOOL;
    return *this;
  }

  ~ValType() { wasmtime_component_valtype_delete(&ty); }

  /**
   * Converts the raw C API representation to this class without taking
   * ownership.
   */
  static const ValType *from_capi(const wasmtime_component_valtype_t *capi) {
    static_assert(sizeof(ValType) == sizeof(wasmtime_component_valtype_t));
    return reinterpret_cast<const ValType *>(capi);
  }

  /// \brief Compares two types to see if they're the same.
  bool operator==(const ValType &other) const {
    return wasmtime_component_valtype_equal(&ty, &other.ty);
  }

  /// \brief Compares two types to see if they're different.
  bool operator!=(const ValType &other) const { return !(*this == other); }

  /// Creates a bool value type.
  static ValType new_bool() {
    return ValType::new_kind(WASMTIME_COMPONENT_VALTYPE_BOOL);
  }

  /// Creates an s8 value type.
  static ValType new_s8() {
    return ValType::new_kind(WASMTIME_COMPONENT_VALTYPE_S8);
  }

  /// Creates an s16 value type.
  static ValType new_s16() {
    return ValType::new_kind(WASMTIME_COMPONENT_VALTYPE_S16);
  }

  /// Creates an s32 value type.
  static ValType new_s32() {
    return ValType::new_kind(WASMTIME_COMPONENT_VALTYPE_S32);
  }

  /// Creates an s64 value type.
  static ValType new_s64() {
    return ValType::new_kind(WASMTIME_COMPONENT_VALTYPE_S64);
  }

  /// Creates a u8 value type.
  static ValType new_u8() {
    return ValType::new_kind(WASMTIME_COMPONENT_VALTYPE_U8);
  }

  /// Creates a u16 value type.
  static ValType new_u16() {
    return ValType::new_kind(WASMTIME_COMPONENT_VALTYPE_U16);
  }

  /// Creates a u32 value type.
  static ValType new_u32() {
    return ValType::new_kind(WASMTIME_COMPONENT_VALTYPE_U32);
  }

  /// Creates a u64 value type.
  static ValType new_u64() {
    return ValType::new_kind(WASMTIME_COMPONENT_VALTYPE_U64);
  }

  /// Creates an f32 value type.
  static ValType new_f32() {
    return ValType::new_kind(WASMTIME_COMPONENT_VALTYPE_F32);
  }

  /// Creates an f64 value type.
  static ValType new_f64() {
    return ValType::new_kind(WASMTIME_COMPONENT_VALTYPE_F64);
  }

  /// Creates a char value type.
  static ValType new_char() {
    return ValType::new_kind(WASMTIME_COMPONENT_VALTYPE_CHAR);
  }

  /// Creates a string value type.
  static ValType new_string() {
    return ValType::new_kind(WASMTIME_COMPONENT_VALTYPE_STRING);
  }

  /// Creates a list value type.
  ValType(ListType list) {
    ty.kind = WASMTIME_COMPONENT_VALTYPE_LIST;
    ty.of.list = list.capi_release();
  }

  /// Creates a record value type.
  ValType(RecordType record) {
    ty.kind = WASMTIME_COMPONENT_VALTYPE_RECORD;
    ty.of.record = record.capi_release();
  }

  /// Creates a tuple value type.
  ValType(TupleType tuple) {
    ty.kind = WASMTIME_COMPONENT_VALTYPE_TUPLE;
    ty.of.tuple = tuple.capi_release();
  }

  /// Creates a variant value type.
  ValType(VariantType variant) {
    ty.kind = WASMTIME_COMPONENT_VALTYPE_VARIANT;
    ty.of.variant = variant.capi_release();
  }

  /// Creates an enum value type.
  ValType(EnumType enum_) {
    ty.kind = WASMTIME_COMPONENT_VALTYPE_ENUM;
    ty.of.enum_ = enum_.capi_release();
  }

  /// Creates an option value type.
  ValType(OptionType option) {
    ty.kind = WASMTIME_COMPONENT_VALTYPE_OPTION;
    ty.of.option = option.capi_release();
  }

  /// Creates a result value type.
  ValType(ResultType result) {
    ty.kind = WASMTIME_COMPONENT_VALTYPE_RESULT;
    ty.of.result = result.capi_release();
  }

  /// Creates a flags value type.
  ValType(FlagsType flags) {
    ty.kind = WASMTIME_COMPONENT_VALTYPE_FLAGS;
    ty.of.flags = flags.capi_release();
  }

  /// Creates an own value type.
  static ValType new_own(ResourceType own) {
    wasmtime_component_valtype_t ty;
    ty.kind = WASMTIME_COMPONENT_VALTYPE_OWN;
    ty.of.own = own.capi_release();
    return ValType(std::move(ty));
  }

  /// Creates an borrow value type.
  static ValType new_borrow(ResourceType borrow) {
    wasmtime_component_valtype_t ty;
    ty.kind = WASMTIME_COMPONENT_VALTYPE_BORROW;
    ty.of.borrow = borrow.capi_release();
    return ValType(std::move(ty));
  }

  /// Creates a future value type.
  ValType(FutureType future) {
    ty.kind = WASMTIME_COMPONENT_VALTYPE_FUTURE;
    ty.of.future = future.capi_release();
  }

  /// Creates a stream value type.
  ValType(StreamType stream) {
    ty.kind = WASMTIME_COMPONENT_VALTYPE_STREAM;
    ty.of.stream = stream.capi_release();
  }

  /// Returns the kind of this value type.
  wasmtime_component_valtype_kind_t kind() const { return ty.kind; }

  /// Returns true if this is a bool type.
  bool is_bool() const { return ty.kind == WASMTIME_COMPONENT_VALTYPE_BOOL; }

  /// Returns true if this is an s8 type.
  bool is_s8() const { return ty.kind == WASMTIME_COMPONENT_VALTYPE_S8; }

  /// Returns true if this is an s16 type.
  bool is_s16() const { return ty.kind == WASMTIME_COMPONENT_VALTYPE_S16; }

  /// Returns true if this is an s32 type.
  bool is_s32() const { return ty.kind == WASMTIME_COMPONENT_VALTYPE_S32; }

  /// Returns true if this is an s64 type.
  bool is_s64() const { return ty.kind == WASMTIME_COMPONENT_VALTYPE_S64; }

  /// Returns true if this is a u8 type.
  bool is_u8() const { return ty.kind == WASMTIME_COMPONENT_VALTYPE_U8; }

  /// Returns true if this is a u16 type.
  bool is_u16() const { return ty.kind == WASMTIME_COMPONENT_VALTYPE_U16; }

  /// Returns true if this is a u32 type.
  bool is_u32() const { return ty.kind == WASMTIME_COMPONENT_VALTYPE_U32; }

  /// Returns true if this is a u64 type.
  bool is_u64() const { return ty.kind == WASMTIME_COMPONENT_VALTYPE_U64; }

  /// Returns true if this is an f32 type.
  bool is_f32() const { return ty.kind == WASMTIME_COMPONENT_VALTYPE_F32; }

  /// Returns true if this is an f64 type.
  bool is_f64() const { return ty.kind == WASMTIME_COMPONENT_VALTYPE_F64; }

  /// Returns true if this is a char type.
  bool is_char() const { return ty.kind == WASMTIME_COMPONENT_VALTYPE_CHAR; }

  /// Returns true if this is a string type.
  bool is_string() const {
    return ty.kind == WASMTIME_COMPONENT_VALTYPE_STRING;
  }

  /// Returns true if this is a list type.
  bool is_list() const { return ty.kind == WASMTIME_COMPONENT_VALTYPE_LIST; }

  /// Returns true if this is a record type.
  bool is_record() const {
    return ty.kind == WASMTIME_COMPONENT_VALTYPE_RECORD;
  }

  /// Returns true if this is a tuple type.
  bool is_tuple() const { return ty.kind == WASMTIME_COMPONENT_VALTYPE_TUPLE; }

  /// Returns true if this is a variant type.
  bool is_variant() const {
    return ty.kind == WASMTIME_COMPONENT_VALTYPE_VARIANT;
  }

  /// Returns true if this is an enum type.
  bool is_enum() const { return ty.kind == WASMTIME_COMPONENT_VALTYPE_ENUM; }

  /// Returns true if this is an option type.
  bool is_option() const {
    return ty.kind == WASMTIME_COMPONENT_VALTYPE_OPTION;
  }

  /// Returns true if this is a result type.
  bool is_result() const {
    return ty.kind == WASMTIME_COMPONENT_VALTYPE_RESULT;
  }

  /// Returns true if this is a flags type.
  bool is_flags() const { return ty.kind == WASMTIME_COMPONENT_VALTYPE_FLAGS; }

  /// Returns true if this is an own type.
  bool is_own() const { return ty.kind == WASMTIME_COMPONENT_VALTYPE_OWN; }

  /// Returns true if this is a borrow type.
  bool is_borrow() const {
    return ty.kind == WASMTIME_COMPONENT_VALTYPE_BORROW;
  }

  /// Returns true if this is a future type.
  bool is_future() const {
    return ty.kind == WASMTIME_COMPONENT_VALTYPE_FUTURE;
  }

  /// Returns true if this is a stream type.
  bool is_stream() const {
    return ty.kind == WASMTIME_COMPONENT_VALTYPE_STREAM;
  }

  /// Returns true if this is an error context type.
  bool is_error_context() const {
    return ty.kind == WASMTIME_COMPONENT_VALTYPE_ERROR_CONTEXT;
  }

  /// Returns the list type, asserting that this is indeed a list.
  const ListType &list() const {
    assert(is_list());
    return *ListType::from_capi(&ty.of.list);
  }

  /// Returns the record type, asserting that this is indeed a record.
  const RecordType &record() const {
    assert(is_record());
    return *RecordType::from_capi(&ty.of.record);
  }

  /// Returns the tuple type, asserting that this is indeed a tuple.
  const TupleType &tuple() const {
    assert(is_tuple());
    return *TupleType::from_capi(&ty.of.tuple);
  }

  /// Returns the variant type, asserting that this is indeed a variant.
  const VariantType &variant() const {
    assert(is_variant());
    return *VariantType::from_capi(&ty.of.variant);
  }

  /// Returns the enum type, asserting that this is indeed a enum.
  const EnumType &enum_() const {
    assert(is_enum());
    return *EnumType::from_capi(&ty.of.enum_);
  }

  /// Returns the option type, asserting that this is indeed a option.
  const OptionType &option() const {
    assert(is_option());
    return *OptionType::from_capi(&ty.of.option);
  }

  /// Returns the result type, asserting that this is indeed a result.
  const ResultType &result() const {
    assert(is_result());
    return *ResultType::from_capi(&ty.of.result);
  }

  /// Returns the flags type, asserting that this is indeed a flags.
  const FlagsType &flags() const {
    assert(is_flags());
    return *FlagsType::from_capi(&ty.of.flags);
  }

  /// Returns the own type, asserting that this is indeed a own.
  const ResourceType &own() const {
    assert(is_own());
    return *ResourceType::from_capi(&ty.of.own);
  }

  /// Returns the borrow type, asserting that this is indeed a borrow.
  const ResourceType &borrow() const {
    assert(is_borrow());
    return *ResourceType::from_capi(&ty.of.borrow);
  }

  /// Returns the future type, asserting that this is indeed a future.
  const FutureType &future() const {
    assert(is_future());
    return *FutureType::from_capi(&ty.of.future);
  }

  /// Returns the stream type, asserting that this is indeed a stream.
  const StreamType &stream() const {
    assert(is_stream());
    return *StreamType::from_capi(&ty.of.stream);
  }

  /// \brief Returns the underlying C API pointer.
  const wasmtime_component_valtype_t *capi() const { return &ty; }
  /// \brief Returns the underlying C API pointer.
  wasmtime_component_valtype_t *capi() { return &ty; }
};

inline ValType ListType::element() const {
  wasmtime_component_valtype_t type_ret;
  wasmtime_component_list_type_element(ptr.get(), &type_ret);
  return ValType(std::move(type_ret));
}

inline std::optional<std::pair<std::string_view, ValType>>
RecordType::field_nth(size_t nth) const {
  const char *name_ptr = nullptr;
  size_t name_len = 0;
  wasmtime_component_valtype_t type_ret;
  if (wasmtime_component_record_type_field_nth(ptr.get(), nth, &name_ptr,
                                               &name_len, &type_ret)) {
    return std::make_pair(std::string_view(name_ptr, name_len),
                          ValType(std::move(type_ret)));
  }
  return std::nullopt;
}

inline std::optional<ValType> TupleType::types_nth(size_t nth) const {
  wasmtime_component_valtype_t type_ret;
  if (wasmtime_component_tuple_type_types_nth(ptr.get(), nth, &type_ret)) {
    return ValType(std::move(type_ret));
  }
  return std::nullopt;
}

inline std::optional<std::pair<std::string_view, std::optional<ValType>>>
VariantType::case_nth(size_t nth) const {
  const char *name_ptr = nullptr;
  size_t name_len = 0;
  bool has_payload = false;
  wasmtime_component_valtype_t payload_ret;
  if (!wasmtime_component_variant_type_case_nth(
          ptr.get(), nth, &name_ptr, &name_len, &has_payload, &payload_ret)) {
    return std::nullopt;
  }
  return std::make_pair(
      std::string_view(name_ptr, name_len),
      has_payload ? std::optional<ValType>(ValType(std::move(payload_ret)))
                  : std::nullopt);
}

inline ValType OptionType::ty() const {
  wasmtime_component_valtype_t type_ret;
  wasmtime_component_option_type_ty(ptr.get(), &type_ret);
  return ValType(std::move(type_ret));
}

inline std::optional<ValType> ResultType::ok() const {
  wasmtime_component_valtype_t type_ret;
  if (wasmtime_component_result_type_ok(ptr.get(), &type_ret)) {
    return ValType(std::move(type_ret));
  }
  return std::nullopt;
}

inline std::optional<ValType> ResultType::err() const {
  wasmtime_component_valtype_t type_ret;
  if (wasmtime_component_result_type_err(ptr.get(), &type_ret)) {
    return ValType(std::move(type_ret));
  }
  return std::nullopt;
}

inline std::optional<ValType> FutureType::ty() const {
  wasmtime_component_valtype_t type_ret;
  if (wasmtime_component_future_type_ty(ptr.get(), &type_ret)) {
    return ValType(std::move(type_ret));
  }
  return std::nullopt;
}

inline std::optional<ValType> StreamType::ty() const {
  wasmtime_component_valtype_t type_ret;
  if (wasmtime_component_stream_type_ty(ptr.get(), &type_ret)) {
    return ValType(std::move(type_ret));
  }
  return std::nullopt;
}

} // namespace component
} // namespace wasmtime

#endif // WASMTIME_FEATURE_COMPONENT_MODEL

#endif // WASMTIME_COMPONENT_TYPES_VAL_HH
