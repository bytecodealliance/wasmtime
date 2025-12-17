/**
 * \file wasmtime/memory.hh
 */

#ifndef WASMTIME_MEMORY_HH
#define WASMTIME_MEMORY_HH

#include <wasmtime/error.hh>
#include <wasmtime/memory.h>
#include <wasmtime/span.hh>
#include <wasmtime/store.hh>
#include <wasmtime/types/memory.hh>

namespace wasmtime {

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

  /// Returns the size of a page, in bytes, for this memory.
  ///
  /// WebAssembly memories are made up of a whole number of pages, so the byte
  /// size will always be a multiple of their page size. Different Wasm memories
  /// may have different page sizes.
  ///
  /// By default this is 64KiB (aka `0x10000`, `2**16`, `1<<16`, or `65536`)
  /// but [the custom-page-sizes proposal] allows opting into a page size of
  /// `1`. Future extensions might allow any power of two as a page size.
  ///
  /// [the custom-page-sizes proposal]:
  /// https://github.com/WebAssembly/custom-page-sizes
  uint64_t page_size(Store::Context cx) const {
    return wasmtime_memory_page_size(cx.ptr, &memory);
  }

  /// Returns the log2 of this memory's page size, in bytes.
  ///
  /// WebAssembly memories are made up of a whole number of pages, so the byte
  /// size will always be a multiple of their page size. Different Wasm memories
  /// may have different page sizes.
  ///
  /// By default the page size is 64KiB (aka `0x10000`, `2**16`, `1<<16`, or
  /// `65536`) but [the custom-page-sizes proposal] allows opting into a page
  /// size of `1`. Future extensions might allow any power of two as a page
  /// size.
  ///
  /// [the custom-page-sizes proposal]:
  /// https://github.com/WebAssembly/custom-page-sizes
  uint8_t page_size_log2(Store::Context cx) const {
    return wasmtime_memory_page_size_log2(cx.ptr, &memory);
  }

  /// Returns the raw underlying C API memory this is using.
  const wasmtime_memory_t &capi() const { return memory; }
};

} // namespace wasmtime

#endif // WASMTIME_MEMORY_HH
