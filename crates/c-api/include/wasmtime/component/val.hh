/// \file wasmtime/component/val.hh

#ifndef WASMTIME_COMPONENT_VAL_HH
#define WASMTIME_COMPONENT_VAL_HH

#include <wasmtime/conf.h>

#ifdef WASMTIME_FEATURE_COMPONENT_MODEL

#include <assert.h>
#include <memory>
#include <optional>
#include <string_view>
#include <utility>
#include <vector>
#include <wasmtime/component/types/val.hh>
#include <wasmtime/component/val.h>
#include <wasmtime/store.hh>

namespace wasmtime {
namespace component {

class Val;
class Record;

namespace detail {

/// Internal helper to convert from C to `Val`, sort of a forward-declaration
/// of `Val::from_capi` which I don't know how to otherwise forward-declare.
inline const Val *val_from_capi(const wasmtime_component_val_t *capi) {
  return reinterpret_cast<const Val *>(capi);
}

} // namespace detail

/// Internal helper macro to define ownership-semanitcs for C++ types based on
/// a C type as a single member where operations are defined in terms of
/// `transfer`, `copy`, and `destroy` functions.
#define VAL_REPR(name, raw_type)                                               \
private:                                                                       \
  using Raw = raw_type;                                                        \
  Raw raw;                                                                     \
                                                                               \
public:                                                                        \
  /**                                                                          \
   * Create a variant that takes ownership of the underlying C API variant.    \
   */                                                                          \
  explicit name(Raw &&capi) { name::transfer(std::move(capi), raw); }          \
                                                                               \
  /**                                                                          \
   * Converts the raw C API representation to this class without taking        \
   * ownership.                                                                \
   */                                                                          \
  static const name *from_capi(const Raw *capi) {                              \
    return reinterpret_cast<const name *>(capi);                               \
  }                                                                            \
                                                                               \
  /**                                                                          \
   * Converts the raw C API representation to this class without taking        \
   * ownership.                                                                \
   */                                                                          \
  static name *from_capi(Raw *capi) { return reinterpret_cast<name *>(capi); } \
                                                                               \
  /**                                                                          \
   * Converts to the raw C API representation to this class without taking     \
   * ownership.                                                                \
   */                                                                          \
  static const Raw *to_capi(const name *capi) {                                \
    return reinterpret_cast<const Raw *>(capi);                                \
  }                                                                            \
                                                                               \
  /**                                                                          \
   * Converts to the raw C API representation to this class without taking     \
   * ownership.                                                                \
   */                                                                          \
  static Raw *to_capi(name *capi) { return reinterpret_cast<Raw *>(capi); }    \
                                                                               \
  /**                                                                          \
   * \brief Copy constructor to clone `other`.                                 \
   */                                                                          \
  name(const name &other) { copy(other.raw); }                                 \
                                                                               \
  /**                                                                          \
   * \brief Copy assignment to clone from `other`.                             \
   */                                                                          \
  name &operator=(const name &other) {                                         \
    destroy();                                                                 \
    copy(other.raw);                                                           \
    return *this;                                                              \
  }                                                                            \
                                                                               \
  /**                                                                          \
   * \brief Move constructor to move the contents of `other`.                  \
   */                                                                          \
  name(name &&other) { name::transfer(std::move(other.raw), raw); }            \
                                                                               \
  /**                                                                          \
   * \brief Move assignment to move the contents of `other`.                   \
   */                                                                          \
  name &operator=(name &&other) {                                              \
    destroy();                                                                 \
    name::transfer(std::move(other.raw), raw);                                 \
    return *this;                                                              \
  }                                                                            \
                                                                               \
  ~name() { destroy(); }                                                       \
                                                                               \
  /**                                                                          \
   * \brief Returns a pointer to the underlying C API representation.          \
   */                                                                          \
  const Raw *capi() const { return &raw; }                                     \
                                                                               \
private:

/// \brief Class representing a field in a record value.
class RecordField {
  friend class Record;

  wasmtime_component_valrecord_entry_t entry;

  // This value can't be constructed or destructed, it's only used in iteration
  // of `Record`.
  RecordField() = delete;
  ~RecordField() = delete;

