/*
Example of instantiating of the WebAssembly module and invoking its exported
function.

You can compile and run this example on Linux with:

   cargo build --release -p wasmtime-c-api
   c++ examples/async.cpp \
       -I crates/c-api/include \
       target/release/libwasmtime.a \
       -std=c++11 \
       -lpthread -ldl -lm \
       -o async
   ./async

Note that on Windows and macOS the command will be similar, but you'll need
to tweak the `-lpthread` and such annotations.

You can also build using cmake:

mkdir build && cd build && cmake .. && cmake --build . --target wasmtime-async
*/

#include <array>
#include <assert.h>
#include <chrono>
#include <cstdlib>
#include <fstream>
#include <future>
#include <iostream>
#include <memory>
#include <optional>
#include <sstream>
#include <streambuf>
#include <string>
#include <thread>
#include <wasmtime.h>

namespace {

template <typename T, void (*fn)(T *)> struct deleter {
  void operator()(T *ptr) { fn(ptr); }
};
template <typename T, void (*fn)(T *)>
using handle = std::unique_ptr<T, deleter<T, fn>>;

void exit_with_error(std::string msg, wasmtime_error_t *err,
                     wasm_trap_t *trap) {
  std::cerr << "error: " << msg << std::endl;
  wasm_byte_vec_t error_message;
  if (err) {
    wasmtime_error_message(err, &error_message);
  } else {
    wasm_trap_message(trap, &error_message);
  }
  std::cerr << std::string(error_message.data, error_message.size) << std::endl;
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

class printer_thread_state {
public:
  void set_value_to_print(int32_t v) {
    _print_finished_future = _print_finished.get_future();
    _value_to_print.set_value(v);
  }
  int32_t get_value_to_print() { return _value_to_print.get_future().get(); }

  bool print_is_pending() const {
    return _print_finished_future.valid() &&
           _print_finished_future.wait_for(std::chrono::seconds(0)) !=
               std::future_status::ready;
  }
  void wait_for_print_result() const { _print_finished_future.wait(); }
  void get_print_result() { _print_finished_future.get(); }
  void set_print_success() { _print_finished.set_value(); }

private:
  std::promise<int32_t> _value_to_print;
  std::promise<void> _print_finished;
  std::future<void> _print_finished_future;
};

printer_thread_state printer_state;

struct async_call_env {
  wasm_trap_t **trap_ret;
};

bool poll_print_finished_state(void *env) {
  std::cout << "polling async host function result" << std::endl;
  auto *async_env = static_cast<async_call_env *>(env);
  // Don't block, just poll the future state.
  if (printer_state.print_is_pending()) {
    return false;
  }
  try {
    printer_state.get_print_result();
  } catch (const std::exception &ex) {
    std::string msg = ex.what();
    *async_env->trap_ret = wasmtime_trap_new(msg.data(), msg.size());
  }
  return true;
}
} // namespace

int main() {
  // A thread that will async perform host function calls.
  std::thread printer_thread([]() {
    int32_t value_to_print = printer_state.get_value_to_print();
    std::cout << "received value to print!" << std::endl;
    std::this_thread::sleep_for(std::chrono::milliseconds(500));
    std::cout << "printing: " << value_to_print << std::endl;
    std::this_thread::sleep_for(std::chrono::milliseconds(500));
    std::cout << "signaling that value is printed" << std::endl;
    printer_state.set_print_success();
  });

  handle<wasmtime_error_t, wasmtime_error_delete> error;

  auto engine = create_engine();
  auto store = create_store(engine.get());
  // This pointer is unowned.
  auto *context = wasmtime_store_context(store.get());
  // Configure the store to periodically yield control
  wasmtime_context_set_fuel(context, 100000);
  wasmtime_context_fuel_async_yield_interval(context, /*interval=*/10000);

  auto compiled_module =
      compile_wat_module_from_file(engine.get(), "examples/async.wat");

  auto linker = create_linker(engine.get());
  static std::string host_module_name = "host";
  static std::string host_func_name = "print";

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
      [](void *, wasmtime_caller_t *, const wasmtime_val_t *args, size_t,
         wasmtime_val_t *, size_t, wasm_trap_t **trap_ret,
         wasmtime_async_continuation_t *continuation_ret) {
        std::cout << "invoking async host function" << std::endl;
        printer_state.set_value_to_print(args[0].of.i32);

        continuation_ret->callback = &poll_print_finished_state;
        continuation_ret->env = new async_call_env{trap_ret};
        continuation_ret->finalizer = [](void *env) {
          std::cout << "deleting async_call_env" << std::endl;
          delete static_cast<async_call_env *>(env);
        };
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
  // delete the linker now that we've created our instance
  linker = nullptr;

  // Grab our exported function
  static std::string guest_func_name = "print_fibonacci";
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
    if (printer_state.print_is_pending()) {
      std::cout << "waiting for async host function to complete" << std::endl;
      printer_state.wait_for_print_result();
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
  call_future = nullptr;
  // At this point, if our host function returned results they would be
  // available in the `results` array.
  std::cout << "async function call complete!" << std::endl;

  // Join our thread and exit.
  printer_thread.join();
  return 0;
}
