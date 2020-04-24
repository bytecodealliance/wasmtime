void bar();
void baz();
int foo(int b, int a) {
  if (b) {
    baz();
    return a + 3;
  }
  bar();
  return a + 4;
}
