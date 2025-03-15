// clang-format off
// clang -o codegen-optimized.wasm -target wasm32-unknown-wasip1 -g codegen-optimized.cpp
// clang-format on

// Make sure to adjust the break locations in lldb.rs when modifying the test.
#define BREAKPOINT

int InvalidateRegisters() {
  int r1 = -1;
  int r2 = -2;
  int r3 = -3;
  int r4 = -4;
  int r5 = -5;
  int r6 = -6;
  int r7 = -7;
  int r8 = -8;
  return r1 + r2 + r3 + r4 + r5 + r6 + r7 + r8;
}

void VariableWithSimpleLifetime() {
  // Here we are testing that the value range of "x" is correctly recorded
  // as being bound by a loclist that is shorted than the entire method body,
  // even as the location can be represented with a single DWARF expression.
  int x = 42;
  InvalidateRegisters();
  BREAKPOINT;
}

void InitializeTest() {}

int main(int argc, char *argv[]) {
  InitializeTest();
  VariableWithSimpleLifetime();
  return 0;
}
