#include <gtest/gtest.h>
#include <wasmtime.hh>
#include <wasmtime/config.hh>

using namespace wasmtime;

TEST(PoolAllocationConfig, Smoke) {
  PoolAllocationConfig config;
  config.max_unused_warm_slots(1);
  config.decommit_batch_size(2);
  config.async_stack_keep_resident(3);
  config.linear_memory_keep_resident(4);
  config.table_keep_resident(5);
  config.total_component_instances(6);
  config.max_component_instance_size(7);
  config.max_core_instances_per_component(8);
  config.max_memories_per_component(9);
  config.max_tables_per_component(10);
  config.total_memories(11);
  config.total_tables(12);
  config.total_stacks(13);
  config.total_core_instances(14);
  config.max_core_instance_size(15);
  config.max_tables_per_module(16);
  config.table_elements(17);
  config.max_memories_per_module(18);
  config.max_memory_size(19);
  config.total_gc_heaps(20);

  PoolAllocationConfig config2 = std::move(config);
  PoolAllocationConfig config3(std::move(config));
}

TEST(Config, Smoke) {
  Config config;
  config.debug_info(false);
  config.epoch_interruption(false);
  config.consume_fuel(false);
  config.max_wasm_stack(100);
  config.wasm_threads(false);
  config.wasm_tail_call(false);
  config.wasm_reference_types(false);
  config.wasm_function_references(false);
  config.wasm_gc(false);
  config.wasm_simd(false);
  config.wasm_relaxed_simd(false);
  config.wasm_relaxed_simd_deterministic(false);
  config.wasm_bulk_memory(false);
  config.wasm_multi_value(false);
  config.wasm_multi_memory(false);
  config.wasm_memory64(false);
  config.wasm_wide_arithmetic(false);
  config.wasm_component_model(false);
  config.strategy(Strategy::Auto);
  config.cranelift_debug_verifier(false);
  config.cranelift_opt_level(OptLevel::Speed);
  config.cranelift_nan_canonicalization(false);
  config.profiler(ProfilingStrategy::None);
  config.memory_reservation(0);
  config.memory_reservation_for_growth(0);
  config.memory_guard_size(0);
  config.memory_may_move(false);
  config.memory_init_cow(false);
  config.native_unwind_info(false);
  config.macos_use_mach_ports(false);
  config.cranelift_flag_enable("foo");
  config.cranelift_flag_set("foo", "bar");
  EXPECT_TRUE(config.cache_load_default());
  EXPECT_FALSE(config.cache_load("nonexistent"));

  PoolAllocationConfig pooling_config;
  config.pooling_allocation_strategy(pooling_config);

  Config config2 = std::move(config);
  Config config3(std::move(config));
}

struct MyMemoryCreator {
  struct Memory {
    std::vector<uint8_t> storage;

    uint8_t *get_memory(size_t *byte_size, size_t *byte_capacity) {
      *byte_size = storage.size();
      *byte_capacity = storage.capacity();
      return &storage[0];
    }

    Result<std::monostate> grow_memory(size_t new_size) {
      storage.resize(new_size, 0);
      return std::monostate();
    }
  };

  Result<Memory> new_memory(const MemoryType::Ref &ty, size_t minimum,
                            size_t maximum, size_t reserved_size_in_bytes,
                            size_t guard_size_in_bytes) {
    EXPECT_EQ(guard_size_in_bytes, 0);
    EXPECT_EQ(reserved_size_in_bytes, 0);

    Memory mem;
    mem.grow_memory(minimum).unwrap();
    return mem;
  }
};

TEST(Config, MemoryCreator) {
  Config config;
  config.memory_guard_size(0);
  config.memory_reservation(0);
  config.memory_reservation_for_growth(0);
  config.host_memory_creator(MyMemoryCreator());

  Engine engine(std::move(config));
  Module m =
      Module::compile(engine, "(module (memory (export \"x\") 1))").unwrap();

  Store store(engine);
  Instance i = Instance::create(store, m, {}).unwrap();
  Memory mem = std::get<Memory>(*i.get(store, "x"));
  {
    auto data = mem.data(store);
    EXPECT_EQ(data.size(), 65536);
    for (auto &i : data) {
      EXPECT_EQ(i, 0);
    }
  }

  EXPECT_EQ(mem.grow(store, 1).unwrap(), 1);

  {
    auto data = mem.data(store);
    EXPECT_EQ(data.size(), 65536 * 2);
    for (auto &i : data) {
      EXPECT_EQ(i, 0);
    }
  }
}
