#include "plugin.h"

void exports_plugin_get_plugin_name(plugin_string_t *ret) {
  plugin_string_set(ret, "add");
}

int32_t exports_plugin_evaluate(int32_t x, int32_t y) { return x + y; }