  static const RecordField *
  from_capi(const wasmtime_component_valrecord_entry_t *capi) {
    return reinterpret_cast<const RecordField *>(capi);
  }

public:
  /// \brief Returns the name of this record field.
  std::string_view name() const {
    return std::string_view{entry.name.data, entry.name.size};
  }

  /// \brief Returns the value of this record field.
  const Val &value() const { return *detail::val_from_capi(&entry.val); }
};

/// \brief Class representing a component model record, a list of name/value
/// pairs.
class Record {
  friend class Val;

  VAL_REPR(Record, wasmtime_component_valrecord_t);

  static void transfer(Raw &&from, Raw &to) {
    to = from;
    from.size = 0;
    from.data = nullptr;
  }

  void copy(const Raw &other) {
    wasmtime_component_valrecord_copy(&raw, &other);
  }

  void destroy() { wasmtime_component_valrecord_delete(&raw); }

public:
  /// Creates a new record from the named field pairs provided.
  Record(std::vector<std::pair<std::string_view, Val>> entries);

  /// \brief Returns the number of entries in the record.
  size_t size() const { return raw.size; }

  /// \brief Returns an iterator to the beginning of the record fields.
  const RecordField *begin() const { return RecordField::from_capi(raw.data); }

  /// \brief Returns an iterator to the end of the record fields.
  const RecordField *end() const {
    return RecordField::from_capi(raw.data + raw.size);
  }
};

/// \brief Class representing a component model list, a sequence of values.
class List {
  friend class Val;

  VAL_REPR(List, wasmtime_component_vallist_t);

  static void transfer(Raw &&from, Raw &to) {
    to = from;
    from.size = 0;
    from.data = nullptr;
  }

  void copy(const Raw &other) { wasmtime_component_vallist_copy(&raw, &other); }

  void destroy() { wasmtime_component_vallist_delete(&raw); }

public:
  /// Creates a new list from the named field pairs provided.
  List(std::vector<Val> entries);

  /// \brief Returns the number of entries in the list.
  size_t size() const { return raw.size; }

  /// \brief Returns an iterator to the beginning of the list.
  const Val *begin() const { return reinterpret_cast<const Val *>(raw.data); }

  /// \brief Returns an iterator to the end of the list.
  const Val *end() const {
    return reinterpret_cast<const Val *>(raw.data + raw.size);
  }
};

/// \brief Class representing a component model tuple.
class Tuple {
  friend class Val;

  VAL_REPR(Tuple, wasmtime_component_valtuple_t);

  static void transfer(Raw &&from, Raw &to) {
    to = from;
    from.size = 0;
    from.data = nullptr;
  }

  void copy(const Raw &other) {
    wasmtime_component_valtuple_copy(&raw, &other);
  }

  void destroy() { wasmtime_component_valtuple_delete(&raw); }

public:
  /// Creates a new tuple from the named field pairs provided.
  Tuple(std::vector<Val> entries);

  /// \brief Returns the number of entries in the tuple.
  size_t size() const { return raw.size; }

  /// \brief Returns an iterator to the beginning of the tuple.
  const Val *begin() const { return reinterpret_cast<const Val *>(raw.data); }

  /// \brief Returns an iterator to the end of the tuple.
  const Val *end() const {
    return reinterpret_cast<const Val *>(raw.data + raw.size);
  }
};

/// Class representing a component model `variant` value.
class Variant {
  friend class Val;

  VAL_REPR(Variant, wasmtime_component_valvariant_t);

  static void transfer(wasmtime_component_valvariant_t &&from,
                       wasmtime_component_valvariant_t &to) {
    to = from;
    from.discriminant.size = 0;
    from.discriminant.data = nullptr;
    from.val = nullptr;
  }

  void copy(const wasmtime_component_valvariant_t &other) {
    wasm_name_copy(&raw.discriminant, &other.discriminant);
    if (other.val) {
      wasmtime_component_val_t clone;
      wasmtime_component_val_clone(other.val, &clone);
      raw.val = wasmtime_component_val_new(&clone);
    } else {
      raw.val = nullptr;
    }
  }

