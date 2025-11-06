/**
 * \file wasmtime/types/export.hh
 */

#ifndef WASMTIME_TYPES_EXPORT_HH
#define WASMTIME_TYPES_EXPORT_HH

#include <string_view>
#include <wasm.h>
#include <wasmtime/helpers.hh>

namespace wasmtime {

/**
 * \brief Type information about a WebAssembly export
 */
class ExportType {
/// bridge between wasm.h and wasmtime.h conventions
#define wasm_exporttype_clone wasm_exporttype_copy
  WASMTIME_CLONE_WRAPPER(ExportType, wasm_exporttype);
#undef wasm_exporttype_clone

  /// \brief Non-owning reference to an `ExportType`.
  ///
  /// Note to get type information you can use `ExternType::from_export`.
  class Ref {
    friend class ExternType;

    const wasm_exporttype_t *ptr;

    const wasm_externtype_t *raw_type() { return wasm_exporttype_type(ptr); }

  public:
    /// Creates a new reference from the raw underlying C API representation.
    Ref(const wasm_exporttype_t *ptr) : ptr(ptr) {}

    /// Returns the name of this export.
    std::string_view name() {
      const auto *name = wasm_exporttype_name(ptr);
      return std::string_view(name->data, name->size);
    }
  };

  /// Returns the `Ref` describing this export.
  Ref ref() const { return Ref(ptr.get()); }

  /// An owned list of `ExportType` instances.
  class List {
    friend class Module;
    wasm_exporttype_vec_t list;

  public:
    /// Creates an empty list
    List() : list{} {
      list.size = 0;
      list.data = nullptr;
    }
    List(const List &other) = delete;
    /// Moves another list into this one.
    List(List &&other) noexcept : list(other.list) { other.list.size = 0; }
    ~List() {
      if (list.size > 0) {
        wasm_exporttype_vec_delete(&list);
      }
    }

    List &operator=(const List &other) = delete;
    /// Moves another list into this one.
    List &operator=(List &&other) noexcept {
      std::swap(list, other.list);
      return *this;
    }

    /// Iterator type, which is a list of non-owning `ExportType::Ref`
    /// instances.
    typedef const Ref *iterator;
    /// Returns the start of iteration.
    iterator begin() const {
      return reinterpret_cast<iterator>(&list.data[0]); // NOLINT
    }
    /// Returns the end of iteration.
    iterator end() const {
      return reinterpret_cast<iterator>(&list.data[list.size]); // NOLINT
    }
    /// Returns the size of this list.
    size_t size() const { return list.size; }
  };
};

}; // namespace wasmtime

#endif // WASMTIME_TYPES_EXPORT_HH
