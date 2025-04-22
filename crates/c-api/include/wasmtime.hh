/**
 * This project is a C++ API for
 * [Wasmtime](https://github.com/bytecodealliance/wasmtime). Support for the
 * C++ API is exclusively built on the [C API of
 * Wasmtime](https://docs.wasmtime.dev/c-api/), so the C++ support for this is
 * simply a single header file. To use this header file, though, it must be
 * combined with the header and binary of Wasmtime's C API. Note, though, that
 * while this header is built on top of the `wasmtime.h` header file you should
 * only need to use the contents of this header file to interact with Wasmtime.
 *
 * Examples can be [found
 * online](https://github.com/bytecodealliance/wasmtime/tree/main/examples)
 * and otherwise be sure to check out the
 * [README](https://github.com/bytecodealliance/wasmtime/blob/main/crates/c-api/README.md)
 * for simple usage instructions. Otherwise you can dive right in to the
 * reference documentation of \ref wasmtime.hh
 *
 * \example hello.cc
 * \example gcd.cc
 * \example linking.cc
 * \example memory.cc
 * \example interrupt.cc
 * \example externref.cc
 */

/**
 * \file wasmtime.hh
 */

#ifndef WASMTIME_HH
#define WASMTIME_HH

#include <any>
#include <array>
#include <cstdio>
#include <initializer_list>
#include <iosfwd>
#include <limits>
#include <memory>
#include <optional>
#include <ostream>
#include <variant>
#include <vector>

#include <wasmtime.h>
#include <wasmtime/config.hh>
#include <wasmtime/engine.hh>
#include <wasmtime/error.hh>
#include <wasmtime/module.hh>
#include <wasmtime/types.hh>

namespace wasmtime {

/**
 * \brief Non-owning reference to a WebAssembly function frame as part of a
 * `Trace`
 *
 * A `FrameRef` represents a WebAssembly function frame on the stack which was
 * collected as part of a trap.
 */
class FrameRef {
  wasm_frame_t *frame;

public:
  /// Returns the WebAssembly function index of this function, in the original
  /// module.
  uint32_t func_index() const { return wasm_frame_func_index(frame); }
  /// Returns the offset, in bytes from the start of the function in the
  /// original module, to this frame's program counter.
  size_t func_offset() const { return wasm_frame_func_offset(frame); }
  /// Returns the offset, in bytes from the start of the original module,
  /// to this frame's program counter.
  size_t module_offset() const { return wasm_frame_module_offset(frame); }

  /// Returns the name, if present, associated with this function.
  ///
  /// Note that this requires that the `name` section is present in the original
  /// WebAssembly binary.
  std::optional<std::string_view> func_name() const {
    const auto *name = wasmtime_frame_func_name(frame);
    if (name != nullptr) {
      return std::string_view(name->data, name->size);
    }
    return std::nullopt;
  }

  /// Returns the name, if present, associated with this function's module.
  ///
  /// Note that this requires that the `name` section is present in the original
  /// WebAssembly binary.
  std::optional<std::string_view> module_name() const {
    const auto *name = wasmtime_frame_module_name(frame);
    if (name != nullptr) {
      return std::string_view(name->data, name->size);
    }
    return std::nullopt;
  }
};

/**
 * \brief An owned vector of `FrameRef` instances representing the WebAssembly
 * call-stack on a trap.
 *
 * This can be used to iterate over the frames of a trap and determine what was
 * running when a trap happened.
 */
class Trace {
  friend class Trap;
  friend class Error;

  wasm_frame_vec_t vec;

  Trace(wasm_frame_vec_t vec) : vec(vec) {}

public:
  ~Trace() { wasm_frame_vec_delete(&vec); }

  Trace(const Trace &other) = delete;
  Trace(Trace &&other) = delete;
  Trace &operator=(const Trace &other) = delete;
  Trace &operator=(Trace &&other) = delete;

  /// Iterator used to iterate over this trace.
  typedef const FrameRef *iterator;

  /// Returns the start of iteration
  iterator begin() const {
    return reinterpret_cast<FrameRef *>(&vec.data[0]); // NOLINT
  }
  /// Returns the end of iteration
  iterator end() const {
    return reinterpret_cast<FrameRef *>(&vec.data[vec.size]); // NOLINT
  }
  /// Returns the size of this trace, or how many frames it contains.
  size_t size() const { return vec.size; }
};

inline Trace Error::trace() const {
  wasm_frame_vec_t frames;
  wasmtime_error_wasm_trace(ptr.get(), &frames);
  return Trace(frames);
}

/**
 * \brief Information about a WebAssembly trap.
 *
 * Traps can happen during normal wasm execution (such as the `unreachable`
 * instruction) but they can also happen in host-provided functions to a host
 * function can simulate raising a trap.
 *
 * Traps have a message associated with them as well as a trace of WebAssembly
 * frames on the stack.
 */
class Trap {
  friend class Linker;
  friend class Instance;
  friend class Func;
  template <typename Params, typename Results> friend class TypedFunc;

  struct deleter {
    void operator()(wasm_trap_t *p) const { wasm_trap_delete(p); }
  };

  std::unique_ptr<wasm_trap_t, deleter> ptr;

  Trap(wasm_trap_t *ptr) : ptr(ptr) {}

public:
  /// Creates a new host-defined trap with the specified message.
  explicit Trap(std::string_view msg)
      : Trap(wasmtime_trap_new(msg.data(), msg.size())) {}

  /// Returns the descriptive message associated with this trap
  std::string message() const {
    wasm_byte_vec_t msg;
    wasm_trap_message(ptr.get(), &msg);
    std::string ret(msg.data, msg.size - 1);
    wasm_byte_vec_delete(&msg);
    return ret;
  }

  /// Returns the trace of WebAssembly frames associated with this trap.
  ///
  /// Note that the `trace` cannot outlive this error object.
  Trace trace() const {
    wasm_frame_vec_t frames;
    wasm_trap_trace(ptr.get(), &frames);
    return Trace(frames);
  }
};

/// Structure used to represent either a `Trap` or an `Error`.
struct TrapError {
  /// Storage for what this trap represents.
  std::variant<Trap, Error> data;

  /// Creates a new `TrapError` from a `Trap`
  TrapError(Trap t) : data(std::move(t)) {}
  /// Creates a new `TrapError` from an `Error`
  TrapError(Error e) : data(std::move(e)) {}

  /// Dispatches internally to return the message associated with this error.
  std::string message() const {
    if (const auto *trap = std::get_if<Trap>(&data)) {
      return trap->message();
    }
    if (const auto *error = std::get_if<Error>(&data)) {
      return std::string(error->message());
    }
    std::abort();
  }
};

/// Result used by functions which can fail because of invariants being violated
/// (such as a type error) as well as because of a WebAssembly trap.
template <typename T> using TrapResult = Result<T, TrapError>;

/**
 * \brief Configuration for an instance of WASI.
 *
 * This is inserted into a store with `Store::Context::set_wasi`.
 */
class WasiConfig {
  friend class Store;

  struct deleter {
    void operator()(wasi_config_t *p) const { wasi_config_delete(p); }
  };

  std::unique_ptr<wasi_config_t, deleter> ptr;

public:
  /// Creates a new configuration object with default settings.
  WasiConfig() : ptr(wasi_config_new()) {}

  /// Configures the argv explicitly with the given string array.
  void argv(const std::vector<std::string> &args) {
    std::vector<const char *> ptrs;
    ptrs.reserve(args.size());
    for (const auto &arg : args) {
      ptrs.push_back(arg.c_str());
    }

    wasi_config_set_argv(ptr.get(), (int)args.size(), ptrs.data());
  }

  /// Configures the argv for wasm to be inherited from this process itself.
  void inherit_argv() { wasi_config_inherit_argv(ptr.get()); }

  /// Configures the environment variables available to wasm, specified here as
  /// a list of pairs where the first element of the pair is the key and the
  /// second element is the value.
  void env(const std::vector<std::pair<std::string, std::string>> &env) {
    std::vector<const char *> names;
    std::vector<const char *> values;
    for (const auto &[name, value] : env) {
      names.push_back(name.c_str());
      values.push_back(value.c_str());
    }
    wasi_config_set_env(ptr.get(), (int)env.size(), names.data(),
                        values.data());
  }

