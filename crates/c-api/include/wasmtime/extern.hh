/**
 * \file wasmtime/extern.hh
 */

#ifndef WASMTIME_EXTERN_HH
#define WASMTIME_EXTERN_HH

#include <variant>
#include <wasmtime/extern.h>

namespace wasmtime {

class Global;
class Func;
class Memory;
class Table;

/// \typedef Extern
/// \brief Representation of an external WebAssembly item
typedef std::variant<Func, Global, Memory, Table> Extern;

} // namespace wasmtime

#endif // WASMTIME_EXTERN_HH