  void destroy() {
    wasm_name_delete(&raw.discriminant);
    wasmtime_component_val_free(raw.val);
  }

public:
  /// Constructs a new variant value with the provided discriminant and payload.
  Variant(std::string_view discriminant, std::optional<Val> x);

  /// Returns the name of the discriminant of this value.
  std::string_view discriminant() const {
    return std::string_view(raw.discriminant.data, raw.discriminant.size);
  }

  /// Returns the optional payload value associated with this variant value.
  const Val *value() const { return detail::val_from_capi(raw.val); }
};

/// Class representing a component model `option` value.
class WitOption {
  friend class Val;

  VAL_REPR(WitOption, wasmtime_component_val_t *);

  static void transfer(Raw &&from, Raw &to) {
    to = from;
    from = nullptr;
  }

  void copy(const Raw &other) {
    if (other) {
      wasmtime_component_val_t clone;
      wasmtime_component_val_clone(other, &clone);
      raw = wasmtime_component_val_new(&clone);
    } else {
      raw = nullptr;
    }
  }

  void destroy() { wasmtime_component_val_free(raw); }

public:
  /// Constructs a new option value with the provided value.
  explicit WitOption(std::optional<Val> val);

  /// Returns the optional payload value associated with this option.
  const Val *value() const { return detail::val_from_capi(raw); }
};

/// Class representing a component model `result` value.
class WitResult {
  friend class Val;

  VAL_REPR(WitResult, wasmtime_component_valresult_t);

  static void transfer(Raw &&from, Raw &to) {
    to = from;
    from.val = nullptr;
  }

  void copy(const Raw &other) {
    raw.is_ok = other.is_ok;
    if (other.val) {
      wasmtime_component_val_t clone;
      wasmtime_component_val_clone(other.val, &clone);
      raw.val = wasmtime_component_val_new(&clone);
    } else {
      raw.val = nullptr;
    }
  }

  void destroy() {
    if (raw.val)
      wasmtime_component_val_free(raw.val);
  }

public:
  /// Constructs a new result value with the `ok` variant.
  static WitResult ok(std::optional<Val> val);

  /// Constructs a new result value with the `err` variant.
  static WitResult err(std::optional<Val> val);

  /// \brief Returns whether this result is the `ok` variant.
  bool is_ok() const { return raw.is_ok; }

  /// Returns the optional payload value associated with this result.
  const Val *payload() const { return detail::val_from_capi(raw.val); }
};

/// Class representing a component model `flags` value.
class Flag {
  friend class Flags;

  VAL_REPR(Flag, wasm_name_t);

  static void transfer(Raw &&from, Raw &to) {
    to = from;
    from.size = 0;
    from.data = nullptr;
  }

  void copy(const Raw &other) { wasm_name_copy(&raw, &other); }

  void destroy() { wasm_name_delete(&raw); }

public:
  /// Creates a new flag from the provided string.
  Flag(std::string_view name) { wasm_name_new(&raw, name.size(), name.data()); }

  /// \brief Returns the name of this flag.
  std::string_view name() const { return std::string_view{raw.data, raw.size}; }
};

/// Class representing a component model `flags` value.
class Flags {
  friend class Val;

  VAL_REPR(Flags, wasmtime_component_valflags_t);

  static void transfer(Raw &&from, Raw &to) {
    to = from;
    from.size = 0;
    from.data = nullptr;
  }

  void copy(const Raw &other) {
    wasmtime_component_valflags_copy(&raw, &other);
  }

  void destroy() { wasmtime_component_valflags_delete(&raw); }

public:
  /// Creates a new flags value from the provided flags.
  Flags(std::vector<Flag> flags) {
    wasmtime_component_valflags_new_uninit(&raw, flags.size());
    auto dst = raw.data;
    for (auto &&val : flags)
      Flag::transfer(std::move(val.raw), *dst++);
  }

  /// \brief Returns the number of flags.
  size_t size() const { return raw.size; }

  /// \brief Returns an iterator to the beginning of the flags.
  const Flag *begin() const { return reinterpret_cast<const Flag *>(raw.data); }

