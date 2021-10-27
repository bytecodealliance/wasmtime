int bar(int a)
{
  int b[50];
  b[0] = a;
  b[29] = a;
  return a;
}

int baz(int a);

__attribute__((export_name("foo"))) int foo()
{
  return baz(10);
}

__attribute__((noinline)) int baz(int a)
{
  return a + 5;
}