  /// Indicates that the entire environment of this process should be inherited
  /// by the wasi configuration.
  void inherit_env() { wasi_config_inherit_env(ptr.get()); }

  /// Configures the provided file to be used for the stdin of this WASI
  /// configuration.
  [[nodiscard]] bool stdin_file(const std::string &path) {
    return wasi_config_set_stdin_file(ptr.get(), path.c_str());
  }

  /// Configures this WASI configuration to inherit its stdin from the host
  /// process.
  void inherit_stdin() { return wasi_config_inherit_stdin(ptr.get()); }

  /// Configures the provided file to be created and all stdout output will be
  /// written there.
  [[nodiscard]] bool stdout_file(const std::string &path) {
    return wasi_config_set_stdout_file(ptr.get(), path.c_str());
  }

  /// Configures this WASI configuration to inherit its stdout from the host
  /// process.
  void inherit_stdout() { return wasi_config_inherit_stdout(ptr.get()); }

  /// Configures the provided file to be created and all stderr output will be
  /// written there.
  [[nodiscard]] bool stderr_file(const std::string &path) {
    return wasi_config_set_stderr_file(ptr.get(), path.c_str());
  }

  /// Configures this WASI configuration to inherit its stdout from the host
  /// process.
  void inherit_stderr() { return wasi_config_inherit_stderr(ptr.get()); }

  /// Opens `path` to be opened as `guest_path` in the WASI pseudo-filesystem.
  [[nodiscard]] bool preopen_dir(const std::string &path,
                                 const std::string &guest_path,
                                 size_t dir_perms,
                                 size_t file_perms) {
    return wasi_config_preopen_dir(ptr.get(), path.c_str(), guest_path.c_str(), dir_perms, file_perms);
  }
};

class Caller;
template <typename Params, typename Results> class TypedFunc;

/**
 * \brief Owner of all WebAssembly objects
 *
 * A `Store` owns all WebAssembly objects such as instances, globals, functions,
 * memories, etc. A `Store` is one of the main central points about working with
 * WebAssembly since it's an argument to almost all APIs. The `Store` serves as
 * a form of "context" to give meaning to the pointers of `Func` and friends.
 *
 * A `Store` can be sent between threads but it cannot generally be shared
 * concurrently between threads. Memory associated with WebAssembly instances
 * will be deallocated when the `Store` is deallocated.
 */
class Store {
  struct deleter {
    void operator()(wasmtime_store_t *p) const { wasmtime_store_delete(p); }
  };

  std::unique_ptr<wasmtime_store_t, deleter> ptr;

  static void finalizer(void *ptr) {
    std::unique_ptr<std::any> _ptr(static_cast<std::any *>(ptr));
  }

public:
  /// Creates a new `Store` within the provided `Engine`.
  explicit Store(Engine &engine)
      : ptr(wasmtime_store_new(engine.ptr.get(), nullptr, finalizer)) {}

  /**
   * \brief An interior pointer into a `Store`.
   *
   * A `Context` object is created from either a `Store` or a `Caller`. It is an
   * interior pointer into a `Store` and cannot be used outside the lifetime of
   * the original object it was created from.
   *
   * This object is an argument to most APIs in Wasmtime but typically doesn't
   * need to be constructed explicitly since it can be created from a `Store&`
   * or a `Caller&`.
   */
  class Context {
    friend class Global;
    friend class Table;
    friend class Memory;
    friend class Func;
    friend class Instance;
    friend class Linker;
    friend class ExternRef;
    friend class Val;
    wasmtime_context_t *ptr;

    Context(wasmtime_context_t *ptr) : ptr(ptr) {}

  public:
    /// Creates a context referencing the provided `Store`.
    Context(Store &store) : Context(wasmtime_store_context(store.ptr.get())) {}
    /// Creates a context referencing the provided `Store`.
    Context(Store *store) : Context(*store) {}
    /// Creates a context referencing the provided `Caller`.
    Context(Caller &caller);
    /// Creates a context referencing the provided `Caller`.
    Context(Caller *caller);

    /// Runs a garbage collection pass in the referenced store to collect loose
    /// `externref` values, if any are available.
    void gc() { wasmtime_context_gc(ptr); }

    /// Injects fuel to be consumed within this store.
    ///
    /// Stores start with 0 fuel and if `Config::consume_fuel` is enabled then
    /// this is required if you want to let WebAssembly actually execute.
    ///
    /// Returns an error if fuel consumption isn't enabled.
    Result<std::monostate> set_fuel(uint64_t fuel) {
      auto *error = wasmtime_context_set_fuel(ptr, fuel);
      if (error != nullptr) {
        return Error(error);
      }
      return std::monostate();
    }

    /// Returns the amount of fuel consumed so far by executing WebAssembly.
    ///
    /// Returns `std::nullopt` if fuel consumption is not enabled.
    Result<uint64_t> get_fuel() const {
      uint64_t fuel = 0;
      auto *error = wasmtime_context_get_fuel(ptr, &fuel);
      if (error != nullptr) {
        return Error(error);
      }
      return fuel;
    }

    /// Set user specified data associated with this store.
    void set_data(std::any data) const {
      finalizer(static_cast<std::any *>(wasmtime_context_get_data(ptr)));
      wasmtime_context_set_data(
          ptr, std::make_unique<std::any>(std::move(data)).release());
    }

    /// Get user specified data associated with this store.
    std::any &get_data() const {
      return *static_cast<std::any *>(wasmtime_context_get_data(ptr));
    }

    /// Configures the WASI state used by this store.
    ///
    /// This will only have an effect if used in conjunction with
    /// `Linker::define_wasi` because otherwise no host functions will use the
    /// WASI state.
    Result<std::monostate> set_wasi(WasiConfig config) {
      auto *error = wasmtime_context_set_wasi(ptr, config.ptr.release());
      if (error != nullptr) {
        return Error(error);
      }
      return std::monostate();
    }

    /// Configures this store's epoch deadline to be the specified number of
    /// ticks beyond the engine's current epoch.
    ///
    /// By default the deadline is the current engine's epoch, immediately
    /// interrupting code if epoch interruption is enabled. This must be called
    /// to extend the deadline to allow interruption.
    void set_epoch_deadline(uint64_t ticks_beyond_current) {
      wasmtime_context_set_epoch_deadline(ptr, ticks_beyond_current);
    }

    /// Returns the raw context pointer for the C API.
    wasmtime_context_t *raw_context() { return ptr; }
  };

  /// \brief Provides limits for a store. Used by hosts to limit resource
  /// consumption of instances. Use negative value to keep the default value
  /// for the limit.
  ///
  /// \param memory_size the maximum number of bytes a linear memory can grow
  /// to. Growing a linear memory beyond this limit will fail. By default,
  /// linear memory will not be limited.
  ///
  /// \param table_elements the maximum number of elements in a table.
  /// Growing a table beyond this limit will fail. By default, table elements
  /// will not be limited.
  ///
  /// \param instances the maximum number of instances that can be created
  /// for a Store. Module instantiation will fail if this limit is exceeded.
  /// This value defaults to 10,000.
  ///
  /// \param tables the maximum number of tables that can be created for a
  /// Store. Module instantiation will fail if this limit is exceeded. This
  /// value defaults to 10,000.
  ///
  /// \param memories the maximum number of linear
  /// memories that can be created for a Store. Instantiation will fail with an
  /// error if this limit is exceeded. This value defaults to 10,000.
  ///
  /// Use any negative value for the parameters that should be kept on
  /// the default values.
  ///
  /// Note that the limits are only used to limit the creation/growth of
  /// resources in the future, this does not retroactively attempt to apply
  /// limits to the store.
  void limiter(int64_t memory_size, int64_t table_elements, int64_t instances,
               int64_t tables, int64_t memories) {
    wasmtime_store_limiter(ptr.get(), memory_size, table_elements, instances,
                           tables, memories);
  }

