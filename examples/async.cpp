/*
Example of instantiating of the WebAssembly module and invoking its exported
function.

You can compile and run this example on Linux with:

   cargo build --release -p wasmtime-c-api
   c++ examples/async.cpp \
       -I crates/c-api/include \
       -I crates/c-api/wasm-c-api/include \
       target/release/libwasmtime.a \
       -std=c++17 \
       -lpthread -ldl -lm \
       -o async
   ./async

Note that on Windows and macOS the command will be similar, but you'll need
to tweak the `-lpthread` and such annotations.

You can also build using cmake:

mkdir build && cd build && cmake .. && cmake --build . --target wasmtime-async
*/

#include "wasmtime/async.h"
#include <assert.h>
#include <chrono>
#include <condition_variable>
#include <cstdlib>
#include <fstream>
#include <future>
#include <iostream>
#include <memory>
#include <mutex>
#include <optional>
#include <sstream>
#include <stdio.h>
#include <stdlib.h>
#include <streambuf>
#include <string>
#include <thread>
#include <wasm.h>
#include <wasmtime.h>

using namespace std::chrono_literals;

namespace {

template <typename T, auto fn> struct deleter {
  void operator()(T *ptr) { fn(ptr); }
};
template <typename T, auto fn>
using handle = std::unique_ptr<T, deleter<T, fn>>;

void exit_with_error(std::string_view msg, wasmtime_error_t *err,
                     wasm_trap_t *trap) {
  std::cerr << "error: " << msg << std::endl;
  wasm_byte_vec_t error_message;
  if (err) {
    wasmtime_error_message(err, &error_message);
  } else {
    wasm_trap_message(trap, &error_message);
  }
  std::cerr << std::string_view(error_message.data, error_message.size)
            << std::endl;
  wasm_byte_vec_delete(&error_message);
  std::exit(1);
}

handle<wasm_engine_t, wasm_engine_delete> create_engine() {
  wasm_config_t *config = wasm_config_new();
  assert(config != nullptr);
  wasmtime_config_async_support_set(config, true);
  wasmtime_config_consume_fuel_set(config, true);
  handle<wasm_engine_t, wasm_engine_delete> engine;
  // this takes ownership of config
  engine.reset(wasm_engine_new_with_config(config));
  assert(engine);
  return engine;
}

handle<wasmtime_store_t, wasmtime_store_delete>
create_store(wasm_engine_t *engine) {
  handle<wasmtime_store_t, wasmtime_store_delete> store;
  store.reset(wasmtime_store_new(engine, nullptr, nullptr));
  assert(store);
  return store;
}

handle<wasmtime_linker_t, wasmtime_linker_delete>
create_linker(wasm_engine_t *engine) {
  handle<wasmtime_linker_t, wasmtime_linker_delete> linker;
  linker.reset(wasmtime_linker_new(engine));
  assert(linker);
  return linker;
}

handle<wasmtime_module_t, wasmtime_module_delete>
compile_wat_module_from_file(wasm_engine_t *engine,
                             const std::string &filename) {
  std::ifstream t(filename);
  std::stringstream buffer;
  buffer << t.rdbuf();
  if (t.bad()) {
    std::cerr << "error reading file: " << filename << std::endl;
    std::exit(1);
  }
  const std::string &content = buffer.str();
  wasm_byte_vec_t wasm_bytes;
  handle<wasmtime_error_t, wasmtime_error_delete> error{
      wasmtime_wat2wasm(content.data(), content.size(), &wasm_bytes)};
  if (error) {
    exit_with_error("failed to parse wat", error.get(), nullptr);
  }
  wasmtime_module_t *mod_ptr = nullptr;
  error.reset(wasmtime_module_new(engine,
                                  reinterpret_cast<uint8_t *>(wasm_bytes.data),
                                  wasm_bytes.size, &mod_ptr));
  wasm_byte_vec_delete(&wasm_bytes);
  handle<wasmtime_module_t, wasmtime_module_delete> mod{mod_ptr};
  if (!mod) {
    exit_with_error("failed to compile module", error.get(), nullptr);
  }
  return mod;
}

struct printer_thread_state {
  std::promise<int32_t> value_to_print;
  std::promise<void> print_finished;
  std::optional<std::future<void>> print_finished_future;

  bool result_is_pending() {
    return print_finished_future.has_value() &&
           print_finished_future->wait_for(0s) != std::future_status::ready;
  }
};

printer_thread_state printer_state;

bool poll_print_finished_state(void *env, wasmtime_caller_t *,
                               wasm_trap_t **trap) {
  std::cout << "polling async host function result" << std::endl;
  // Don't block, just poll the future state.
  if (printer_state.result_is_pending()) {
    return false;
  }
  try {
    printer_state.print_finished_future->get();
  } catch (const std::exception &ex) {
    std::string_view msg = ex.what();
    *trap = wasmtime_trap_new(msg.data(), msg.size());
  }
  printer_state.print_finished_future = std::nullopt;
  return true;
}
} // namespace

