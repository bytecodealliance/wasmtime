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

  int InstanceMethod() {
    debug_break();
    return BaseValue + DerivedValue;
  }

  int ConstInstanceMethod() const {
    debug_break();
    return BaseValue + DerivedValue;
  }
};

int TestInheritance() {
  DerivedType inst;
  debug_break();
  return inst.BaseValue + inst.DerivedValue != 3 ? 1 : 0;
}

int TestInstanceMethod() {
  debug_break();

  DerivedType inst;
  inst.BaseValue = 2;
  inst.DerivedValue = 4;
  if (inst.InstanceMethod() != 6)
    return 1;

  inst.BaseValue++;
  volatile DerivedType volatileInst = inst;
  if (inst.InstanceMethod() != 7)
    return 2;

  inst.DerivedValue++;
  const DerivedType constInst = inst;
  if (inst.ConstInstanceMethod() != 8)
    return 3;

  return 0;
}

int main() {
  int exitCode = 0;
  exitCode += TestClassDefinitionSpreadAcrossCompileUnits();
  exitCode += TestInheritance();
  exitCode += TestInstanceMethod();
  return exitCode;
}
