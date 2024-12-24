#include <inttypes.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <wasm.h>
#include <wasmtime.h>
#include <wasmtime/component.h>

const char *multiply_data = "hello multiply";
const char *apply_data = "hello apply";
const char *context_data = "context data";

static void exit_with_error(const char *message, wasmtime_error_t *error,
                            wasm_trap_t *trap);

wasm_trap_t *mult(void *env, wasmtime_context_t *context,
                  const wasmtime_component_val_t *args, size_t nargs,
                  wasmtime_component_val_t *results, size_t nresults) {
  assert(env == (void *)multiply_data);
  const char *exec_env = (const char *)wasmtime_context_get_data(context);
  assert(exec_env == context_data);
  assert(nargs == 2);
  assert(nresults == 1);
  assert(args[0].kind == WASMTIME_COMPONENT_KIND_F32);
  assert(args[1].kind == WASMTIME_COMPONENT_KIND_F32);
  float res = args[0].payload.f32 * args[1].payload.f32;
  results[0].kind = WASMTIME_COMPONENT_KIND_F32;
  results[0].payload.f32 = res;
  return NULL;
}

wasm_trap_t *apply(void *env, wasmtime_context_t *context,
                   const wasmtime_component_val_t *args, size_t nargs,
                   wasmtime_component_val_t *results, size_t nresults) {
  assert(env == (void *)apply_data);
  const char *exec_env = (const char *)wasmtime_context_get_data(context);
  assert(exec_env == context_data);
  assert(nargs == 3);
  assert(nresults == 1);
  assert(args[0].kind == WASMTIME_COMPONENT_KIND_F32);
  assert(args[1].kind == WASMTIME_COMPONENT_KIND_F32);
  assert(args[2].kind == WASMTIME_COMPONENT_KIND_ENUM);
  float res = args[2].payload.enumeration.discriminant == 0
                  ? args[0].payload.f32 + args[1].payload.f32
                  : args[0].payload.f32 * args[1].payload.f32;
  results[0].kind = WASMTIME_COMPONENT_KIND_F32;
  results[0].payload.f32 = res;
  return NULL;
}

