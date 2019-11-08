#include <emscripten.h>
#include "wasmtime_ssp.h"
#include "../src/posix.h"

static __thread struct fd_table curfds_pointee;

int main(int argc, char *argv[]) {
    return 0;
}

void handleFiles(void) {
    struct fd_table *curfds = &curfds_pointee;

    fd_table_init(curfds);

    // Prepopulate curfds with stdin, stdout, and stderr file descriptors.
    if (!fd_table_insert_existing(curfds, 0, 0))
        __builtin_trap();
    if (!fd_table_insert_existing(curfds, 1, 1))
        __builtin_trap();
    if (!fd_table_insert_existing(curfds, 2, 2))
        __builtin_trap();

    EM_ASM(" \
        const imports = {\
            wasi_unstable: WASIPolyfill, \
            wasi_unstable_preview0: WASIPolyfill \
        }; \
        let file = document.getElementById('input').files[0]; \
        let file_with_mime_type = file.slice(0, file.size, 'application/wasm'); \
        let response = new Response(file_with_mime_type); \
        wasi_instantiateStreaming(response, imports) \
        .then(obj => { \
            setInstance(obj.instance); \
            try { \
                obj.instance.exports._start(); \
            } catch (e) { \
                if (e instanceof WASIExit) { \
                    handleWASIExit(e); \
                } else { \
                } \
            } \
        }) \
        .catch(error => { \
            console.log('error! ' + error); \
        }); \
    ");
}