  /// Explicit function to acquire a `Context` from this store.
  Context context() { return this; }
};

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
  friend class Val;

  wasmtime_externref_t val;

  static void finalizer(void *ptr) {
    std::unique_ptr<std::any> _ptr(static_cast<std::any *>(ptr));
  }

public:
  /// Creates a new `ExternRef` directly from its C-API representation.
  explicit ExternRef(wasmtime_externref_t val) : val(val) {}

  /// Creates a new `externref` value from the provided argument.
  ///
  /// Note that `val` should be safe to send across threads and should own any
  /// memory that it points to. Also note that `ExternRef` is similar to a
  /// `std::shared_ptr` in that there can be many references to the same value.
  template <typename T> explicit ExternRef(Store::Context cx, T val) {
    void *ptr = std::make_unique<std::any>(std::move(val)).release();
    bool ok = wasmtime_externref_new(cx.ptr, ptr, finalizer, &this->val);
    if (!ok)  {
      fprintf(stderr, "failed to allocate a new externref");
      abort();
    }
  }

  /// Creates a new `ExternRef` which is separately rooted from this one.
  ExternRef clone(Store::Context cx) {
    wasmtime_externref_t other;
    wasmtime_externref_clone(cx.ptr, &val, &other);
    return ExternRef(other);
  }

  /// Returns the underlying host data associated with this `ExternRef`.
  std::any &data(Store::Context cx) {
    return *static_cast<std::any *>(wasmtime_externref_data(cx.ptr, &val));
  }

  /// Unroots this value from the context provided, enabling a future GC to
  /// collect the internal object if there are no more references.
  void unroot(Store::Context cx) {
    wasmtime_externref_unroot(cx.ptr, &val);
  }

  /// Returns the raw underlying C API value.
  ///
  /// This class still retains ownership of the pointer.
  const wasmtime_externref_t *raw() const { return &val; }
};

class Func;
class Global;
class Instance;
class Memory;
class Table;

/// \typedef Extern
/// \brief Representation of an external WebAssembly item
typedef std::variant<Func, Global, Memory, Table> Extern;

/// \brief Container for the `v128` WebAssembly type.
struct V128 {
  /// \brief The little-endian bytes of the `v128` value.
  wasmtime_v128 v128;

  /// \brief Creates a new zero-value `v128`.
  V128() : v128{} { memset(&v128[0], 0, sizeof(wasmtime_v128)); }

  /// \brief Creates a new `V128` from its C API representation.
  V128(const wasmtime_v128 &v) : v128{} {
    memcpy(&v128[0], &v[0], sizeof(wasmtime_v128));
  }
};

/**
 * \brief Representation of a generic WebAssembly value.
 *
 * This is roughly equivalent to a tagged union of all possible WebAssembly
 * values. This is later used as an argument with functions, globals, tables,
 * etc.
 *
 * Note that a `Val` can represent owned GC pointers. In this case the `unroot`
 * method must be used to ensure that they can later be garbage-collected.
 */
class Val {
  friend class Global;
  friend class Table;
  friend class Func;

  wasmtime_val_t val;

  Val() : val{} {
    val.kind = WASMTIME_I32;
    val.of.i32 = 0;
  }
  Val(wasmtime_val_t val) : val(val) {}

public:
  /// Creates a new `i32` WebAssembly value.
  Val(int32_t i32) : val{} {
    val.kind = WASMTIME_I32;
    val.of.i32 = i32;
  }
  /// Creates a new `i64` WebAssembly value.
  Val(int64_t i64) : val{} {
    val.kind = WASMTIME_I64;
    val.of.i64 = i64;
  }
  /// Creates a new `f32` WebAssembly value.
  Val(float f32) : val{} {
    val.kind = WASMTIME_F32;
    val.of.f32 = f32;
  }
  /// Creates a new `f64` WebAssembly value.
  Val(double f64) : val{} {
    val.kind = WASMTIME_F64;
    val.of.f64 = f64;
  }
  /// Creates a new `v128` WebAssembly value.
  Val(const V128 &v128) : val{} {
    val.kind = WASMTIME_V128;
    memcpy(&val.of.v128[0], &v128.v128[0], sizeof(wasmtime_v128));
  }
  /// Creates a new `funcref` WebAssembly value.
  Val(std::optional<Func> func);
  /// Creates a new `funcref` WebAssembly value which is not `ref.null func`.
  Val(Func func);
  /// Creates a new `externref` value.
  Val(std::optional<ExternRef> ptr) : val{} {
    val.kind = WASMTIME_EXTERNREF;
    if (ptr) {
      val.of.externref = ptr->val;
    } else {
      wasmtime_externref_set_null(&val.of.externref);
    }
  }
  /// Creates a new `externref` WebAssembly value which is not `ref.null
  /// extern`.
  Val(ExternRef ptr);

  /// Returns the kind of value that this value has.
  ValKind kind() const {
    switch (val.kind) {
    case WASMTIME_I32:
      return ValKind::I32;
    case WASMTIME_I64:
      return ValKind::I64;
    case WASMTIME_F32:
      return ValKind::F32;
    case WASMTIME_F64:
      return ValKind::F64;
    case WASMTIME_FUNCREF:
      return ValKind::FuncRef;
    case WASMTIME_EXTERNREF:
      return ValKind::ExternRef;
    case WASMTIME_V128:
      return ValKind::V128;
    }
    std::abort();
  }

  /// Returns the underlying `i32`, requires `kind() == KindI32` or aborts the
  /// process.
  int32_t i32() const {
    if (val.kind != WASMTIME_I32) {
      std::abort();
    }
    return val.of.i32;
  }

  /// Returns the underlying `i64`, requires `kind() == KindI64` or aborts the
  /// process.
  int64_t i64() const {
    if (val.kind != WASMTIME_I64) {
      std::abort();
    }
    return val.of.i64;
  }

  /// Returns the underlying `f32`, requires `kind() == KindF32` or aborts the
  /// process.
  float f32() const {
    if (val.kind != WASMTIME_F32) {
      std::abort();
    }
    return val.of.f32;
  }

  /// Returns the underlying `f64`, requires `kind() == KindF64` or aborts the
  /// process.
  double f64() const {
    if (val.kind != WASMTIME_F64) {
      std::abort();
    }
    return val.of.f64;
  }

  /// Returns the underlying `v128`, requires `kind() == KindV128` or aborts
  /// the process.
  V128 v128() const {
    if (val.kind != WASMTIME_V128) {
      std::abort();
    }
    return val.of.v128;
  }

  /// Returns the underlying `externref`, requires `kind() == KindExternRef` or
  /// aborts the process.
  ///
  /// Note that `externref` is a nullable reference, hence the `optional` return
  /// value.
  std::optional<ExternRef> externref(Store::Context cx) const {
    if (val.kind != WASMTIME_EXTERNREF) {
      std::abort();
    }
    if (val.of.externref.store_id == 0) {
      return std::nullopt;
    }
    wasmtime_externref_t other;
    wasmtime_externref_clone(cx.ptr, &val.of.externref, &other);
    return ExternRef(other);
  }

  /// Returns the underlying `funcref`, requires `kind() == KindFuncRef` or
  /// aborts the process.
  ///
  /// Note that `funcref` is a nullable reference, hence the `optional` return
  /// value.
  std::optional<Func> funcref() const;

  /// Unroots any GC references this `Val` points to within the `cx` provided.
  void unroot(Store::Context cx) {
    wasmtime_val_unroot(cx.ptr, &val);
  }
};

/**
 * \brief Structure provided to host functions to lookup caller information or
 * acquire a `Store::Context`.
 *
 * This structure is passed to all host functions created with `Func`. It can be
 * used to create a `Store::Context`.
 */
class Caller {
  friend class Func;
  friend class Store;
  wasmtime_caller_t *ptr;
  Caller(wasmtime_caller_t *ptr) : ptr(ptr) {}

public:
  /// Attempts to load an exported item from the calling instance.
  ///
  /// For more information see the Rust documentation -
  /// https://docs.wasmtime.dev/api/wasmtime/struct.Caller.html#method.get_export
  std::optional<Extern> get_export(std::string_view name);

