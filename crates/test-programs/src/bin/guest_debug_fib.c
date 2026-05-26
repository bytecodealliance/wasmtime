//! flags = []

#include <stdio.h>

int fib(int n) {
  int t, a = 0, b = 1;
  for (int i = 0; i < n; i++) {
    t = a;
    a = b;
    b += t;
  }
  return b;
}

int main() {
  int result = fib(5);
  printf("fib(5) = %d\n", result);
  return 0;
}
