/**
 * \file wasmtime/component/component.hh
 */

#ifndef WASMTIME_COMPONENT_COMPONENT_HH
#define WASMTIME_COMPONENT_COMPONENT_HH

#include <wasmtime/conf.h>

#ifdef WASMTIME_FEATURE_COMPONENT_MODEL

#include <memory>
#include <optional>
#include <string_view>
#include <vector>
#include <wasmtime/component/component.h>
#include <wasmtime/component/types/component.hh>
#include <wasmtime/engine.hh>
#include <wasmtime/error.hh>
#include <wasmtime/span.hh>
#include <wasmtime/wat.hh>

namespace wasmtime {
namespace component {

/**
 * \brief An index to an exported item within a particular component.
 *
 * This structure is acquired from a `Component` and used to lookup exports on
 * instances.
 */
class ExportIndex {
  WASMTIME_CLONE_WRAPPER(ExportIndex, wasmtime_component_export_index);
};

/**
 * \brief Representation of a compiled WebAssembly component.
 */
class Component {
  WASMTIME_CLONE_WRAPPER(Component, wasmtime_component);

#ifdef WASMTIME_FEATURE_COMPILER
  /**
   * \brief Compiles a component from the WebAssembly text format.
   *
   * This function will automatically use `wat2wasm` on the input and then
   * delegate to the #compile function.
   */
  static Result<Component> compile(Engine &engine, std::string_view wat) {
    auto wasm = wat2wasm(wat);
    if (!wasm) {
      return wasm.err();
    }
    auto bytes = wasm.ok();
    return compile(engine, bytes);
  }

  /**
   * \brief Compiles a component from the WebAssembly binary format.
   *
   * This function compiles the provided WebAssembly binary specified by `wasm`
   * within the compilation settings configured by `engine`. This method is
   * synchronous and will not return until the component has finished compiling.
   *
   * This function can fail if the WebAssembly binary is invalid or doesn't
   * validate (or similar). Note that this API does not compile WebAssembly
   * modules, which is done with `Module` instead of `Component`.
   */
  static Result<Component> compile(Engine &engine, Span<uint8_t> wasm) {
    wasmtime_component_t *ret = nullptr;
    auto *error =
        wasmtime_component_new(engine.capi(), wasm.data(), wasm.size(), &ret);
    if (error != nullptr) {
      return Error(error);
    }
    return Component(ret);
  }
#endif // WASMTIME_FEATURE_COMPILER

  /**
   * \brief Deserializes a previous list of bytes created with `serialize`.
   *
   * This function is intended to be much faster than `compile` where it uses
   * the artifacts of a previous compilation to quickly create an in-memory
   * component ready for instantiation.
   *
   * It is not safe to pass arbitrary input to this function, it is only safe to
   * pass in output from previous calls to `serialize`. For more information see
   * the Rust documentation -
   * https://docs.wasmtime.dev/api/wasmtime/struct.Module.html#method.deserialize
   */
  static Result<Component> deserialize(Engine &engine, Span<uint8_t> wasm) {
    wasmtime_component_t *ret = nullptr;
    auto *error = wasmtime_component_deserialize(engine.capi(), wasm.data(),
                                                 wasm.size(), &ret);
    if (error != nullptr) {
      return Error(error);
    }
    return Component(ret);
  }

  /**
   * \brief Deserializes a component from an on-disk file.
   *
   * This function is the same as `deserialize` except that it reads the data
   * for the serialized component from the path on disk. This can be faster than
   * the alternative which may require copying the data around.
   *
   * It is not safe to pass arbitrary input to this function, it is only safe to
   * pass in output from previous calls to `serialize`. For more information see
   * the Rust documentation -
   * https://docs.wasmtime.dev/api/wasmtime/struct.Module.html#method.deserialize
   */
  static Result<Component> deserialize_file(Engine &engine,
                                            const std::string &path) {
    wasmtime_component_t *ret = nullptr;
    auto *error =
        wasmtime_component_deserialize_file(engine.capi(), path.c_str(), &ret);
    if (error != nullptr) {
      return Error(error);
    }
    return Component(ret);
  }

#ifdef WASMTIME_FEATURE_COMPILER
  /**
   * \brief Serializes this component to a list of bytes.
   *
   * The returned bytes can then be used to later pass to `deserialize` to
   * quickly recreate this component in a different process perhaps.
   */
  Result<std::vector<uint8_t>> serialize() const {
    wasm_byte_vec_t bytes;
    auto *error = wasmtime_component_serialize(ptr.get(), &bytes);
    if (error != nullptr) {
      return Error(error);
    }
    std::vector<uint8_t> ret;
    Span<uint8_t> raw(reinterpret_cast<uint8_t *>(bytes.data), bytes.size);
    ret.assign(raw.begin(), raw.end());
    wasm_byte_vec_delete(&bytes);
    return ret;
  }
#endif // WASMTIME_FEATURE_COMPILER

  /**
   * \brief Returns the export index for the export named `name` in this
   * component.
   *
   * The `instance` argument is an optionally provided index which is the
   * instance under which the `name` should be looked up.
   */
  std::optional<ExportIndex> export_index(ExportIndex *instance,
                                          std::string_view name) {
    auto ret = wasmtime_component_get_export_index(
        capi(), instance ? instance->capi() : nullptr, name.data(),
        name.size());
    if (ret) {
      return ExportIndex(ret);
    }
    return std::nullopt;
  };

  /// \brief Returns the type of this component.
  ComponentType type() const {
    return ComponentType(wasmtime_component_type(ptr.get()));
  }
};

} // namespace component
} // namespace wasmtime

#endif // WASMTIME_FEATURE_COMPONENT_MODEL

#endif // WASMTIME_COMPONENT_COMPONENT_HH
