//! flags = ['-O2']

// clang-format off
// clang -o codegen-optimized.wasm -target wasm32-unknown-wasip1 -g -O2 codegen-optimized.cpp

// Make sure to adjust the break locations in lldb.rs when modifying this test.
#define BREAKPOINT
#define NOINLINE __attribute__((noinline))

NOINLINE int NoInlineSideEffect() {
  volatile int x = 1;
  return x;
}
NOINLINE int NoInlineSideEffect_TwoArgs(int a, int b) {
  volatile int x[] = {1, 2, 3};
  return x[(a + b) >> 16];
}

NOINLINE int VariableRanges_SingleVRegBrokenUp(int b) {
  if (b < 0) {
    __builtin_trap(); // This will split the live range of 'b'.
  }
  NoInlineSideEffect(); BREAKPOINT;
  return b;
}

NOINLINE int VariableRanges_SingleVRegRegReused(int b) {
  int t = b & 420;
  NoInlineSideEffect_TwoArgs(t, b); BREAKPOINT;
  return 0;
}

NOINLINE void InitializeTest(volatile int *x) {
  *x = 42; // Have something to set a breakpoint on.
}

int main(int argc, char *argv[]) {
  volatile int x;
  InitializeTest(&x);
  VariableRanges_SingleVRegBrokenUp(x++);
  VariableRanges_SingleVRegRegReused(x++);
  return 0;
}