  /// Explicitly acquire a `Store::Context` from this `Caller`.
  Store::Context context() { return this; }
};

inline Store::Context::Context(Caller &caller)
    : Context(wasmtime_caller_context(caller.ptr)) {}
inline Store::Context::Context(Caller *caller) : Context(*caller) {}

namespace detail {

/// A "trait" for native types that correspond to WebAssembly types for use with
/// `Func::wrap` and `TypedFunc::call`
template <typename T> struct WasmType { static const bool valid = false; };

/// Helper macro to define `WasmType` definitions for primitive types like
/// int32_t and such.
// NOLINTNEXTLINE
#define NATIVE_WASM_TYPE(native, valkind, field)                               \
  template <> struct WasmType<native> {                                        \
    static const bool valid = true;                                            \
    static const ValKind kind = ValKind::valkind;                              \
    static void store(Store::Context cx, wasmtime_val_raw_t *p,                \
                      const native &t) {                                       \
      p->field = t;                                                            \
    }                                                                          \
    static native load(Store::Context cx, wasmtime_val_raw_t *p) {             \
      return p->field;                                                         \
    }                                                                          \
  };

NATIVE_WASM_TYPE(int32_t, I32, i32)
NATIVE_WASM_TYPE(uint32_t, I32, i32)
NATIVE_WASM_TYPE(int64_t, I64, i64)
NATIVE_WASM_TYPE(uint64_t, I64, i64)
NATIVE_WASM_TYPE(float, F32, f32)
NATIVE_WASM_TYPE(double, F64, f64)

#undef NATIVE_WASM_TYPE

/// Type information for `externref`, represented on the host as an optional
/// `ExternRef`.
template <> struct WasmType<std::optional<ExternRef>> {
  static const bool valid = true;
  static const ValKind kind = ValKind::ExternRef;
  static void store(Store::Context cx, wasmtime_val_raw_t *p,
                    const std::optional<ExternRef> &ref) {
    if (ref) {
      p->externref = wasmtime_externref_to_raw(cx.raw_context(), ref->raw());
    } else {
      p->externref = 0;
    }
  }
  static std::optional<ExternRef> load(Store::Context cx,
                                       wasmtime_val_raw_t *p) {
    if (p->externref == 0) {
      return std::nullopt;
    }
    wasmtime_externref_t val;
    wasmtime_externref_from_raw(cx.raw_context(), p->externref, &val);
    return ExternRef(val);
  }
};

/// Type information for the `V128` host value used as a wasm value.
template <> struct WasmType<V128> {
  static const bool valid = true;
  static const ValKind kind = ValKind::V128;
  static void store(Store::Context cx, wasmtime_val_raw_t *p, const V128 &t) {
    memcpy(&p->v128[0], &t.v128[0], sizeof(wasmtime_v128));
  }
  static V128 load(Store::Context cx, wasmtime_val_raw_t *p) { return p->v128; }
};

/// A "trait" for a list of types and operations on them, used for `Func::wrap`
/// and `TypedFunc::call`
///
/// The base case is a single type which is a list of one element.
template <typename T> struct WasmTypeList {
  static const bool valid = WasmType<T>::valid;
  static const size_t size = 1;
  static bool matches(ValType::ListRef types) {
    return WasmTypeList<std::tuple<T>>::matches(types);
  }
  static void store(Store::Context cx, wasmtime_val_raw_t *storage,
                    const T &t) {
    WasmType<T>::store(cx, storage, t);
  }
  static T load(Store::Context cx, wasmtime_val_raw_t *storage) {
    return WasmType<T>::load(cx, storage);
  }
  static std::vector<ValType> types() { return {WasmType<T>::kind}; }
};

/// std::monostate translates to an empty list of types.
template <> struct WasmTypeList<std::monostate> {
  static const bool valid = true;
  static const size_t size = 0;
  static bool matches(ValType::ListRef types) { return types.size() == 0; }
  static void store(Store::Context cx, wasmtime_val_raw_t *storage,
                    const std::monostate &t) {}
  static std::monostate load(Store::Context cx, wasmtime_val_raw_t *storage) {
    return std::monostate();
  }
  static std::vector<ValType> types() { return {}; }
};

/// std::tuple<> translates to the corresponding list of types
template <typename... T> struct WasmTypeList<std::tuple<T...>> {
  static const bool valid = (WasmType<T>::valid && ...);
  static const size_t size = sizeof...(T);
  static bool matches(ValType::ListRef types) {
    if (types.size() != size) {
      return false;
    }
    size_t n = 0;
    return ((WasmType<T>::kind == types.begin()[n++].kind()) && ...);
  }
  static void store(Store::Context cx, wasmtime_val_raw_t *storage,
                    const std::tuple<T...> &t) {
    size_t n = 0;
    std::apply(
        [&](const auto &...val) {
          (WasmType<T>::store(cx, &storage[n++], val), ...); // NOLINT
        },
        t);
  }
  static std::tuple<T...> load(Store::Context cx, wasmtime_val_raw_t *storage) {
    size_t n = 0;
    return std::tuple<T...>{WasmType<T>::load(cx, &storage[n++])...}; // NOLINT
  }
  static std::vector<ValType> types() { return {WasmType<T>::kind...}; }
};

/// A "trait" for what can be returned from closures specified to `Func::wrap`.
///
/// The base case here is a bare return value like `int32_t`.
template <typename R> struct WasmHostRet {
  using Results = WasmTypeList<R>;

  template <typename F, typename... A>
  static std::optional<Trap> invoke(F f, Caller cx, wasmtime_val_raw_t *raw,
                                    A... args) {
    auto ret = f(args...);
    Results::store(cx, raw, ret);
    return std::nullopt;
  }
};

/// Host functions can return nothing
template <> struct WasmHostRet<void> {
  using Results = WasmTypeList<std::tuple<>>;

  template <typename F, typename... A>
  static std::optional<Trap> invoke(F f, Caller cx, wasmtime_val_raw_t *raw,
                                    A... args) {
    f(args...);
    return std::nullopt;
  }
};

// Alternative method of returning "nothing" (also enables `std::monostate` in
// the `R` type of `Result` below)
template <> struct WasmHostRet<std::monostate> : public WasmHostRet<void> {};

/// Host functions can return a result which allows them to also possibly return
/// a trap.
template <typename R> struct WasmHostRet<Result<R, Trap>> {
  using Results = WasmTypeList<R>;

  template <typename F, typename... A>
  static std::optional<Trap> invoke(F f, Caller cx, wasmtime_val_raw_t *raw,
                                    A... args) {
    Result<R, Trap> ret = f(args...);
    if (!ret) {
      return ret.err();
    }
    Results::store(cx, raw, ret.ok());
    return std::nullopt;
  }
};

template <typename F, typename = void> struct WasmHostFunc;

/// Base type information for host free-function pointers being used as wasm
/// functions
template <typename R, typename... A> struct WasmHostFunc<R (*)(A...)> {
  using Params = WasmTypeList<std::tuple<A...>>;
  using Results = typename WasmHostRet<R>::Results;

  template <typename F>
  static std::optional<Trap> invoke(F &f, Caller cx, wasmtime_val_raw_t *raw) {
    auto params = Params::load(cx, raw);
    return std::apply(
        [&](const auto &...val) {
          return WasmHostRet<R>::invoke(f, cx, raw, val...);
        },
        params);
  }
};

/// Function type information, but with a `Caller` first parameter
template <typename R, typename... A>
struct WasmHostFunc<R (*)(Caller, A...)> : public WasmHostFunc<R (*)(A...)> {
  // Override `invoke` here to pass the `cx` as the first parameter
  template <typename F>
  static std::optional<Trap> invoke(F &f, Caller cx, wasmtime_val_raw_t *raw) {
    auto params = WasmTypeList<std::tuple<A...>>::load(cx, raw);
    return std::apply(
        [&](const auto &...val) {
          return WasmHostRet<R>::invoke(f, cx, raw, cx, val...);
        },
        params);
  }
};

/// Function type information, but with a class method.
template <typename R, typename C, typename... A>
struct WasmHostFunc<R (C::*)(A...)> : public WasmHostFunc<R (*)(A...)> {};

/// Function type information, but with a const class method.
template <typename R, typename C, typename... A>
struct WasmHostFunc<R (C::*)(A...) const> : public WasmHostFunc<R (*)(A...)> {};

/// Function type information, but as a host method with a `Caller` first
/// parameter.
template <typename R, typename C, typename... A>
struct WasmHostFunc<R (C::*)(Caller, A...)>
    : public WasmHostFunc<R (*)(Caller, A...)> {};

/// Function type information, but as a host const method with a `Caller`
/// first parameter.
template <typename R, typename C, typename... A>
struct WasmHostFunc<R (C::*)(Caller, A...) const>
    : public WasmHostFunc<R (*)(Caller, A...)> {};

/// Base type information for host callables being used as wasm
/// functions
template <typename T>
struct WasmHostFunc<T, std::void_t<decltype(&T::operator())>>
    : public WasmHostFunc<decltype(&T::operator())> {};

} // namespace detail

