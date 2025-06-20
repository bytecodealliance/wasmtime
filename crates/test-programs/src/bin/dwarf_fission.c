//! flags = ['-gsplit-dwarf', '-gdwarf-5', '-gpubnames']
//! dwp = true

#include <stdio.h>

int main() {
  int i = 1;
  i++;
  return i - 2;
}
