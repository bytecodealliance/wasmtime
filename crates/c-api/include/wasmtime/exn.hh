/**
 * \file wasmtime/exn.hh
 */

#ifndef WASMTIME_EXN_HH
#define WASMTIME_EXN_HH

#include <vector>
#include <wasmtime/error.hh>
#include <wasmtime/exn.h>
#include <wasmtime/store.hh>
#include <wasmtime/tag.hh>
#include <wasmtime/val.hh>

namespace wasmtime {

/**
 * \brief A WebAssembly exception object.
 *
 * Exception objects carry a tag and a set of field values. They are
 * allocated on the GC heap within a store.
 *
 * This type owns its underlying `wasmtime_exn_t` handle. When it goes out
 * of scope the handle is freed.
 */
class Exn {
  struct deleter {
    void operator()(wasmtime_exn_t *p) const { wasmtime_exn_delete(p); }
  };

  std::unique_ptr<wasmtime_exn_t, deleter> ptr;

public:
  /// Takes ownership of a raw `wasmtime_exn_t` pointer.
  explicit Exn(wasmtime_exn_t *raw) : ptr(raw) {}

  Exn(Exn &&other) = default;
  Exn &operator=(Exn &&other) = default;
  Exn(const Exn &) = delete;
  Exn &operator=(const Exn &) = delete;
  ~Exn() = default;

  /**
   * \brief Create a new exception object.
   *
   * \param cx the store in which to allocate the exception
   * \param tag the tag to associate with this exception
   * \param tag_type the tag type (must match `tag`)
   * \param fields the field values matching the tag's payload signature
   */
  static Result<Exn> create(Store::Context cx, const Tag &tag,
                            const TagType &ty, const std::vector<Val> &fields) {
    wasmtime_exn_t *exn = nullptr;
    auto *error = wasmtime_exn_new(
        cx.ptr, &tag.capi(), ty.ptr.get(),
        reinterpret_cast<const wasmtime_val_t *>(fields.data()), fields.size(),
        &exn);
    if (error != nullptr) {
      return Error(error);
    }
    return Exn(exn);
  }

  /// Returns the tag associated with this exception.
  Result<Tag> tag(Store::Context cx) const {
    wasmtime_tag_t tag;
    auto *error = wasmtime_exn_tag(cx.ptr, ptr.get(), &tag);
    if (error != nullptr) {
      return Error(error);
    }
    return Tag(tag);
  }

  /// Returns the number of fields in this exception.
  size_t field_count(Store::Context cx) const {
    return wasmtime_exn_field_count(cx.ptr, ptr.get());
  }

  /// Reads a field value by index.
  Result<Val> field(Store::Context cx, size_t index) const {
    wasmtime_val_t val;
    auto *error = wasmtime_exn_field(cx.ptr, ptr.get(), index, &val);
    if (error != nullptr) {
      return Error(error);
    }
    return Val(val);
  }

  /// Returns the raw underlying C API pointer (non-owning).
  wasmtime_exn_t *capi() const { return ptr.get(); }

  /// Releases ownership of the underlying C API pointer.
  wasmtime_exn_t *release() { return ptr.release(); }
};

/**
 * \brief Sets the pending exception on the store and returns a Trap.
 *
 * This transfers ownership of `exn`. After this call, `exn` is consumed.
 *
 * Returns a Trap that the host callback MUST return to propagate the
 * exception through Wasm catch blocks.
 */
inline Trap throw_exception(Store::Context cx, Exn exn) {
  return Trap(wasmtime_context_set_exception(cx.ptr, exn.release()));
}

/**
 * \brief Takes the pending exception from the store, if any.
 *
 * Returns the exception if one was pending, or std::nullopt.
 */
inline std::optional<Exn> take_exception(Store::Context cx) {
  wasmtime_exn_t *exn = nullptr;
  if (wasmtime_context_take_exception(cx.ptr, &exn)) {
    return Exn(exn);
  }
  return std::nullopt;
}

/**
 * \brief Tests whether there is a pending exception on the store.
 */
inline bool has_exception(Store::Context cx) {
  return wasmtime_context_has_exception(cx.ptr);
}

} // namespace wasmtime

#endif // WASMTIME_EXN_HH
