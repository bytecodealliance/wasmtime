// clang generic.cpp generic-satellite.cpp -o generic.wasm -g -target
// wasm32-unknown-wasip1
//
#include "generic.h"

int SomeClass::MainDefinedFunction() {
  int x = HIDE_FROM_CHECKER(1);
  int y = SatelliteFunction(x);
  return x + y;
}

int TestClassDefinitionSpreadAcrossCompileUnits() {
  int result = SomeClass::MainDefinedFunction();
  return result != 3 ? 1 : 0;
}

int main() {
  int exitCode = 0;
  exitCode += TestClassDefinitionSpreadAcrossCompileUnits();
  return exitCode;
}