using namespace detail;

/**
 * \brief Representation of a WebAssembly function.
 *
 * This class represents a WebAssembly function, either created through
 * instantiating a module or a host function.
 *
 * Note that this type does not itself own any resources. It points to resources
 * owned within a `Store` and the `Store` must be passed in as the first
 * argument to the functions defined on `Func`. Note that if the wrong `Store`
 * is passed in then the process will be aborted.
 */
class Func {
  friend class Val;
  friend class Instance;
  friend class Linker;
  template <typename Params, typename Results> friend class TypedFunc;

  wasmtime_func_t func;

  template <typename F>
  static wasm_trap_t *raw_callback(void *env, wasmtime_caller_t *caller,
                                   const wasmtime_val_t *args, size_t nargs,
                                   wasmtime_val_t *results, size_t nresults) {
    static_assert(alignof(Val) == alignof(wasmtime_val_t));
    static_assert(sizeof(Val) == sizeof(wasmtime_val_t));
    F *func = reinterpret_cast<F *>(env);                          // NOLINT
    Span<const Val> args_span(reinterpret_cast<const Val *>(args), // NOLINT
                              nargs);
    Span<Val> results_span(reinterpret_cast<Val *>(results), // NOLINT
                           nresults);
    Result<std::monostate, Trap> result =
        (*func)(Caller(caller), args_span, results_span);
    if (!result) {
      return result.err().ptr.release();
    }
    return nullptr;
  }

  template <typename F>
  static wasm_trap_t *
  raw_callback_unchecked(void *env, wasmtime_caller_t *caller,
                         wasmtime_val_raw_t *args_and_results,
                         size_t nargs_and_results) {
    using HostFunc = WasmHostFunc<F>;
    Caller cx(caller);
    F *func = reinterpret_cast<F *>(env); // NOLINT
    auto trap = HostFunc::invoke(*func, cx, args_and_results);
    if (trap) {
      return trap->ptr.release();
    }
    return nullptr;
  }

  template <typename F> static void raw_finalize(void *env) {
    std::unique_ptr<F> ptr(reinterpret_cast<F *>(env)); // NOLINT
  }

public:
  /// Creates a new function from the raw underlying C API representation.
  Func(wasmtime_func_t func) : func(func) {}

  /**
   * \brief Creates a new host-defined function.
   *
   * This constructor is used to create a host function within the store
   * provided. This is how WebAssembly can call into the host and make use of
   * external functionality.
   *
   * > **Note**: host functions created this way are more flexible but not
   * > as fast to call as those created by `Func::wrap`.
   *
   * \param cx the store to create the function within
   * \param ty the type of the function that will be created
   * \param f the host callback to be executed when this function is called.
   *
   * The parameter `f` is expected to be a lambda (or a lambda lookalike) which
   * takes three parameters:
   *
   * * The first parameter is a `Caller` to get recursive access to the store
   *   and other caller state.
   * * The second parameter is a `Span<const Val>` which is the list of
   *   parameters to the function. These parameters are guaranteed to be of the
   *   types specified by `ty` when constructing this function.
   * * The last argument is `Span<Val>` which is where to write the return
   *   values of the function. The function must produce the types of values
   *   specified by `ty` or otherwise a trap will be raised.
   *
   * The parameter `f` is expected to return `Result<std::monostate, Trap>`.
   * This allows `f` to raise a trap if desired, or otherwise return no trap and
   * finish successfully. If a trap is raised then the results pointer does not
   * need to be written to.
   */
  template <typename F,
            std::enable_if_t<
                std::is_invocable_r_v<Result<std::monostate, Trap>, F, Caller,
                                      Span<const Val>, Span<Val>>,
                bool> = true>
  Func(Store::Context cx, const FuncType &ty, F f) : func({}) {
    wasmtime_func_new(cx.ptr, ty.ptr.get(), raw_callback<F>,
                      std::make_unique<F>(f).release(), raw_finalize<F>, &func);
  }

  /**
   * \brief Creates a new host function from the provided callback `f`,
   * inferring the WebAssembly function type from the host signature.
   *
   * This function is akin to the `Func` constructor except that the WebAssembly
   * type does not need to be specified and additionally the signature of `f`
   * is different. The main goal of this function is to enable WebAssembly to
   * call the function `f` as-fast-as-possible without having to validate any
   * types or such.
   *
   * The function `f` can optionally take a `Caller` as its first parameter,
   * but otherwise its arguments are translated to WebAssembly types:
   *
   * * `int32_t`, `uint32_t` - `i32`
   * * `int64_t`, `uint64_t` - `i64`
   * * `float` - `f32`
   * * `double` - `f64`
   * * `std::optional<Func>` - `funcref`
   * * `std::optional<ExternRef>` - `externref`
   * * `wasmtime::V128` - `v128`
   *
   * The function may only take these arguments and if it takes any other kinds
   * of arguments then it will fail to compile.
   *
   * The function may return a few different flavors of return values:
   *
   * * `void` - interpreted as returning nothing
   * * Any type above - interpreted as a singular return value.
   * * `std::tuple<T...>` where `T` is one of the valid argument types -
   *   interpreted as returning multiple values.
   * * `Result<T, Trap>` where `T` is another valid return type - interpreted as
   *   a function that returns `T` to wasm but is optionally allowed to also
   *   raise a trap.
   *
   * It's recommended, if possible, to use this function over the `Func`
   * constructor since this is generally easier to work with and also enables
   * a faster path for WebAssembly to call this function.
   */
  template <typename F,
            std::enable_if_t<WasmHostFunc<F>::Params::valid, bool> = true,
            std::enable_if_t<WasmHostFunc<F>::Results::valid, bool> = true>
  static Func wrap(Store::Context cx, F f) {
    using HostFunc = WasmHostFunc<F>;
    auto params = HostFunc::Params::types();
    auto results = HostFunc::Results::types();
    auto ty = FuncType::from_iters(params, results);
    wasmtime_func_t func;
    wasmtime_func_new_unchecked(cx.ptr, ty.ptr.get(), raw_callback_unchecked<F>,
                                std::make_unique<F>(f).release(),
                                raw_finalize<F>, &func);
    return func;
  }

