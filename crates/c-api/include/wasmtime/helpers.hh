#ifndef WASMTIME_HELPERS_HH
#define WASMTIME_HELPERS_HH

#include <memory>

#define WASMTIME_OWN_WRAPPER(name, capi_type)                                  \
public:                                                                        \
  /**                                                                          \
   * \brief Non-owning reference to an instance of this type.                  \
   */                                                                          \
  using Raw = capi_type##_t;                                                   \
                                                                               \
private:                                                                       \
  struct deleter {                                                             \
    void operator()(Raw *p) const { capi_type##_delete(p); }                   \
  };                                                                           \
                                                                               \
  std::unique_ptr<Raw, deleter> ptr;                                           \
                                                                               \
public:                                                                        \
  /**                                                                          \
   * \brief Takes ownership of `raw` and wraps it with this class.             \
   */                                                                          \
  explicit name(Raw *raw) : ptr(raw) {}                                        \
                                                                               \
  ~name() = default;                                                           \
                                                                               \
  /**                                                                          \
   * \brief Moves type information from another type into this one.            \
   */                                                                          \
  name(name &&other) = default;                                                \
                                                                               \
  /**                                                                          \
   * \brief Moves type information from another type into this one.            \
   */                                                                          \
  name &operator=(name &&other) = default;                                     \
                                                                               \
  /**                                                                          \
   * \brief Returns the underlying C API pointer.                              \
   */                                                                          \
  const Raw *capi() const { return ptr.get(); }                                \
                                                                               \
  /**                                                                          \
   * \brief Returns the underlying C API pointer.                              \
   */                                                                          \
  Raw *capi() { return ptr.get(); }                                            \
                                                                               \
  /**                                                                          \
   * \brief Releases the underlying C API pointer.                             \
   */                                                                          \
  Raw *capi_release() { return ptr.release(); }                                \
                                                                               \
  /**                                                                          \
   * Converts the raw C API representation to this class without taking        \
   * ownership.                                                                \
   */                                                                          \
  static const name *from_capi(Raw *const *capi) {                             \
    static_assert(sizeof(name) == sizeof(void *));                             \
    return reinterpret_cast<const name *>(capi);                               \
  }

#define WASMTIME_CLONE_WRAPPER(name, capi_type)                                \
  WASMTIME_OWN_WRAPPER(name, capi_type)                                        \
                                                                               \
public:                                                                        \
  /**                                                                          \
   * \brief Copies another type into this one.                                 \
   */                                                                          \
  name(const name &other) : ptr(capi_type##_clone(other.ptr.get())) {}         \
                                                                               \
  /**                                                                          \
   * \brief Copies another type into this one.                                 \
   */                                                                          \
  name &operator=(const name &other) {                                         \
    ptr.reset(capi_type##_clone(other.ptr.get()));                             \
    return *this;                                                              \
  }

#define WASMTIME_CLONE_EQUAL_WRAPPER(name, capi_type)                          \
  WASMTIME_CLONE_WRAPPER(name, capi_type)                                      \
                                                                               \
  /**                                                                          \
   * \brief Compares two types for equality.                                   \
   */                                                                          \
  bool operator==(const name &other) const {                                   \
    return capi_type##_equal(ptr.get(), other.ptr.get());                      \
  }                                                                            \
                                                                               \
  /**                                                                          \
   * \brief Compares two types for inequality.                                 \
   */                                                                          \
  bool operator!=(const name &other) const { return !(*this == other); }

#endif // WASMTIME_HELPERS_HH
