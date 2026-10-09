;;! target = "x86_64"
;;! test = "compile"
;;! filter = "wasm[1]::function"
;;! flags = "-C inlining=y -Wconcurrency-support=y"
;;! unsafe_intrinsics = "unsafe-intrinsics"

(component
    (import "unsafe-intrinsics"
         (instance $intrinsics
             (export "store-data-address" (func (result u64)))
             (export "u64-native-load" (func (param "pointer" u64) (result u64)))
             (export "u8-native-load" (func (param "pointer" u64) (result u8)))
             (export "u8-native-store" (func (param "pointer" u64) (param "value" u8)))
         )
    )

    (component $HostBuf
        (import "unsafe-intrinsics"
            (instance $intrinsics
                (export "store-data-address" (func (result u64)))
                (export "u64-native-load" (func (param "pointer" u64) (result u64)))
                (export "u8-native-load" (func (param "pointer" u64) (result u8)))
                (export "u8-native-store" (func (param "pointer" u64) (param "value" u8)))
            )
        )

        ;; The core Wasm module that implements the safe API.
        (core module $host-buf-impl
            (import "" "store-data-address" (func $store-data-address (result i64)))
            (import "" "u64-native-load" (func $u64-native-load (param i64) (result i64)))
            (import "" "u8-native-load" (func $u8-native-load (param i64) (result i32)))
            (import "" "u8-native-store" (func $u8-native-store (param i64 i32)))

            ;; Load the `ExposedBuf::buf_ptr` field
            (func $get-buf-ptr (result i64)
                (call $u64-native-load (i64.add (call $store-data-address) (i64.const 0)))
            )

            ;; Load the `ExposedBuf::buf_len` field
            (func $get-buf-len (result i64)
                (call $u64-native-load (i64.add (call $store-data-address) (i64.const 8)))
            )

            ;; Check that `$i` is within `ExposedBuf` buffer's bounds, raising a trap
            ;; otherwise.
            (func $bounds-check (param $i i64)
                (if (i64.lt_u (local.get $i) (call $get-buf-len))
                    (then (return))
                    (else (unreachable))
                )
            )

            ;; A safe function to get the `i`th byte from `ExposedBuf`'s buffer,
            ;; raising a trap on out-of-bounds accesses.
            (func (export "get") (param $i i64) (result i32)
                (call $bounds-check (local.get $i))
                (call $u8-native-load (i64.add (call $get-buf-ptr) (local.get $i)))
            )

            ;; A safe function to set the `i`th byte in `ExposedBuf`'s buffer,
            ;; raising a trap on out-of-bounds accesses.
            (func (export "set") (param $i i64) (param $value i32)
                (call $bounds-check (local.get $i))
                (call $u8-native-store (i64.add (call $get-buf-ptr) (local.get $i))
                                       (local.get $value))
            )

            ;; A safe function to get the length of the `ExposedBuf` buffer.
            (func (export "len") (result i64)
                (call $get-buf-len)
            )
        )

        ;; Lower the imported intrinsics from component functions to core functions.
        (core func $store-data-address' (canon lower (func $intrinsics "store-data-address")))
        (core func $u64-native-load' (canon lower (func $intrinsics "u64-native-load")))
        (core func $u8-native-load' (canon lower (func $intrinsics "u8-native-load")))
        (core func $u8-native-store' (canon lower (func $intrinsics "u8-native-store")))

        ;; Instantiate our safe API implementation, passing in the lowered unsafe
        ;; intrinsics as its imports.
        (core instance $instance
            (instantiate $host-buf-impl
                (with "" (instance
                    (export "store-data-address" (func $store-data-address'))
                    (export "u64-native-load" (func $u64-native-load'))
                    (export "u8-native-load" (func $u8-native-load'))
                    (export "u8-native-store" (func $u8-native-store'))
                ))
            )
        )

        ;; Lift the safe API's exports from core functions to component functions
        ;; and export them.
        (func (export "get") (param "i" u64) (result u8)
            (canon lift (core func $instance "get"))
        )
        (func (export "set") (param "i" u64) (param "value" u8)
            (canon lift (core func $instance "set"))
        )
        (func (export "len") (result u64)
            (canon lift (core func $instance "len"))
        )
    )
    (instance $host-buf (instantiate $HostBuf (with "unsafe-intrinsics" (instance $intrinsics))))

    (component $Guest
        ;; Import the safe API.
        (import "host-buf"
            (instance $host-buf
                (export "get" (func (param "i" u64) (result u8)))
                (export "set" (func (param "i" u64) (param "value" u8)))
                (export "len" (func (result u64)))
            )
        )

        ;; Define this component's core module implementation.
        (core module $main-impl
            (import "" "get" (func $get (param i64) (result i32)))
            (import "" "set" (func $set (param i64 i32)))
            (import "" "len" (func $len (result i64)))

            (func (export "main")
                (local $i i64)
                (local $n i64)

                (local.set $i (i64.const 0))
                (local.set $n (call $len))

                (loop $loop
                    ;; When we have iterated over every byte in the
                    ;; buffer, exit.
                    (if (i64.ge_u (local.get $i) (local.get $n))
                        (then (return)))

                    ;; Increment the `i`th byte in the buffer.
                    (call $set (local.get $i)
                               (i32.add (call $get (local.get $i))
                                        (i32.const 1)))

                    ;; Increment `i` and continue to the next iteration
                    ;; of the loop.
                    (local.set $i (i64.add (local.get $i) (i64.const 1)))
                    (br $loop)
                )
            )
        )

        ;; Lower the imported safe APIs from component functions to core functions.
        (core func $get' (canon lower (func $host-buf "get")))
        (core func $set' (canon lower (func $host-buf "set")))
        (core func $len' (canon lower (func $host-buf "len")))

        ;; Instantiate our module, providing the lowered safe APIs as imports.
        (core instance $instance
            (instantiate $main-impl
                (with "" (instance
                    (export "get" (func $get'))
                    (export "set" (func $set'))
                    (export "len" (func $len'))
                ))
            )
        )

        ;; Lift the implementation's `main` from a core function to a component function
        ;; and export it!
        (func (export "main")
            (canon lift (core func $instance "main"))
        )
    )
    (instance $guest (instantiate $Guest (with "host-buf" (instance $host-buf))))

    (export "main" (func $guest "main"))
)

;; wasm[1]::function[3]:
;;       pushq   %rbp
;;       movq    %rsp, %rbp
;;       movq    0x48(%rdi), %rcx
;;       movq    0xe8(%rcx), %rax
;;       movl    (%rax), %edx
;;       testl   %edx, %edx
;;       je      0x16d
;;  119: movq    0x48(%rcx), %rcx
;;       movq    8(%rcx), %rcx
;;       movq    0x68(%rcx), %rcx
;;       movq    8(%rcx), %rdx
;;       xorq    %rsi, %rsi
;;       cmpq    %rdx, %rsi
;;       jae     0x168
;;  135: movl    (%rax), %edi
;;       testl   %edi, %edi
;;       je      0x16f
;;  13f: movq    8(%rcx), %rdi
;;       cmpq    %rdi, %rsi
;;       jae     0x171
;;  14c: movq    (%rcx), %rdi
;;       movzbq  (%rdi, %rsi), %r8
;;       addb    $1, %r8b
;;       movb    %r8b, (%rdi, %rsi)
;;       addq    $1, %rsi
;;       jmp     0x12c
;;  168: movq    %rbp, %rsp
;;       popq    %rbp
;;       retq
;;  16d: ud2
;;  16f: ud2
;;  171: ud2