int main() {
  wasm_engine_t *engine = wasm_engine_new();

  // Create a component linker with host functions defined
  wasmtime_component_linker_t *linker = wasmtime_component_linker_new(engine);
  // `multiply` only uses types without additional owned data, that can live on
  // the stack
  wasmtime_component_type_t mult_param_types[2];
  mult_param_types[0].kind = WASMTIME_COMPONENT_KIND_F32;
  mult_param_types[1].kind = WASMTIME_COMPONENT_KIND_F32;
  wasmtime_component_type_t f32_result_type;
  f32_result_type.kind = WASMTIME_COMPONENT_KIND_F32;

  wasmtime_error_t *error = wasmtime_component_linker_define_func(
      linker, "host", 4, "multiply", 8, mult_param_types, 2, &f32_result_type,
      1, mult, (void *)multiply_data, NULL);
  if (error)
    exit_with_error("failed to define function multiply", error, NULL);

  // `apply` uses an enum, which needs additional data, use a
  // wasmtime_component_type_vec_t
  wasmtime_component_type_vec_t apply_param_types;
  wasmtime_component_type_vec_new_uninitialized(&apply_param_types, 3);
  apply_param_types.data[0].kind = WASMTIME_COMPONENT_KIND_F32;
  apply_param_types.data[1].kind = WASMTIME_COMPONENT_KIND_F32;
  apply_param_types.data[2].kind = WASMTIME_COMPONENT_KIND_ENUM;
  wasmtime_component_string_vec_new_uninitialized(
      &apply_param_types.data[2].payload.enumeration, 2);
  wasm_name_new_from_string(
      &apply_param_types.data[2].payload.enumeration.data[0], "add");
  wasm_name_new_from_string(
      &apply_param_types.data[2].payload.enumeration.data[1], "multiply");
  error = wasmtime_component_linker_define_func(
      linker, "host", 4, "apply", 5, apply_param_types.data,
      apply_param_types.size, &f32_result_type, 1, apply, (void *)apply_data,
      NULL);
  if (error)
    exit_with_error("failed to define function apply", error, NULL);
  // deleting the vector also drops the full types hierarchy
  wasmtime_component_type_vec_delete(&apply_param_types);

  error = wasmtime_component_linker_build(linker);
  if (error)
    exit_with_error("failed to build linker", error, NULL);

  // Load binary.
  printf("Loading binary...\n");
  // Note that the binary should be a component (not a plain module), typically
  // built by running `cargo component build -p example-component-wasm --target
  // wasm32-unknown-unknown`, here created with a secondary usage of rust test
  FILE *file =
      fopen("target/wasm32-unknown-unknown/debug/guest-component.wasm", "rb");
  if (!file) {
    printf("> Error opening component!\n");
    return 1;
  }
  fseek(file, 0L, SEEK_END);
  size_t file_size = ftell(file);
  fseek(file, 0L, SEEK_SET);
  wasm_byte_vec_t binary;
  wasm_byte_vec_new_uninitialized(&binary, file_size);
  if (fread(binary.data, file_size, 1, file) != 1) {
    printf("> Error reading component!\n");
    return 1;
  }
  fclose(file);

  // Compile.
  printf("Compiling component...\n");
  wasmtime_component_t *component;
  error = wasmtime_component_from_binary(engine, (uint8_t *)binary.data,
                                         binary.size, &component);
  if (error)
    exit_with_error("failed to build component", error, NULL);
  wasm_byte_vec_delete(&binary);

  wasmtime_store_t *store =
      wasmtime_store_new(engine, (void *)context_data, NULL);
  wasmtime_context_t *context = wasmtime_store_context(store);

  // Instantiate.
  printf("Instantiating component...\n");
  wasmtime_component_instance_t *instance;
  error = wasmtime_component_linker_instantiate(linker, context, component,
                                                &instance);
  if (error)
    exit_with_error("failed to instantiate component", error, NULL);

  // Lookup functions.
  wasmtime_component_func_t *convert1;
  const char *func_name1 = "convert-celsius-to-fahrenheit";
  bool ok = wasmtime_component_instance_get_func(instance, context, func_name1,
                                                 strlen(func_name1), &convert1);
  if (!ok)
    exit_with_error("function convert-celsius-to-fahrenheit not found", NULL,
                    NULL);
  wasmtime_component_func_t *convert2;
  const char *func_name2 = "convert";
  ok = wasmtime_component_instance_get_func(instance, context, func_name2,
                                            strlen(func_name2), &convert2);
  if (!ok)
    exit_with_error("function convert not found", NULL, NULL);

  // Call.
  printf("Calling convert-celsius-to-fahrenheit...\n");
  wasmtime_component_val_t param_val, result_val;
  param_val.kind = WASMTIME_COMPONENT_KIND_F32;
  param_val.payload.f32 = 23.4f;
  // will be written, but must be "droppable", i.e. without owned data
  result_val.kind = WASMTIME_COMPONENT_KIND_BOOL;
  wasm_trap_t *trap = NULL;
  error = wasmtime_component_func_call(convert1, context, &param_val, 1,
                                       &result_val, 1, &trap);
  if (error != NULL || trap != NULL)
    exit_with_error("failed to call function convert-celsius-to-fahrenheit",
                    error, trap);

  assert(result_val.kind == WASMTIME_COMPONENT_KIND_F32);
  printf("23.4°C = %f°F\n", result_val.payload.f32);

  printf("Calling convert...\n");
  wasmtime_component_val_t *t = wasmtime_component_val_new();
  t->kind = WASMTIME_COMPONENT_KIND_VARIANT;
  t->payload.variant.discriminant = 1;
  t->payload.variant.val = wasmtime_component_val_new();
  t->payload.variant.val->kind = WASMTIME_COMPONENT_KIND_F32;
  t->payload.variant.val->payload.f32 = 66.2f;
  wasmtime_component_val_t *result = wasmtime_component_val_new();

  error =
      wasmtime_component_func_call(convert2, context, t, 1, result, 1, &trap);
  if (error != NULL || trap != NULL)
    exit_with_error("failed to call function", error, trap);
  wasmtime_component_val_delete(t);

  assert(result->kind == WASMTIME_COMPONENT_KIND_VARIANT);
  assert(result->payload.variant.discriminant == 0);
  assert(result->payload.variant.val != NULL);
  assert(result->payload.variant.val->kind == WASMTIME_COMPONENT_KIND_F32);
  printf("66.2°F = %f°C\n", result->payload.variant.val->payload.f32);
  wasmtime_component_val_delete(result);

  wasmtime_store_delete(store);

  // Shut down.
  printf("Shutting down...\n");
  wasmtime_component_delete(component);
  wasmtime_component_linker_delete(linker);
  wasm_engine_delete(engine);

  // All done.
  printf("Done.\n");
  return 0;
}

static void exit_with_error(const char *message, wasmtime_error_t *error,
                            wasm_trap_t *trap) {
  fprintf(stderr, "error: %s\n", message);
  wasm_byte_vec_t error_message;
  if (error != NULL) {
    wasmtime_error_message(error, &error_message);
  } else {
    wasm_trap_message(trap, &error_message);
  }
  fprintf(stderr, "%.*s\n", (int)error_message.size, error_message.data);
  wasm_byte_vec_delete(&error_message);
  exit(1);
}
