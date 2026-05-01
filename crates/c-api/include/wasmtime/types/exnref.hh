/**
 * \file wasmtime/types/exnref.hh
 */

#ifndef WASMTIME_TYPES_EXNREF_HH
#define WASMTIME_TYPES_EXNREF_HH

#include <initializer_list>
#include <memory>
#include <vector>
#include <wasmtime/engine.hh>
#include <wasmtime/error.hh>
#include <wasmtime/types/_val_class.hh>
#include <wasmtime/types/exnref.h>
#include <wasmtime/types/tag.hh>

namespace wasmtime {

/**
 * \brief Owned handle to a WebAssembly exception type definition.
 */
class ExnType {
/// Workaround for slightly different naming conventions
#define wasmtime_exn_type_clone wasmtime_exn_type_copy
  WASMTIME_CLONE_WRAPPER(ExnType, wasmtime_exn_type)
#undef wasmtime_exn_type_clone

public:
  /// Creates a new exception type with the given parameter types.
  static Result<ExnType> create(const Engine &engine,
                                const std::initializer_list<ValType> &params) {
    std::vector<const wasm_valtype_t *> tmp;
    for (const auto &param : params)
      tmp.push_back(param.capi());
    wasm_valtype_vec_t params_vec;
    params_vec.data = const_cast<wasm_valtype_t **>(tmp.data());
    params_vec.size = tmp.size();
    wasmtime_exn_type_t *result = nullptr;
    auto error = wasmtime_exn_type_new(engine.capi(), &params_vec, &result);
    if (error)
      return Error(error);
    return ExnType(result);
  }

  /// Returns the tag type associated with this exception type.
  TagType tag_type() const {
    wasm_tagtype_t *raw = wasmtime_exn_type_tag_type(capi());
    auto result = TagType(TagType::Ref(raw));
    wasm_tagtype_delete(raw);
    return result;
  }
};

} // namespace wasmtime

#endif // WASMTIME_TYPES_EXNREF_HH
