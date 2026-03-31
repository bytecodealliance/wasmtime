/**
 * \file wasmtime/exn.hh
 */

#ifndef WASMTIME_EXN_HH
#define WASMTIME_EXN_HH

#include <wasmtime/conf.h>

#ifdef WASMTIME_FEATURE_GC

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

  /// Constructs a new exception.
  Exn(Exn &&other) = default;
  /// Moves an exception.
  Exn &operator=(Exn &&other) = default;
  Exn(const Exn &) = delete;
  Exn &operator=(const Exn &) = delete;
  /// Destroys an exception.
  ~Exn() = default;

  /**
   * \brief Create a new exception object.
   *
   * \param cx the store in which to allocate the exception
   * \param tag the tag to associate with this exception
   * \param fields the field values matching the tag's payload signature
   */
  static Result<Exn> create(Store::Context cx, const Tag &tag,
                            const std::vector<Val> &fields) {
    wasmtime_exn_t *exn = nullptr;
    auto *error = wasmtime_exn_new(
        cx.capi(), &tag.capi(),
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
    auto *error = wasmtime_exn_tag(cx.capi(), ptr.get(), &tag);
    if (error != nullptr) {
      return Error(error);
    }
    return Tag(tag);
  }

  /// Returns the number of fields in this exception.
  size_t field_count(Store::Context cx) const {
    return wasmtime_exn_field_count(cx.capi(), ptr.get());
  }

  /// Reads a field value by index.
  Result<Val> field(Store::Context cx, size_t index) const {
    wasmtime_val_t val;
    auto *error = wasmtime_exn_field(cx.capi(), ptr.get(), index, &val);
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

inline Trap Store::Context::throw_exception(Exn exn) {
  return Trap(wasmtime_context_set_exception(capi(), exn.release()));
}

inline std::optional<Exn> Store::Context::take_exception() {
  wasmtime_exn_t *exn = nullptr;
  if (wasmtime_context_take_exception(capi(), &exn)) {
    return Exn(exn);
  }
  return std::nullopt;
}

inline bool Store::Context::has_exception() {
  return wasmtime_context_has_exception(capi());
}

} // namespace wasmtime

#endif // WASMTIME_FEATURE_GC

#endif // WASMTIME_EXN_HH
