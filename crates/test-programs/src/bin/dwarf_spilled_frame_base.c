// Originally built using WASI SDK 20.0, "clang spilled_frame_base.c -o
// spilled_frame_base.wasm -g -target wasm32-wasi"
void func_11(int x, int y, int z, int a, int b, int c, int d, int e, int f,
             int g, int h) {}
void func_12(int x, int y, int z, int a, int b, int c, int d, int e, int f,
             int g, int h, int i) {}
void func_13(int x, int y, int z, int a, int b, int c, int d, int e, int f,
             int g, int h, int i, int j) {}
void func_14(int x, int y, int z, int a, int b, int c, int d, int e, int f,
             int g, int h, int i, int j, int k) {}

int main() {
  int i = 1;

  func_11(55, 56, 57, 58, 59, 60, 61, 62, 63, 64, 65);
  func_12(66, 67, 68, 69, 70, 71, 72, 73, 74, 75, 76, 77);
  func_13(78, 79, 80, 81, 82, 83, 84, 85, 86, 87, 88, 89, 90);
  func_14(91, 92, 93, 94, 95, 96, 97, 98, 99, 100, 101, 102, 103, 104);

  return i + i;
}