  /// \brief Returns an iterator to the end of the flags.
  const Flag *end() const {
    return reinterpret_cast<const Flag *>(raw.data + raw.size);
  }
};

class ResourceHost;

/// Class representing a component model `resource` value which is either a
/// guest or host-defined resource.
class ResourceAny {
  WASMTIME_CLONE_WRAPPER(ResourceAny, wasmtime_component_resource_any);

  /// \brief Returns whether this resource is owned.
  bool owned() const { return wasmtime_component_resource_any_owned(capi()); }

  /// \brief Returns the type of this resource.
  ResourceType type() const {
    wasmtime_component_resource_type_t *ty =
        wasmtime_component_resource_any_type(capi());
    return ResourceType(ty);
  }

  /// \brief Drops this resource in the component-model sense, cleaning up
  /// borrow state and executing the wasm destructor, if any.
  Result<std::monostate> drop(Store::Context cx) const {
    wasmtime_error_t *err =
        wasmtime_component_resource_any_drop(cx.capi(), capi());
    if (err)
      return Error(err);
    return std::monostate();
  }

  /// \brief Attempts to convert this resource to a host-defined resource.
  Result<ResourceHost> to_host(Store::Context cx) const;
};

/// Class representing a component model `resource` value which is a host-owned
/// resource.
class ResourceHost {
  WASMTIME_CLONE_WRAPPER(ResourceHost, wasmtime_component_resource_host);

  /// \brief Creates a new host-defined resource with the specified `owned`,
  /// `rep`, and `ty` identifiers.
  ResourceHost(bool owned, uint32_t rep, uint32_t ty)
      : ptr(wasmtime_component_resource_host_new(owned, rep, ty)) {}

  /// \brief Returns whether this resource is owned.
  bool owned() const { return wasmtime_component_resource_host_owned(capi()); }

  /// \brief Returns the "rep" identifier associated with this resource.
  uint32_t rep() const { return wasmtime_component_resource_host_rep(capi()); }

  /// \brief Returns the "type" identifier associated with this resource.
  uint32_t type() const {
    return wasmtime_component_resource_host_type(capi());
  }

  /// \brief Converts this host-defined resource into a generic resource-any.
  Result<ResourceAny> to_any(Store::Context cx) const {
    wasmtime_component_resource_any_t *out;
    wasmtime_error_t *err =
        wasmtime_component_resource_host_to_any(cx.capi(), capi(), &out);
    if (err)
      return Error(err);
    return ResourceAny(out);
  }
};

inline Result<ResourceHost> ResourceAny::to_host(Store::Context cx) const {
  wasmtime_component_resource_host_t *out;
  wasmtime_error_t *err =
      wasmtime_component_resource_any_to_host(cx.capi(), capi(), &out);
  if (err)
    return Error(err);
  return ResourceHost(out);
}

/**
 * \brief Class representing an instantiated WebAssembly component.
 */
class Val {
  friend class Variant;
  friend class WitOption;
  friend class WitResult;

  VAL_REPR(Val, wasmtime_component_val_t);

  static void transfer(Raw &&from, Raw &to) {
    to = from;
    from.kind = WASMTIME_COMPONENT_BOOL;
    from.of.boolean = false;
  }

  void copy(const Raw &other) { wasmtime_component_val_clone(&other, &raw); }

  void destroy() { wasmtime_component_val_delete(&raw); }

public:
  /// Creates a new boolean value.
  Val(bool v) {
    raw.kind = WASMTIME_COMPONENT_BOOL;
    raw.of.boolean = v;
  }

  /// Creates a new u8 value.
  Val(uint8_t v) {
    raw.kind = WASMTIME_COMPONENT_U8;
    raw.of.u8 = v;
  }

  /// Creates a new s8 value.
  Val(int8_t v) {
    raw.kind = WASMTIME_COMPONENT_S8;
    raw.of.s8 = v;
  }

  /// Creates a new u16 value.
  Val(uint16_t v) {
    raw.kind = WASMTIME_COMPONENT_U16;
    raw.of.u16 = v;
  }

  /// Creates a new s16 value.
  Val(int16_t v) {
    raw.kind = WASMTIME_COMPONENT_S16;
    raw.of.s16 = v;
  }

