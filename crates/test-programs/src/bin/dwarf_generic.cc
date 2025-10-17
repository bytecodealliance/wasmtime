//! extra-files = ['dwarf_generic_satellite.cpp']

// clang-format off
// clang -o generic.wasm -target wasm32-unknown-wasip1 -g generic.cpp generic-satellite.cpp
// clang-format on
//
#include "dwarf_generic.h"

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

__asm("FunctionWithoutWasmDWARF:\n"
      ".global	FunctionWithoutWasmDWARF\n"
      ".functype	FunctionWithoutWasmDWARF (i32, i32) -> (i32)\n"
      "local.get	0\n"
      "local.get	1\n"
      "i32.div_u\n"
      "end_function\n");
extern "C" int FunctionWithoutWasmDWARF(int a, int b);

int TestFunctionWithoutWasmDWARF() {
  debug_break();
  int x = FunctionWithoutWasmDWARF(9, 10);
  return x == 0 ? 0 : 4;
}

int main() {
  int exitCode = 0;
  exitCode += TestClassDefinitionSpreadAcrossCompileUnits();
  exitCode += TestInheritance();
  exitCode += TestInstanceMethod();
  exitCode += TestFunctionWithoutWasmDWARF();
  return exitCode;
}
