#include <stdio.h>
#include <wizer.h>

class Test {
public:
  Test() : value(1) {
    printf("global constructor (should be the first printed line)\n");
  }
  ~Test() { printf("global destructor (should be the last printed line)\n"); }
  int value;
};

bool initialized = false;
int orig_value = 0;
Test t;

static void init_func() {
  // This should run after the ctor for `t`, and before `main`.
  orig_value = t.value;
  t.value = 2;
  initialized = true;
}

WIZER_INIT(init_func);

int main(int argc, char **argv) {
  if (!initialized)
    init_func();
  printf("argc (should not be baked into snapshot): %d\n", argc);
  printf("orig_value (should be 1): %d\n", orig_value);
  printf("t.value (should be 2): %d\n", t.value);
}