  /// Creates a new u32 value.
  Val(uint32_t v) {
    raw.kind = WASMTIME_COMPONENT_U32;
    raw.of.u32 = v;
  }

  /// Creates a new s32 value.
  Val(int32_t v) {
    raw.kind = WASMTIME_COMPONENT_S32;
    raw.of.s32 = v;
  }

  /// Creates a new u64 value.
  Val(uint64_t v) {
    raw.kind = WASMTIME_COMPONENT_U64;
    raw.of.u64 = v;
  }

  /// Creates a new s64 value.
  Val(int64_t v) {
    raw.kind = WASMTIME_COMPONENT_S64;
    raw.of.s64 = v;
  }

  /// Creates a new f32 value.
  Val(float v) {
    raw.kind = WASMTIME_COMPONENT_F32;
    raw.of.f32 = v;
  }

  /// Creates a new f64 value.
  Val(double v) {
    raw.kind = WASMTIME_COMPONENT_F64;
    raw.of.f64 = v;
  }

  /// Creates a new char value.
  static Val char_(uint32_t v) {
    wasmtime_component_val_t raw = {
        .kind = WASMTIME_COMPONENT_CHAR,
        .of = {.character = v},
    };
    return Val(std::move(raw));
  }

  /// Creates a new string value.
  static Val string(std::string_view v) {
    wasmtime_component_val_t raw;
    raw.kind = WASMTIME_COMPONENT_STRING;
    wasm_byte_vec_new(&raw.of.string, v.size(), v.data());
    return Val(std::move(raw));
  }

  /// Creates a new list value.
  Val(List v) {
    raw.kind = WASMTIME_COMPONENT_LIST;
    List::transfer(std::move(v.raw), raw.of.list);
  }

  /// Creates a new record value.
  Val(Record r) {
    raw.kind = WASMTIME_COMPONENT_RECORD;
    Record::transfer(std::move(r.raw), raw.of.record);
  }

  /// Creates a new tuple value.
  Val(Tuple v) {
    raw.kind = WASMTIME_COMPONENT_TUPLE;
    Tuple::transfer(std::move(v.raw), raw.of.tuple);
  }

  /// Creates a new variant value.
  Val(Variant v) {
    raw.kind = WASMTIME_COMPONENT_VARIANT;
    Variant::transfer(std::move(v.raw), raw.of.variant);
  }

  /// Creates a new option value.
  Val(WitOption v) {
    raw.kind = WASMTIME_COMPONENT_OPTION;
    WitOption::transfer(std::move(v.raw), raw.of.option);
  }

  /// Creates a new result value.
  Val(WitResult r) {
    raw.kind = WASMTIME_COMPONENT_RESULT;
    WitResult::transfer(std::move(r.raw), raw.of.result);
  }

  /// Creates a new enum value.
  static Val enum_(std::string_view discriminant) {
    wasmtime_component_val_t raw;
    raw.kind = WASMTIME_COMPONENT_ENUM;
    wasm_byte_vec_new(&raw.of.enumeration, discriminant.size(),
                      discriminant.data());
    return Val(std::move(raw));
  }

  /// Creates a new flags value.
  Val(Flags f) {
    raw.kind = WASMTIME_COMPONENT_FLAGS;
    Flags::transfer(std::move(f.raw), raw.of.flags);
  }

  /// Creates a new resource value.
  Val(ResourceAny r) {
    raw.kind = WASMTIME_COMPONENT_RESOURCE;
    raw.of.resource = r.capi_release();
  }

  /// \brief Returns whether this value is a boolean.
  bool is_bool() const { return raw.kind == WASMTIME_COMPONENT_BOOL; }

  /// \brief Returns the boolean value, only valid if `is_bool()`.
  bool get_bool() const {
    assert(is_bool());
    return raw.of.boolean;
  }

  /// \brief Returns whether this value is a u8.
  bool is_u8() const { return raw.kind == WASMTIME_COMPONENT_U8; }

  /// \brief Returns the u8 value, only valid if `is_u8()`.
  uint8_t get_u8() const {
    assert(is_u8());
    return raw.of.u8;
  }

