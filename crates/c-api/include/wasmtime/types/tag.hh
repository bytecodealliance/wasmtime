/**
 * \file wasmtime/types/tag.hh
 */

#ifndef WASMTIME_TYPES_TAG_HH
#define WASMTIME_TYPES_TAG_HH

#include <memory>
#include <wasmtime/engine.hh>
#include <wasmtime/tag.h>
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
    void operator()(wasmtime_tagtype_t *p) const { wasmtime_tagtype_delete(p); }
  };

  std::unique_ptr<wasmtime_tagtype_t, deleter> ptr;

public:
  /// Non-owning reference to a `TagType`, must not be used after the original
  /// owner is deleted.
  class Ref {
    friend class TagType;
    const wasmtime_tagtype_t *ptr;

  public:
    /// Creates a reference from the raw underlying C API representation.
    Ref(const wasmtime_tagtype_t *ptr) : ptr(ptr) {}
    /// Creates a reference to the provided `TagType`.
    Ref(const TagType &ty) : Ref(ty.ptr.get()) {}

    /// Returns the function type describing the exception payload of this tag.
    ///
    /// The caller owns the returned `FuncType`.
    FuncType functype() const {
      return FuncType(wasmtime_tagtype_functype(ptr));
    }
  };

private:
  Ref ref;
  TagType(wasmtime_tagtype_t *ptr) : ptr(ptr), ref(ptr) {}

public:
  /// Creates a new tag type from the given function type and engine.
  TagType(Engine &engine, FuncType &functype)
      : ptr(wasmtime_tagtype_new(engine.capi(), functype.ptr.get())),
        ref(ptr.get()) {}

  /// Copies a reference into a uniquely owned tag type.
  TagType(Ref other) : TagType(wasmtime_tagtype_copy(other.ptr)) {}
  /// Copies another tag type into this one.
  TagType(const TagType &other)
      : TagType(wasmtime_tagtype_copy(other.ptr.get())) {}
  /// Copies another tag type into this one.
  TagType &operator=(const TagType &other) {
    ptr.reset(wasmtime_tagtype_copy(other.ptr.get()));
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