  /**
   * \brief Invoke a WebAssembly function.
   *
   * This function will execute this WebAssembly function. This function muts be
   * defined within the `cx`'s store provided. The `params` argument is the list
   * of parameters that are passed to the wasm function, and the types of the
   * values within `params` must match the type signature of this function.
   *
   * This may return one of three values:
   *
   * * First the function could succeed, returning a vector of values
   *   representing the results of the function.
   * * Otherwise a `Trap` might be generated by the WebAssembly function.
   * * Finally an `Error` could be returned indicating that `params` were not of
   *   the right type.
   *
   * > **Note**: for optimized calls into WebAssembly where the function
   * > signature is statically known it's recommended to use `Func::typed` and
   * > `TypedFunc::call`.
   */
  template <typename I>
  TrapResult<std::vector<Val>> call(Store::Context cx, const I &begin,
                                    const I &end) const {
    std::vector<wasmtime_val_t> raw_params;
    raw_params.reserve(end - begin);
    for (auto i = begin; i != end; i++) {
      raw_params.push_back(i->val);
    }
    size_t nresults = this->type(cx)->results().size();
    std::vector<wasmtime_val_t> raw_results(nresults);

    wasm_trap_t *trap = nullptr;
    auto *error =
        wasmtime_func_call(cx.ptr, &func, raw_params.data(), raw_params.size(),
                           raw_results.data(), raw_results.capacity(), &trap);
    if (error != nullptr) {
      return TrapError(Error(error));
    }
    if (trap != nullptr) {
      return TrapError(Trap(trap));
    }

    std::vector<Val> results;
    results.reserve(nresults);
    for (size_t i = 0; i < nresults; i++) {
      results.push_back(raw_results[i]);
    }
    return results;
  }

  /**
   * \brief Helper function for `call(Store::Context cx, const I &begin, const I
   * &end)`
   *
   * \see call(Store::Context cx, const I &begin, const I &end)
   */
  TrapResult<std::vector<Val>> call(Store::Context cx,
                                    const std::vector<Val> &params) const {
    return this->call(cx, params.begin(), params.end());
  }

  /**
   * \brief Helper function for `call(Store::Context cx, const I &begin, const I
   * &end)`
   *
   * \see call(Store::Context cx, const I &begin, const I &end)
   */
  TrapResult<std::vector<Val>>
  call(Store::Context cx, const std::initializer_list<Val> &params) const {
    return this->call(cx, params.begin(), params.end());
  }

  /// Returns the type of this function.
  FuncType type(Store::Context cx) const {
    return wasmtime_func_type(cx.ptr, &func);
  }

  /**
   * \brief Statically checks this function against the provided types.
   *
   * This function will check whether it takes the statically known `Params`
   * and returns the statically known `Results`. If the type check succeeds then
   * a `TypedFunc` is returned which enables a faster method of invoking
   * WebAssembly functions.
   *
   * The `Params` and `Results` specified as template parameters here are the
   * parameters and results of the wasm function. They can either be a bare
   * type which means that the wasm takes/returns one value, or they can be a
   * `std::tuple<T...>` of types to represent multiple arguments or multiple
   * returns.
   *
   * The valid types for this function are those mentioned as the arguments
   * for `Func::wrap`.
   */
  template <typename Params, typename Results,
            std::enable_if_t<WasmTypeList<Params>::valid, bool> = true,
            std::enable_if_t<WasmTypeList<Results>::valid, bool> = true>
  Result<TypedFunc<Params, Results>, Trap> typed(Store::Context cx) const {
    auto ty = this->type(cx);
    if (!WasmTypeList<Params>::matches(ty->params()) ||
        !WasmTypeList<Results>::matches(ty->results())) {
      return Trap("static type for this function does not match actual type");
    }
    TypedFunc<Params, Results> ret(*this);
    return ret;
  }

  /// Returns the raw underlying C API function this is using.
  const wasmtime_func_t &raw_func() const { return func; }
};

/**
 * \brief A version of a WebAssembly `Func` where the type signature of the
 * function is statically known.
 */
template <typename Params, typename Results> class TypedFunc {
  friend class Func;
  Func f;
  TypedFunc(Func func) : f(func) {}

public:
  /**
   * \brief Calls this function with the provided parameters.
   *
   * This function is akin to `Func::call` except that since static type
   * information is available it statically takes its parameters and statically
   * returns its results.
   *
   * Note that this function still may return a `Trap` indicating that calling
   * the WebAssembly function failed.
   */
  TrapResult<Results> call(Store::Context cx, Params params) const {
    std::array<wasmtime_val_raw_t, std::max(WasmTypeList<Params>::size,
                                            WasmTypeList<Results>::size)>
        storage;
    wasmtime_val_raw_t *ptr = storage.data();
    if (ptr == nullptr)
      ptr = reinterpret_cast<wasmtime_val_raw_t*>(alignof(wasmtime_val_raw_t));
    WasmTypeList<Params>::store(cx, ptr, params);
    wasm_trap_t *trap = nullptr;
    auto *error = wasmtime_func_call_unchecked(
        cx.raw_context(), &f.func, ptr, storage.size(), &trap);
    if (error != nullptr) {
      return TrapError(Error(error));
    }
    if (trap != nullptr) {
      return TrapError(Trap(trap));
    }
    return WasmTypeList<Results>::load(cx, ptr);
  }

  /// Returns the underlying un-typed `Func` for this function.
  const Func &func() const { return f; }
};

inline Val::Val(std::optional<Func> func) : val{} {
  val.kind = WASMTIME_FUNCREF;
  if (func) {
    val.of.funcref = (*func).func;
  } else {
    wasmtime_funcref_set_null(&val.of.funcref);
  }
}

inline Val::Val(Func func) : Val(std::optional(func)) {}
inline Val::Val(ExternRef ptr) : Val(std::optional(ptr)) {}

inline std::optional<Func> Val::funcref() const {
  if (val.kind != WASMTIME_FUNCREF) {
    std::abort();
  }
  if (val.of.funcref.store_id == 0) {
    return std::nullopt;
  }
  return Func(val.of.funcref);
}

/// Definition for the `funcref` native wasm type
template <> struct detail::WasmType<std::optional<Func>> {
  /// @private
  static const bool valid = true;
  /// @private
  static const ValKind kind = ValKind::FuncRef;
  /// @private
  static void store(Store::Context cx, wasmtime_val_raw_t *p,
                    const std::optional<Func> func) {
    if (func) {
      p->funcref = wasmtime_func_to_raw(cx.raw_context(), &func->raw_func());
    } else {
      p->funcref = 0;
    }
  }
  /// @private
  static std::optional<Func> load(Store::Context cx, wasmtime_val_raw_t *p) {
    if (p->funcref == 0) {
      return std::nullopt;
    }
    wasmtime_func_t ret;
    wasmtime_func_from_raw(cx.raw_context(), p->funcref, &ret);
    return ret;
  }
};

/**
 * \brief A WebAssembly global.
 *
 * This class represents a WebAssembly global, either created through
 * instantiating a module or a host global. Globals contain a WebAssembly value
 * and can be read and optionally written to.
 *
 * Note that this type does not itself own any resources. It points to resources
 * owned within a `Store` and the `Store` must be passed in as the first
 * argument to the functions defined on `Global`. Note that if the wrong `Store`
 * is passed in then the process will be aborted.
 */
class Global {
  friend class Instance;
  wasmtime_global_t global;

public:
  /// Creates as global from the raw underlying C API representation.
  Global(wasmtime_global_t global) : global(global) {}

  /**
   * \brief Create a new WebAssembly global.
   *
   * \param cx the store in which to create the global
   * \param ty the type that this global will have
   * \param init the initial value of the global
   *
   * This function can fail if `init` does not have a value that matches `ty`.
   */
  static Result<Global> create(Store::Context cx, const GlobalType &ty,
                               const Val &init) {
    wasmtime_global_t global;
    auto *error = wasmtime_global_new(cx.ptr, ty.ptr.get(), &init.val, &global);
    if (error != nullptr) {
      return Error(error);
    }
    return Global(global);
  }

  /// Returns the type of this global.
  GlobalType type(Store::Context cx) const {
    return wasmtime_global_type(cx.ptr, &global);
  }

  /// Returns the current value of this global.
  Val get(Store::Context cx) const;

  /// Sets this global to a new value.
  ///
  /// This can fail if `val` has the wrong type or if this global isn't mutable.
  Result<std::monostate> set(Store::Context cx, const Val &val) const {
    auto *error = wasmtime_global_set(cx.ptr, &global, &val.val);
    if (error != nullptr) {
      return Error(error);
    }
    return std::monostate();
  }
};

