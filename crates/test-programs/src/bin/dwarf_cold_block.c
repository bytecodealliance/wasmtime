struct myfile {
  int data;
  void (*f)();
} myfile;

void f() {}

int foo(struct myfile *f1) {
  f1->f();
  if (f1->data == 42)
    return 0;
  return 1;
}

int main() {
  struct myfile f1;
  f1.f = &f;
  f1.data = 42;
  return foo(&f1);
}
