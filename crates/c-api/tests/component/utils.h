#pragma once

#define CHECK_ERR(err)                                                         \
  do {                                                                         \
    if (err) {                                                                 \
      auto msg = wasm_name_t{};                                                \
      wasmtime_error_message(err, &msg);                                       \
      EXPECT_EQ(err, nullptr) << std::string_view{msg.data, msg.size};         \
    }                                                                          \
  } while (false)