/**
 * \brief A WebAssembly table.
 *
 * This class represents a WebAssembly table, either created through
 * instantiating a module or a host table. Tables are contiguous vectors of
 * WebAssembly reference types, currently either `externref` or `funcref`.
 *
 * Note that this type does not itself own any resources. It points to resources
 * owned within a `Store` and the `Store` must be passed in as the first
 * argument to the functions defined on `Table`. Note that if the wrong `Store`
 * is passed in then the process will be aborted.
 */
class Table {
  friend class Instance;
  wasmtime_table_t table;

public:
  /// Creates a new table from the raw underlying C API representation.
  Table(wasmtime_table_t table) : table(table) {}

  /**
   * \brief Creates a new host-defined table.
   *
   * \param cx the store in which to create the table.
   * \param ty the type of the table to be created
   * \param init the initial value for all table slots.
   *
   * Returns an error if `init` has the wrong value for the `ty` specified.
   */
  static Result<Table> create(Store::Context cx, const TableType &ty,
                              const Val &init) {
    wasmtime_table_t table;
    auto *error = wasmtime_table_new(cx.ptr, ty.ptr.get(), &init.val, &table);
    if (error != nullptr) {
      return Error(error);
    }
    return Table(table);
  }

  /// Returns the type of this table.
  TableType type(Store::Context cx) const {
    return wasmtime_table_type(cx.ptr, &table);
  }

  /// Returns the size, in elements, that the table currently has.
  uint64_t size(Store::Context cx) const {
    return wasmtime_table_size(cx.ptr, &table);
  }

  /// Loads a value from the specified index in this table.
  ///
  /// Returns `std::nullopt` if `idx` is out of bounds.
  std::optional<Val> get(Store::Context cx, uint64_t idx) const {
    Val val;
    if (wasmtime_table_get(cx.ptr, &table, idx, &val.val)) {
      return val;
    }
    return std::nullopt;
  }

  /// Stores a value into the specified index in this table.
  ///
  /// Returns an error if `idx` is out of bounds or if `val` has the wrong type.
  Result<std::monostate> set(Store::Context cx, uint64_t idx,
                             const Val &val) const {
    auto *error = wasmtime_table_set(cx.ptr, &table, idx, &val.val);
    if (error != nullptr) {
      return Error(error);
    }
    return std::monostate();
  }

  /// Grow this table.
  ///
  /// \param cx the store that owns this table.
  /// \param delta the number of new elements to be added to this table.
  /// \param init the initial value of all new elements in this table.
  ///
  /// Returns an error if `init` has the wrong type for this table. Otherwise
  /// returns the previous size of the table before growth.
  Result<uint64_t> grow(Store::Context cx, uint64_t delta,
                        const Val &init) const {
    uint64_t prev = 0;
    auto *error = wasmtime_table_grow(cx.ptr, &table, delta, &init.val, &prev);
    if (error != nullptr) {
      return Error(error);
    }
    return prev;
  }
};

// gcc 8.3.0 seems to require that this comes after the definition of `Table`. I
// don't know why...
inline Val Global::get(Store::Context cx) const {
  Val val;
  wasmtime_global_get(cx.ptr, &global, &val.val);
  return val;
}

/**
 * \brief A WebAssembly linear memory.
 *
 * This class represents a WebAssembly memory, either created through
 * instantiating a module or a host memory.
 *
 * Note that this type does not itself own any resources. It points to resources
 * owned within a `Store` and the `Store` must be passed in as the first
 * argument to the functions defined on `Table`. Note that if the wrong `Store`
 * is passed in then the process will be aborted.
 */
class Memory {
  friend class Instance;
  wasmtime_memory_t memory;

public:
  /// Creates a new memory from the raw underlying C API representation.
  Memory(wasmtime_memory_t memory) : memory(memory) {}

  /// Creates a new host-defined memory with the type specified.
  static Result<Memory> create(Store::Context cx, const MemoryType &ty) {
    wasmtime_memory_t memory;
    auto *error = wasmtime_memory_new(cx.ptr, ty.ptr.get(), &memory);
    if (error != nullptr) {
      return Error(error);
    }
    return Memory(memory);
  }

  /// Returns the type of this memory.
  MemoryType type(Store::Context cx) const {
    return wasmtime_memory_type(cx.ptr, &memory);
  }

  /// Returns the size, in WebAssembly pages, of this memory.
  uint64_t size(Store::Context cx) const {
    return wasmtime_memory_size(cx.ptr, &memory);
  }

  /// Returns a `span` of where this memory is located in the host.
  ///
  /// Note that embedders need to be very careful in their usage of the returned
  /// `span`. It can be invalidated with calls to `grow` and/or calls into
  /// WebAssembly.
  Span<uint8_t> data(Store::Context cx) const {
    auto *base = wasmtime_memory_data(cx.ptr, &memory);
    auto size = wasmtime_memory_data_size(cx.ptr, &memory);
    return {base, size};
  }

  /// Grows the memory by `delta` WebAssembly pages.
  ///
  /// On success returns the previous size of this memory in units of
  /// WebAssembly pages.
  Result<uint64_t> grow(Store::Context cx, uint64_t delta) const {
    uint64_t prev = 0;
    auto *error = wasmtime_memory_grow(cx.ptr, &memory, delta, &prev);
    if (error != nullptr) {
      return Error(error);
    }
    return prev;
  }
};

/**
 * \brief A WebAssembly instance.
 *
 * This class represents a WebAssembly instance, created by instantiating a
 * module. An instance is the collection of items exported by the module, which
 * can be accessed through the `Store` that owns the instance.
 *
 * Note that this type does not itself own any resources. It points to resources
 * owned within a `Store` and the `Store` must be passed in as the first
 * argument to the functions defined on `Instance`. Note that if the wrong
 * `Store` is passed in then the process will be aborted.
 */
class Instance {
  friend class Linker;
  friend class Caller;

  wasmtime_instance_t instance;

  static Extern cvt(wasmtime_extern_t &e) {
    switch (e.kind) {
    case WASMTIME_EXTERN_FUNC:
      return Func(e.of.func);
    case WASMTIME_EXTERN_GLOBAL:
      return Global(e.of.global);
    case WASMTIME_EXTERN_MEMORY:
      return Memory(e.of.memory);
    case WASMTIME_EXTERN_TABLE:
      return Table(e.of.table);
    }
    std::abort();
  }

  static void cvt(const Extern &e, wasmtime_extern_t &raw) {
    if (const auto *func = std::get_if<Func>(&e)) {
      raw.kind = WASMTIME_EXTERN_FUNC;
      raw.of.func = func->func;
    } else if (const auto *global = std::get_if<Global>(&e)) {
      raw.kind = WASMTIME_EXTERN_GLOBAL;
      raw.of.global = global->global;
    } else if (const auto *table = std::get_if<Table>(&e)) {
      raw.kind = WASMTIME_EXTERN_TABLE;
      raw.of.table = table->table;
    } else if (const auto *memory = std::get_if<Memory>(&e)) {
      raw.kind = WASMTIME_EXTERN_MEMORY;
      raw.of.memory = memory->memory;
    } else {
      std::abort();
    }
  }

public:
  /// Creates a new instance from the raw underlying C API representation.
  Instance(wasmtime_instance_t instance) : instance(instance) {}

  /**
   * \brief Instantiates the module `m` with the provided `imports`
   *
   * \param cx the store in which to instantiate the provided module
   * \param m the module to instantiate
   * \param imports the list of imports to use to instantiate the module
   *
   * This `imports` parameter is expected to line up 1:1 with the imports
   * required by the `m`. The type of `m` can be inspected to determine in which
   * order to provide the imports. Note that this is a relatively low-level API
   * and it's generally recommended to use `Linker` instead for name-based
   * instantiation.
   *
   * This function can return an error if any of the `imports` have the wrong
   * type, or if the wrong number of `imports` is provided.
   */
  static TrapResult<Instance> create(Store::Context cx, const Module &m,
                                     const std::vector<Extern> &imports) {
    std::vector<wasmtime_extern_t> raw_imports;
    for (const auto &item : imports) {
      raw_imports.push_back(wasmtime_extern_t{});
      auto &last = raw_imports.back();
      Instance::cvt(item, last);
    }
    wasmtime_instance_t instance;
    wasm_trap_t *trap = nullptr;
    auto *error = wasmtime_instance_new(cx.ptr, m.ptr.get(), raw_imports.data(),
                                        raw_imports.size(), &instance, &trap);
    if (error != nullptr) {
      return TrapError(Error(error));
    }
    if (trap != nullptr) {
      return TrapError(Trap(trap));
    }
    return Instance(instance);
  }