int main() {
  // A thread that will async perform host function calls.
  std::thread printer_thread([]() {
    int32_t value_to_print = printer_state.value_to_print.get_future().get();
    std::cout << "recieved value to print!" << std::endl;
    std::this_thread::sleep_for(1s);
    std::cout << "printing: " << value_to_print << std::endl;
    std::this_thread::sleep_for(1s);
    std::cout << "signaling that value is printed" << std::endl;
    printer_state.print_finished.set_value();
  });

  handle<wasmtime_error_t, wasmtime_error_delete> error;

  auto engine = create_engine();
  auto store = create_store(engine.get());
  // This pointer is unowned.
  auto *context = wasmtime_store_context(store.get());
  // Configure the store to periodically yield control
  wasmtime_context_out_of_fuel_async_yield(context,
                                           /*injection_count=*/10,
                                           /*fuel_to_inject=*/10000);

  auto compiled_module =
      compile_wat_module_from_file(engine.get(), "examples/async.wat");
  auto linker = create_linker(engine.get());
  constexpr std::string_view host_module_name = "host";
  constexpr std::string_view host_func_name = "print";

  // Declare our async host function's signature and definition.
  wasm_valtype_vec_t arg_types;
  wasm_valtype_vec_t result_types;
  wasm_valtype_vec_new_uninitialized(&arg_types, 1);
  arg_types.data[0] = wasm_valtype_new_i32();
  wasm_valtype_vec_new_empty(&result_types);
  handle<wasm_functype_t, wasm_functype_delete> functype{
      wasm_functype_new(&arg_types, &result_types)};

  error.reset(wasmtime_linker_define_async_func(
      linker.get(), host_module_name.data(), host_module_name.size(),
      host_func_name.data(), host_func_name.size(), functype.get(),
      [](void *env, wasmtime_caller_t *caller, const wasmtime_val_t *args,
         size_t nargs, wasmtime_val_t *results, size_t nresults) {
        std::cout << "invoking async host function" << std::endl;
        printer_state.print_finished_future =
            printer_state.print_finished.get_future();
        printer_state.value_to_print.set_value(args[0].of.i32);
        return new wasmtime_async_continuation_t{.callback =
                                                     &poll_print_finished_state,
                                                 .env = nullptr,
                                                 .finalizer = nullptr};
      },
      /*env=*/nullptr, /*finalizer=*/nullptr));
  if (error) {
    exit_with_error("failed to define host function", error.get(), nullptr);
  }

  // Now instantiate our module using the linker.
  handle<wasmtime_call_future_t, wasmtime_call_future_delete> call_future;
  wasm_trap_t *trap_ptr = nullptr;
  wasmtime_error_t *error_ptr = nullptr;
  wasmtime_instance_t instance;
  call_future.reset(wasmtime_linker_instantiate_async(
      linker.get(), context, compiled_module.get(), &instance, &trap_ptr,
      &error_ptr));
  while (!wasmtime_call_future_poll(call_future.get())) {
    std::cout << "yielding instantiation!" << std::endl;
  }
  error.reset(error_ptr);
  handle<wasm_trap_t, wasm_trap_delete> trap{trap_ptr};
  if (error || trap) {
    exit_with_error("failed to instantiate module", error.get(), trap.get());
  }
  // delete call future - it's no longer needed
  call_future = nullptr;

  // Grab our exported function
  constexpr std::string_view guest_func_name = "print_fibonacci";
  wasmtime_extern_t guest_func_extern;
  bool found =
      wasmtime_instance_export_get(context, &instance, guest_func_name.data(),
                                   guest_func_name.size(), &guest_func_extern);
  assert(found);
  assert(guest_func_extern.kind == WASMTIME_EXTERN_FUNC);

  // Now call our print_fibonacci function with n=15
  std::array<wasmtime_val_t, 1> args;
  args[0].kind = WASMTIME_I32;
  args[0].of.i32 = 15;
  std::array<wasmtime_val_t, 0> results;
  call_future.reset(wasmtime_func_call_async(
      context, &guest_func_extern.of.func, args.data(), args.size(),
      results.data(), results.size(), &trap_ptr, &error_ptr));
  // Poll the execution of the call. This can yield control back if there is an
  // async host call or if we ran out of fuel.
  while (!wasmtime_call_future_poll(call_future.get())) {
    // if we have an async host call pending then wait for that future to finish
    // before continuing.
    if (printer_state.result_is_pending()) {
      std::cout << "waiting for async host function to complete" << std::endl;
      printer_state.print_finished_future->wait();
      std::cout << "async host function completed" << std::endl;
      continue;
    }
    // Otherwise we ran out of fuel and yielded.
    std::cout << "yield!" << std::endl;
  }
  // Extract if there were failures or traps after poll returns that execution
  // completed.
  error.reset(error_ptr);
  trap.reset(trap_ptr);
  if (error || trap) {
    exit_with_error("running guest function failed", error.get(), trap.get());
  }
  // At this point, if our host function returned results they would be
  // available in the `results` array.

  // Join our thread and exit.
  printer_thread.join();
  return 0;
}