  /// \brief Returns whether this value is a s8.
  bool is_s8() const { return raw.kind == WASMTIME_COMPONENT_S8; }

  /// \brief Returns the s8 value, only valid if `is_s8()`.
  int8_t get_s8() const {
    assert(is_s8());
    return raw.of.s8;
  }

  /// \brief Returns whether this value is a u16.
  bool is_u16() const { return raw.kind == WASMTIME_COMPONENT_U16; }

  /// \brief Returns the u16 value, only valid if `is_u16()`.
  uint16_t get_u16() const {
    assert(is_u16());
    return raw.of.u16;
  }

  /// \brief Returns whether this value is a s16.
  bool is_s16() const { return raw.kind == WASMTIME_COMPONENT_S16; }

  /// \brief Returns the s16 value, only valid if `is_s16()`.
  int16_t get_s16() const {
    assert(is_s16());
    return raw.of.s16;
  }

  /// \brief Returns whether this value is a u32.
  bool is_u32() const { return raw.kind == WASMTIME_COMPONENT_U32; }

  /// \brief Returns the u32 value, only valid if `is_u32()`.
  uint32_t get_u32() const {
    assert(is_u32());
    return raw.of.u32;
  }

  /// \brief Returns whether this value is a s32.
  bool is_s32() const { return raw.kind == WASMTIME_COMPONENT_S32; }

  /// \brief Returns the s32 value, only valid if `is_s32()`.
  int32_t get_s32() const {
    assert(is_s32());
    return raw.of.s32;
  }

  /// \brief Returns whether this value is a u64.
  bool is_u64() const { return raw.kind == WASMTIME_COMPONENT_U64; }

  /// \brief Returns the u64 value, only valid if `is_u64()`.
  uint64_t get_u64() const {
    assert(is_u64());
    return raw.of.u64;
  }

  /// \brief Returns whether this value is a s64.
  bool is_s64() const { return raw.kind == WASMTIME_COMPONENT_S64; }

  /// \brief Returns the s64 value, only valid if `is_s64()`.
  int64_t get_s64() const {
    assert(is_s64());
    return raw.of.s64;
  }

  /// \brief Returns whether this value is a f32.
  bool is_f32() const { return raw.kind == WASMTIME_COMPONENT_F32; }

  /// \brief Returns the f32 value, only valid if `is_f32()`.
  float get_f32() const {
    assert(is_f32());
    return raw.of.f32;
  }

  /// \brief Returns whether this value is a f64.
  bool is_f64() const { return raw.kind == WASMTIME_COMPONENT_F64; }

  /// \brief Returns the f64 value, only valid if `is_f64()`.
  double get_f64() const {
    assert(is_f64());
    return raw.of.f64;
  }

  /// \brief Returns whether this value is a string.
  bool is_string() const { return raw.kind == WASMTIME_COMPONENT_STRING; }

  /// \brief Returns the string value, only valid if `is_string()`.
  std::string_view get_string() const {
    assert(is_string());
    return std::string_view(raw.of.string.data, raw.of.string.size);
  }

  /// \brief Returns whether this value is a list.
  bool is_list() const { return raw.kind == WASMTIME_COMPONENT_LIST; }

  /// \brief Returns the list value, only valid if `is_list()`.
  const List &get_list() const {
    assert(is_list());
    return *List::from_capi(&raw.of.list);
  }

  /// \brief Returns whether this value is a record.
  bool is_record() const { return raw.kind == WASMTIME_COMPONENT_RECORD; }

  /// \brief Returns the record value, only valid if `is_record()`.
  const Record &get_record() const {
    assert(is_record());
    return *Record::from_capi(&raw.of.record);
  }

  /// \brief Returns whether this value is a tuple.
  bool is_tuple() const { return raw.kind == WASMTIME_COMPONENT_TUPLE; }

  /// \brief Returns the tuple value, only valid if `is_tuple()`.
  const Tuple &get_tuple() const {
    assert(is_tuple());
    return *Tuple::from_capi(&raw.of.tuple);
  }

  /// \brief Returns whether this value is a variant.
  bool is_variant() const { return raw.kind == WASMTIME_COMPONENT_VARIANT; }

