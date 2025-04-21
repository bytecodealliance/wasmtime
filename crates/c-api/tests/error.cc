#include <wasmtime/error.hh>
#include <gtest/gtest.h>

using namespace wasmtime;

TEST(Result, Simple) {
  Result<int> ok_result(1);
  EXPECT_TRUE(ok_result);
  EXPECT_EQ(ok_result.ok(), 1);
  EXPECT_EQ(ok_result.unwrap(), 1);

  Result<int, std::string> err_result("x");
  EXPECT_FALSE(err_result);
  EXPECT_EQ(err_result.err(), "x");
}

TEST(Error, Simple) {
  Error err("hello");
  EXPECT_EQ(err.message(), "hello");
  EXPECT_FALSE(err.i32_exit());
}
