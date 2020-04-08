// Compile with:
//   clang --target=wasm32 reverse-str.c -o reverse-str.wasm -g \
//     -O0 -nostdlib -fdebug-prefix-map=$PWD=.
#include <stdlib.h>

void reverse(char *s, size_t len)
{
    if (!len) return;
    size_t i = 0, j = len - 1;
    while (i < j) {
        char t = s[i];
        s[i++] = s[j];
        s[j--] = t;
    }
}

void _start()
{
    char hello[] = "Hello, world.";
    reverse(hello, 13);
}
