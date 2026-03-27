/**
 * \file wasmtime/tag.hh
 */

#ifndef WASMTIME_TAG_HH
#define WASMTIME_TAG_HH

#include <wasmtime/error.hh>
#include <wasmtime/store.hh>
#include <wasmtime/tag.h>
#include <wasmtime/types/tag.hh>

namespace wasmtime {

/**
 * \brief A WebAssembly tag.
 *
 * Tags are used to identify exception types. A tag describes the payload
 * signature of exceptions created with it.
 *
 * Note that this type does not itself own any resources. It points to resources
 * owned within a `Store` and the `Store` must be passed in as the first
 * argument to the functions defined on `Tag`. Note that if the wrong `Store`
 * is passed in then the process will be aborted.
 */
class Tag {
  friend class Instance;
  wasmtime_tag_t tag;

public:
  /// Creates a tag from the raw underlying C API representation.
  Tag(wasmtime_tag_t tag) : tag(tag) {}

  /**
   * \brief Create a new host-defined tag.
   *
   * \param cx the store in which to create the tag
   * \param ty the tag type describing the exception payload
   */
  static Result<Tag> create(Store::Context cx, const TagType &ty) {
    wasmtime_tag_t tag;
    auto *error = wasmtime_tag_new(cx.ptr, ty.ptr.get(), &tag);
    if (error != nullptr) {
      return Error(error);
    }
    return Tag(tag);
  }

  /// Returns the type of this tag.
  TagType type(Store::Context cx) const {
    return TagType(wasmtime_tag_type(cx.ptr, &tag));
  }

  /// Tests whether two tags are identical.
  bool eq(Store::Context cx, const Tag &other) const {
    return wasmtime_tag_eq(cx.ptr, &tag, &other.tag);
  }

  /// Returns the raw underlying C API tag this is using.
  const wasmtime_tag_t &capi() const { return tag; }
};

} // namespace wasmtime

#endif // WASMTIME_TAG_HH