  /// \brief Returns the variant value, only valid if `is_variant()`.
  const Variant &get_variant() const {
    assert(is_variant());
    return *Variant::from_capi(&raw.of.variant);
  }

  /// \brief Returns whether this value is an option.
  bool is_option() const { return raw.kind == WASMTIME_COMPONENT_OPTION; }

  /// \brief Returns the option value, only valid if `is_option()`.
  const WitOption &get_option() const {
    assert(is_option());
    return *WitOption::from_capi(&raw.of.option);
  }

  /// \brief Returns whether this value is an enum.
  bool is_enum() const { return raw.kind == WASMTIME_COMPONENT_ENUM; }

  /// \brief Returns the enum discriminant, only valid if `is_enum()`.
  std::string_view get_enum() const {
    assert(is_enum());
    return std::string_view(raw.of.enumeration.data, raw.of.enumeration.size);
  }

  /// \brief Returns whether this value is a result.
  bool is_result() const { return raw.kind == WASMTIME_COMPONENT_RESULT; }

  /// \brief Returns the result value, only valid if `is_result()`.
  const WitResult &get_result() const {
    assert(is_result());
    return *WitResult::from_capi(&raw.of.result);
  }

  /// \brief Returns whether this value is flags.
  bool is_flags() const { return raw.kind == WASMTIME_COMPONENT_FLAGS; }

  /// \brief Returns the flags value, only valid if `is_flags()`.
  const Flags &get_flags() const {
    assert(is_flags());
    return *Flags::from_capi(&raw.of.flags);
  }

  /// \brief Returns whether this value is a resource.
  bool is_resource() const { return raw.kind == WASMTIME_COMPONENT_RESOURCE; }

  /// \brief Returns the flags value, only valid if `is_flags()`.
  const ResourceAny &get_resource() const {
    assert(is_resource());
    return *ResourceAny::from_capi(&raw.of.resource);
  }
};

#undef VAL_REPR

inline Record::Record(std::vector<std::pair<std::string_view, Val>> entries) {
  wasmtime_component_valrecord_new_uninit(&raw, entries.size());
  auto dst = raw.data;
  for (auto &&[name, val] : entries) {
    wasm_byte_vec_new(&dst->name, name.size(), name.data());
    new (&dst->val) Val(std::move(val));
    dst++;
  }
}

inline List::List(std::vector<Val> values) {
  wasmtime_component_vallist_new_uninit(&raw, values.size());
  auto dst = raw.data;
  for (auto &&val : values)
    new (dst++) Val(std::move(val));
}

inline Tuple::Tuple(std::vector<Val> values) {
  wasmtime_component_valtuple_new_uninit(&raw, values.size());
  auto dst = raw.data;
  for (auto &&val : values)
    new (dst++) Val(std::move(val));
}

inline Variant::Variant(std::string_view discriminant, std::optional<Val> x) {
  wasm_name_new(&raw.discriminant, discriminant.size(), discriminant.data());
  if (x) {
    raw.val = wasmtime_component_val_new(&x->raw);
  } else {
    raw.val = nullptr;
  }
}

inline WitOption::WitOption(std::optional<Val> v) {
  if (v) {
    raw = wasmtime_component_val_new(&v->raw);
  } else {
    raw = nullptr;
  }
}

inline WitResult WitResult::ok(std::optional<Val> v) {
  wasmtime_component_valresult_t raw;
  raw.is_ok = true;
  if (v) {
    raw.val = wasmtime_component_val_new(&v->raw);
  } else {
    raw.val = nullptr;
  }
  return WitResult(std::move(raw));
}

inline WitResult WitResult::err(std::optional<Val> v) {
  wasmtime_component_valresult_t raw;
  raw.is_ok = false;
  if (v) {
    raw.val = wasmtime_component_val_new(&v->raw);
  } else {
    raw.val = nullptr;
  }
  return WitResult(std::move(raw));
}

} // namespace component
} // namespace wasmtime

#endif // WASMTIME_FEATURE_COMPONENT_MODEL

#endif // WASMTIME_COMPONENT_VAL_H
