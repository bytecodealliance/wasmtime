#include "generic.h"

int SomeClass::SatelliteFunction(int x) {
  x *= 2;
  debug_break();
  return x;
}
