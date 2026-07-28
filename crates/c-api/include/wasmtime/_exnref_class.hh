#ifndef WASMTIME_EXNREF_CLASS_HH
#define WASMTIME_EXNREF_CLASS_HH

#include <wasmtime/conf.h>

#ifdef WASMTIME_FEATURE_GC

#include <vector>
#include <wasmtime/_store_class.hh>
#include <wasmtime/error.hh>
#include <wasmtime/exnref.h>
#include <wasmtime/helpers.hh>
#include <wasmtime/tag.hh>

namespace wasmtime {

class Val;

/**
 * \brief A WebAssembly exception object.
 *
 * Exception objects carry a tag and a set of field values. They are
 * allocated on the GC heap within a store.
 *
 * This type owns its underlying `wasmtime_exnref_t` handle. When it goes out
 * of scope the handle is freed.
 */
class ExnRef {
  WASMTIME_TOP_REF_WRAPPER(ExnRef, wasmtime_exnref);

  /**
   * \brief Create a new exception object.
   *
   * \param cx the store in which to allocate the exception
   * \param tag the tag to associate with this exception
   * \param fields the field values matching the tag's payload signature
   */
  static Result<ExnRef> create(Store::Context cx, const Tag &tag,
                               const std::vector<Val> &fields);

  /// Returns the tag associated with this exception.
  Result<Tag> tag(Store::Context cx) const {
    wasmtime_tag_t tag;
    auto *error = wasmtime_exnref_tag(cx.capi(), &raw, &tag);
    if (error != nullptr) {
      return Error(error);
    }
    return Tag(tag);
  }

  /// Returns the number of fields in this exception.
  size_t field_count(Store::Context cx) const {
    return wasmtime_exnref_field_count(cx.capi(), &raw);
  }

  /// Reads a field value by index.
  Result<Val> field(Store::Context cx, size_t index) const;
};

} // namespace wasmtime

#endif // WASMTIME_FEATURE_GC

#endif // WASMTIME_EXNREF_CLASS_HH
