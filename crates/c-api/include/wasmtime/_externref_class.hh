#ifndef WASMTIME_EXTERNREF_CLASS_HH
#define WASMTIME_EXTERNREF_CLASS_HH

#include <wasmtime/conf.h>

#ifdef WASMTIME_FEATURE_GC

#include <wasmtime/_store_class.hh>
#include <wasmtime/externref.h>
#include <wasmtime/helpers.hh>

namespace wasmtime {

/**
 * \brief Representation of a WebAssembly `externref` value.
 *
 * This class represents an value that cannot be forged by WebAssembly itself.
 * All `ExternRef` values are guaranteed to be created by the host and its
 * embedding. It's suitable to place private data structures in here which
 * WebAssembly will not have access to, only other host functions will have
 * access to them.
 *
 * Note that `ExternRef` values are rooted within a `Store` and must be manually
 * unrooted via the `unroot` function. If this is not used then values will
 * never be candidates for garbage collection.
 */
class ExternRef {
  WASMTIME_TOP_REF_WRAPPER(ExternRef, wasmtime_externref);

private:
  static void finalizer(void *ptr) {
    std::unique_ptr<std::any> _ptr(static_cast<std::any *>(ptr));
  }

public:
  /// Creates a new `externref` value from the provided argument.
  ///
  /// Note that `val` should be safe to send across threads and should own any
  /// memory that it points to. Also note that `ExternRef` is similar to a
  /// `std::shared_ptr` in that there can be many references to the same value.
  explicit ExternRef(Store::Context cx, std::any val) {
    void *ptr = std::make_unique<std::any>(val).release();
    bool ok = wasmtime_externref_new(cx.capi(), ptr, finalizer, &this->raw);
    if (!ok) {
      fprintf(stderr, "failed to allocate a new externref");
      abort();
    }
  }

  /// Returns the underlying host data associated with this `ExternRef`.
  std::any &data(Store::Context cx) {
    return *static_cast<std::any *>(wasmtime_externref_data(cx.capi(), &raw));
  }
};

} // namespace wasmtime

#endif // WASMTIME_FEATURE_GC

#endif // WASMTIME_EXTERNREF_CLASS_HH
