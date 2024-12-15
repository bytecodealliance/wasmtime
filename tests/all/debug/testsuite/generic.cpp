// clang-format off
// clang generic.cpp generic-satellite.cpp -o generic.wasm -g -target wasm32-unknown-wasip1
// clang-format on
//
#include "generic.h"

int SomeClass::MainDefinedFunction() {
  int x = HIDE_FROM_CHECKER(1);
  debug_break();
  int y = SatelliteFunction(x);
  return x + y;
}

int TestClassDefinitionSpreadAcrossCompileUnits() {
  int result = SomeClass::MainDefinedFunction();
  return result != 3 ? 1 : 0;
}

struct BaseType {
  int BaseValue = 1;
};
struct DerivedType : BaseType {
  long long DerivedValue = 2;
};

int TestInheritance() {
  DerivedType inst;
  debug_break();
  return inst.BaseValue + inst.DerivedValue != 3 ? 1 : 0;
}

int main() {
  int exitCode = 0;
  exitCode += TestClassDefinitionSpreadAcrossCompileUnits();
  exitCode += TestInheritance();
  return exitCode;
}
