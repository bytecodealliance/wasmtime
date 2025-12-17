/*
Example of instantiating of the WebAssembly module and invoking its exported
function in a separate thread.

You can build the example using CMake:

mkdir build && (cd build && cmake .. && \
  cmake --build . --target wasmtime-threads-cpp)

And then run it:

build/wasmtime-threads-cpp
*/

#include <fstream>
#include <iostream>
#include <mutex>
#include <sstream>
#include <thread>
#include <unordered_map>
#include <vector>
#include <wasmtime.hh>

using namespace wasmtime;

std::string readFile(const char *name) {
  std::ifstream watFile;
  watFile.open(name);
  std::stringstream strStream;
  strStream << watFile.rdbuf();
  return strStream.str();
}

#if defined(WASMTIME_ASAN)
const int N_THREADS = 1;
const int N_REPS = 1;
#else
const int N_THREADS = 10;
const int N_REPS = 3;
#endif

std::mutex print_mutex;

void run_worker(Engine engine, Module module) {
  std::thread::id id = std::this_thread::get_id();
  Store store(engine);
  for (int i = 0; i < N_REPS; i++) {
    {
      std::lock_guard<std::mutex> lock(print_mutex);
      std::cout << "Instantiating module...\n";
    }
    Func hello_func = Func::wrap(store, []() {
      std::lock_guard<std::mutex> lock(print_mutex);
      std::thread::id id = std::this_thread::get_id();
      std::cout << "> Hello from ThreadId(" << id << ")\n";
    });
    auto instance_res = Instance::create(store, module, {hello_func});
    if (!instance_res) {
      std::cout << "> Error instantiating module!\n";
      return;
    }
    Instance instance = instance_res.unwrap();
    Func run = std::get<Func>(*instance.get(store, "run"));
    {
      std::lock_guard<std::mutex> lock(print_mutex);
      std::cout << "Executing...\n";
    }
    run.call(store, {}).unwrap();
    std::this_thread::sleep_for(std::chrono::milliseconds(100));
  }

  // Move store to a new thread once.
  {
    std::lock_guard<std::mutex> lock(print_mutex);
    std::cout << "> Moving (" << id << ") to a new thread\n";
  }
  auto handle = std::thread([store = std::move(store), module]() mutable {
    Func hello_func = Func::wrap(store, []() {
      std::lock_guard<std::mutex> lock(print_mutex);
      std::thread::id id = std::this_thread::get_id();
      std::cout << "> Hello from ThreadId(" << id << ")\n";
    });
    Instance instance = Instance::create(store, module, {hello_func}).unwrap();
    Func run = std::get<Func>(*instance.get(store, "run"));
    run.call(store, {}).unwrap();
  });
  handle.join();
}

int main() {
  std::cout << "Initializing...\n";

  Engine engine;
  auto wat = readFile("examples/threads.wat");
  Module module = Module::compile(engine, wat).unwrap();

  std::vector<std::thread> threads;
  threads.reserve(N_THREADS);
  for (int i = 0; i < N_THREADS; i++)
    threads.emplace_back(run_worker, engine, module);
  for (auto &t : threads)
    t.join();
  return 0;
}
