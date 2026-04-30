/**
 * \file wasmtime/types/structref.hh
 */

#ifndef WASMTIME_TYPES_STRUCTREF_HH
#define WASMTIME_TYPES_STRUCTREF_HH

#include <wasmtime/types/_structref_class.hh>
#include <wasmtime/types/_val_class.hh>

namespace wasmtime {

inline StorageType::StorageType(const ValType &ty) {
  this->ty.kind = WASMTIME_STORAGE_TYPE_KIND_VALTYPE;
  this->ty.valtype = wasm_valtype_copy(ty.capi());
}

} // namespace wasmtime

#endif // WASMTIME_TYPES_STRUCTREF_HH
