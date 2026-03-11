/**
 * \file wasmtime/types/tag.hh
 */

#ifndef WASMTIME_TYPES_TAG_HH
#define WASMTIME_TYPES_TAG_HH

#include <wasmtime/tag.h>
#include <wasmtime/types/val.hh>

namespace wasmtime {

/**
 * \brief Type information for a WebAssembly exception tag.
 *
 * An exception tag is described by the parameter types of its associated
 * function type (the exception payload). Tags have no results.
 */
class TagType {
  struct deleter {
    void operator()(wasm_tagtype_t *p) const { wasm_tagtype_delete(p); }
  };

  std::unique_ptr<wasm_tagtype_t, deleter> ptr;

public:
  /// Non-owning reference to a `TagType`, must not be used after the original
  /// owner is deleted.
  class Ref {
    friend class TagType;

    const wasm_tagtype_t *ptr;

  public:
    /// Creates a reference from the raw underlying C API representation.
    Ref(const wasm_tagtype_t *ptr) : ptr(ptr) {}
    /// Creates a reference to the provided `TagType`.
    Ref(const TagType &ty) : Ref(ty.ptr.get()) {}

    /// Returns the payload (parameter) types of this exception tag.
    ValType::ListRef params() const { return wasm_tagtype_params(ptr); }
  };

private:
  Ref ref;
  TagType(wasm_tagtype_t *ptr) : ptr(ptr), ref(ptr) {}

public:
  /// Creates a new tag type with the given exception payload types.
  TagType(std::initializer_list<ValType> params) : ptr(nullptr), ref(nullptr) {
    wasm_valtype_vec_t ps;
    wasm_valtype_vec_new_uninitialized(&ps, params.size());
    size_t i = 0;
    for (auto p : params) {
      ps.data[i++] = p.ptr.release(); // NOLINT
    }
    ptr.reset(wasm_tagtype_new(&ps));
    ref = ptr.get();
  }

  /// Copies a reference into a uniquely owned tag type.
  TagType(Ref other) : TagType(wasm_tagtype_copy(other.ptr)) {}
  /// Copies another tag type into this one.
  TagType(const TagType &other) : TagType(wasm_tagtype_copy(other.ptr.get())) {}
  /// Copies another tag type into this one.
  TagType &operator=(const TagType &other) {
    ptr.reset(wasm_tagtype_copy(other.ptr.get()));
    ref = ptr.get();
    return *this;
  }
  ~TagType() = default;
  /// Moves the tag type resources from another type to this one.
  TagType(TagType &&other) = default;
  /// Moves the tag type resources from another type to this one.
  TagType &operator=(TagType &&other) = default;

  /// \brief Returns the underlying `Ref`, a non-owning reference pointing to
  /// this instance.
  Ref *operator->() { return &ref; }
  /// \brief Returns the underlying `Ref`, a non-owning reference pointing to
  /// this instance.
  Ref *operator*() { return &ref; }
};

}; // namespace wasmtime

#endif // WASMTIME_TYPES_TAG_HH
