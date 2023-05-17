#include <windows.h>

LPVOID wasmtime_fiber_get_current() {
   return GetCurrentFiber();
}
