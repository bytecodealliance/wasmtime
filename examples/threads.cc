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
#include <sstream>
#include <thread>
#include <vector>
#include <mutex>
#include <unordered_map>
#include <wasmtime.hh>

using namespace wasmtime;

std::string readFile(const char *name) {
  std::ifstream watFile;
  watFile.open(name);
  std::stringstream strStream;
  strStream << watFile.rdbuf();
  return strStream.str();
}

const int N_THREADS = 10;
const int N_REPS = 3;

// Synchronization for clean output and mapping to small numeric thread ids.
static std::mutex print_mutex;
static std::unordered_map<std::thread::id, int> id_map;
static std::atomic<int> next_id{15}; // Rust sample started at ThreadId(15) in captured run

int thread_number() {
  std::lock_guard<std::mutex> lock(print_mutex);
  auto tid = std::this_thread::get_id();
  auto it = id_map.find(tid);
  if (it != id_map.end()) return it->second;
  int id = next_id++;
  id_map.emplace(tid, id);
  return id;
}

void print_line(const std::string &s) {
  std::lock_guard<std::mutex> lock(print_mutex);
  std::cout << s << '\n';
}

void run_worker(Engine engine, Module module) {
  Store store(engine);
  for (int i = 0; i < N_REPS; i++) {
    print_line("Instantiating module...");
    Func hello_func = Func::wrap(store, []() {
      int id = thread_number();
      print_line("> Hello from ThreadId(" + std::to_string(id) + ")");
    });
    auto instance_res = Instance::create(store, module, {hello_func});
    if (!instance_res) {
      print_line("> Error instantiating module!");
      return;
    }
    Instance instance = instance_res.unwrap();
    Func run = std::get<Func>(*instance.get(store, "run"));
    print_line("Executing...");
    run.call(store, {}).unwrap();
    std::this_thread::sleep_for(std::chrono::milliseconds(100));
  }

  // Move store to a new thread once.
  int old_id = thread_number();
  print_line("> Moving ThreadId(" + std::to_string(old_id) + ") to a new thread");
  auto handle = std::thread([store = std::move(store), module]() mutable {
    Func hello_func = Func::wrap(store, []() {
      int id = thread_number();
      print_line("> Hello from ThreadId(" + std::to_string(id) + ")");
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
  for (int i = 0; i < N_THREADS; i++) threads.emplace_back(run_worker, engine, module);
  for (auto &t : threads) t.join();
  return 0;
}