  /**
   * \brief Load an instance's export by name.
   *
   * This function will look for an export named `name` on this instance and, if
   * found, return it as an `Extern`.
   */
  std::optional<Extern> get(Store::Context cx, std::string_view name) {
    wasmtime_extern_t e;
    if (!wasmtime_instance_export_get(cx.ptr, &instance, name.data(),
                                      name.size(), &e)) {
      return std::nullopt;
    }
    return Instance::cvt(e);
  }

  /**
   * \brief Load an instance's export by index.
   *
   * This function will look for the `idx`th export of this instance. This will
   * return both the name of the export as well as the exported item itself.
   */
  std::optional<std::pair<std::string_view, Extern>> get(Store::Context cx,
                                                         size_t idx) {
    wasmtime_extern_t e;
    // I'm not sure why clang-tidy thinks this is using va_list or anything
    // related to that...
    // NOLINTNEXTLINE(cppcoreguidelines-pro-type-vararg)
    char *name = nullptr;
    size_t len = 0;
    if (!wasmtime_instance_export_nth(cx.ptr, &instance, idx, &name, &len,
                                      &e)) {
      return std::nullopt;
    }
    std::string_view n(name, len);
    return std::pair(n, Instance::cvt(e));
  }
};

inline std::optional<Extern> Caller::get_export(std::string_view name) {
  wasmtime_extern_t item;
  if (wasmtime_caller_export_get(ptr, name.data(), name.size(), &item)) {
    return Instance::cvt(item);
  }
  return std::nullopt;
}

/**
 * \brief Helper class for linking modules together with name-based resolution.
 *
 * This class is used for easily instantiating `Module`s by defining names into
 * the linker and performing name-based resolution during instantiation. A
 * `Linker` can also be used to link in WASI functions to instantiate a module.
 */
class Linker {
  struct deleter {
    void operator()(wasmtime_linker_t *p) const { wasmtime_linker_delete(p); }
  };

  std::unique_ptr<wasmtime_linker_t, deleter> ptr;

public:
  /// Creates a new linker which will instantiate in the given engine.
  explicit Linker(Engine &engine)
      : ptr(wasmtime_linker_new(engine.ptr.get())) {}

  /// Configures whether shadowing previous names is allowed or not.
  ///
  /// By default shadowing is not allowed.
  void allow_shadowing(bool allow) {
    wasmtime_linker_allow_shadowing(ptr.get(), allow);
  }

  /// Defines the provided item into this linker with the given name.
  Result<std::monostate> define(Store::Context cx, std::string_view module,
                                std::string_view name, const Extern &item) {
    wasmtime_extern_t raw;
    Instance::cvt(item, raw);
    auto *error =
        wasmtime_linker_define(ptr.get(), cx.ptr, module.data(), module.size(),
                               name.data(), name.size(), &raw);
    if (error != nullptr) {
      return Error(error);
    }
    return std::monostate();
  }

  /// Defines WASI functions within this linker.
  ///
  /// Note that `Store::Context::set_wasi` must also be used for instantiated
  /// modules to have access to configured WASI state.
  Result<std::monostate> define_wasi() {
    auto *error = wasmtime_linker_define_wasi(ptr.get());
    if (error != nullptr) {
      return Error(error);
    }
    return std::monostate();
  }

  /// Defines all exports of the `instance` provided in this linker with the
  /// given module name of `name`.
  Result<std::monostate>
  define_instance(Store::Context cx, std::string_view name, Instance instance) {
    auto *error = wasmtime_linker_define_instance(
        ptr.get(), cx.ptr, name.data(), name.size(), &instance.instance);
    if (error != nullptr) {
      return Error(error);
    }
    return std::monostate();
  }

  /// Instantiates the module `m` provided within the store `cx` using the items
  /// defined within this linker.
  TrapResult<Instance> instantiate(Store::Context cx, const Module &m) {
    wasmtime_instance_t instance;
    wasm_trap_t *trap = nullptr;
    auto *error = wasmtime_linker_instantiate(ptr.get(), cx.ptr, m.ptr.get(),
                                              &instance, &trap);
    if (error != nullptr) {
      return TrapError(Error(error));
    }
    if (trap != nullptr) {
      return TrapError(Trap(trap));
    }
    return Instance(instance);
  }

  /// Defines instantiations of the module `m` within this linker under the
  /// given `name`.
  Result<std::monostate> module(Store::Context cx, std::string_view name,
                                const Module &m) {
    auto *error = wasmtime_linker_module(ptr.get(), cx.ptr, name.data(),
                                         name.size(), m.ptr.get());
    if (error != nullptr) {
      return Error(error);
    }
    return std::monostate();
  }

  /// Attempts to load the specified named item from this linker, returning
  /// `std::nullopt` if it was not defined.
  [[nodiscard]] std::optional<Extern>
  get(Store::Context cx, std::string_view module, std::string_view name) {
    wasmtime_extern_t item;
    if (wasmtime_linker_get(ptr.get(), cx.ptr, module.data(), module.size(),
                            name.data(), name.size(), &item)) {
      return Instance::cvt(item);
    }
    return std::nullopt;
  }

  /// Defines a new function in this linker in the style of the `Func`
  /// constructor.
  template <typename F,
            std::enable_if_t<
                std::is_invocable_r_v<Result<std::monostate, Trap>, F, Caller,
                                      Span<const Val>, Span<Val>>,
                bool> = true>
  Result<std::monostate> func_new(std::string_view module,
                                  std::string_view name, const FuncType &ty,
                                  F&& f) {

    auto *error = wasmtime_linker_define_func(
        ptr.get(), module.data(), module.length(), name.data(), name.length(),
        ty.ptr.get(), Func::raw_callback<std::remove_reference_t<F>>, std::make_unique<std::remove_reference_t<F>>(std::forward<F>(f)).release(),
        Func::raw_finalize<std::remove_reference_t<F>>);

    if (error != nullptr) {
      return Error(error);
    }

    return std::monostate();
  }

  /// Defines a new function in this linker in the style of the `Func::wrap`
  /// constructor.
  template <typename F,
            std::enable_if_t<WasmHostFunc<F>::Params::valid, bool> = true,
            std::enable_if_t<WasmHostFunc<F>::Results::valid, bool> = true>
  Result<std::monostate> func_wrap(std::string_view module,
                                   std::string_view name, F&& f) {
    using HostFunc = WasmHostFunc<F>;
    auto params = HostFunc::Params::types();
    auto results = HostFunc::Results::types();
    auto ty = FuncType::from_iters(params, results);
    auto *error = wasmtime_linker_define_func_unchecked(
        ptr.get(), module.data(), module.length(), name.data(), name.length(),
        ty.ptr.get(), Func::raw_callback_unchecked<std::remove_reference_t<F>>,
        std::make_unique<std::remove_reference_t<F>>(std::forward<F>(f)).release(), Func::raw_finalize<std::remove_reference_t<F>>);

    if (error != nullptr) {
      return Error(error);
    }

    return std::monostate();
  }

  /// Loads the "default" function, according to WASI commands and reactors, of
  /// the module named `name` in this linker.
  Result<Func> get_default(Store::Context cx, std::string_view name) {
    wasmtime_func_t item;
    auto *error = wasmtime_linker_get_default(ptr.get(), cx.ptr, name.data(),
                                              name.size(), &item);
    if (error != nullptr) {
      return Error(error);
    }
    return Func(item);
  }
};

} // namespace wasmtime

#endif // WASMTIME_HH
