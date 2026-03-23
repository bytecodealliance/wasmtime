/**
 * \file wasmtime/types/tag.hh
 */

#ifndef WASMTIME_TYPES_TAG_HH
#define WASMTIME_TYPES_TAG_HH

#include <memory>
#include <wasm.h>
#include <wasmtime/types/func.hh>

namespace wasmtime {

/**
 * \brief Type information for a WebAssembly exception tag.
 *
 * A tag type is described by a function type whose parameters are the exception
 * payload types (and whose results are empty under the current proposal).
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

    /// Returns the function type describing the exception payload of this tag.
    ///
    /// The caller owns the returned `FuncType`.
    FuncType functype() const {
      return FuncType(wasm_functype_copy(wasm_tagtype_functype(ptr)));
    }
  };

private:
  Ref ref;
  TagType(wasm_tagtype_t *ptr) : ptr(ptr), ref(ptr) {}

public:
  /// Creates a new tag type from the given function type.
  /// Copies `functype` so the original is not consumed.
  explicit TagType(const FuncType &functype)
      : TagType(wasm_tagtype_new(wasm_functype_copy(functype.ptr.get()))) {}

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
