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

    ;; A host buffer. Its rep is an index into the store data's
    ;; `ExposedVec<ExposedBuf>`.
    (type $host-buf (resource (rep i32)))
    (core func $host-buf-new (canon resource.new $host-buf))

    ;; The core Wasm module that implements the safe API.
    ;;
    ;; This component defines the `host-buf` resource, so lifted `borrow`s
    ;; of it arrive as reps rather than handles.
    (core module $host-buf-impl
        (import "" "store-data-address" (func $store-data-address (result i64)))
        (import "" "u64-native-load" (func $u64-native-load (param i64) (result i64)))
        (import "" "u8-native-load" (func $u8-native-load (param i64) (result i32)))
        (import "" "u8-native-store" (func $u8-native-store (param i64 i32)))
        (import "" "host-buf-new" (func $host-buf-new (param i32) (result i32)))

        ;; Load the `ExposedVec::ptr` field
        (func $get-vec-ptr (result i64)
            (call $u64-native-load (i64.add (call $store-data-address) (i64.const 0)))
        )

        ;; Load the `ExposedVec::len` field
        (func $get-vec-len (result i64)
            (call $u64-native-load (i64.add (call $store-data-address) (i64.const 8)))
        )

        ;; Check that `$index` is within the `ExposedVec`'s bounds, raising
        ;; a trap otherwise.
        (func $vec-bounds-check (param $index i32)
            (if (i64.lt_u (i64.extend_i32_u (local.get $index)) (call $get-vec-len))
                (then (return))
                (else (unreachable))
            )
        )

        ;; Get the address of the `$buf`th `ExposedBuf` in the
        ;; `ExposedVec`, raising a trap on out-of-bounds accesses.
        (func $get-buf-addr (param $buf i32) (result i64)
            (call $vec-bounds-check (local.get $buf))
            (i64.add (call $get-vec-ptr)
                     ;; `size_of::<ExposedBuf>() == 16`
                     (i64.mul (i64.extend_i32_u (local.get $buf)) (i64.const 16)))
        )

        ;; Load the `$buf`th `ExposedBuf::buf_ptr` field
        (func $get-buf-ptr (param $buf i32) (result i64)
            (call $u64-native-load (i64.add (call $get-buf-addr (local.get $buf))
                                            (i64.const 0)))
        )

        ;; Load the `$buf`th `ExposedBuf::buf_len` field
        (func $get-buf-len (param $buf i32) (result i64)
            (call $u64-native-load (i64.add (call $get-buf-addr (local.get $buf))
                                            (i64.const 8)))
        )

        ;; Check that `$i` is within the `$buf`th `ExposedBuf` buffer's
        ;; bounds, raising a trap otherwise.
        (func $buf-bounds-check (param $buf i32) (param $i i64)
            (if (i64.lt_u (local.get $i) (call $get-buf-len (local.get $buf)))
                (then (return))
                (else (unreachable))
            )
        )

        ;; A safe function to create a `host-buf` resource for the
        ;; `$index`th `ExposedBuf`, raising a trap on out-of-bounds
        ;; indices.
        (func (export "new") (param $index i32) (result i32)
            (call $vec-bounds-check (local.get $index))
            (call $host-buf-new (local.get $index))
        )

        ;; A safe function to get the `i`th byte from an `ExposedBuf`'s
        ;; buffer, raising a trap on out-of-bounds accesses.
        (func (export "get") (param $buf i32) (param $i i64) (result i32)
            (call $buf-bounds-check (local.get $buf) (local.get $i))
            (call $u8-native-load (i64.add (call $get-buf-ptr (local.get $buf))
                                           (local.get $i)))
        )

        ;; A safe function to set the `i`th byte in an `ExposedBuf`'s
        ;; buffer, raising a trap on out-of-bounds accesses.
        (func (export "set") (param $buf i32) (param $i i64) (param $value i32)
            (call $buf-bounds-check (local.get $buf) (local.get $i))
            (call $u8-native-store (i64.add (call $get-buf-ptr (local.get $buf))
                                            (local.get $i))
                                   (local.get $value))
        )

        ;; A safe function to get the length of an `ExposedBuf`'s buffer.
        (func (export "len") (param $buf i32) (result i64)
            (call $get-buf-len (local.get $buf))
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
                (export "host-buf-new" (func $host-buf-new))
            ))
        )
    )

    ;; Export the resource type and lift the safe API's exports from core
    ;; functions to component functions and export them.
    (export $host-buf' "host-buf" (type $host-buf))
    (func (export "new") (param "index" u32) (result (own $host-buf'))
        (canon lift (core func $instance "new"))
    )
    (func (export "get") (param "buf" (borrow $host-buf')) (param "i" u64) (result u8)
        (canon lift (core func $instance "get"))
    )
    (func (export "set") (param "buf" (borrow $host-buf')) (param "i" u64) (param "value" u8)
        (canon lift (core func $instance "set"))
    )
    (func (export "len") (param "buf" (borrow $host-buf')) (result u64)
        (canon lift (core func $instance "len"))
    )
)
    (instance $host-buf (instantiate $HostBuf (with "unsafe-intrinsics" (instance $intrinsics))))

    (component $Guest
        ;; Import the safe API.
        (import "host-buf"
            (instance $host-buf
                (export "host-buf" (type $host-buf (sub resource)))
                (export "get" (func (param "buf" (borrow $host-buf)) (param "i" u64) (result u8)))
                (export "set" (func (param "buf" (borrow $host-buf)) (param "i" u64) (param "value" u8)))
                (export "len" (func (param "buf" (borrow $host-buf)) (result u64)))
            )
        )
        (alias export $host-buf "host-buf" (type $host-buf))

        ;; Define this component's core module implementation.
        (core module $main-impl
            (import "" "drop" (func $drop (param i32)))
            (import "" "get" (func $get (param i32 i64) (result i32)))
            (import "" "set" (func $set (param i32 i64 i32)))
            (import "" "len" (func $len (param i32) (result i64)))

            (func (export "main") (param $buf i32)
                (local $i i64)
                (local $n i64)

                (local.set $i (i64.const 0))
                (local.set $n (call $len (local.get $buf)))

                (loop $loop
                    ;; When we have iterated over every byte in the
                    ;; buffer, exit.
                    (if (i64.ge_u (local.get $i) (local.get $n))
                        (then
                            ;; Borrows must be dropped before returning.
                            (call $drop (local.get $buf))
                            (return)))

                    ;; Increment the `i`th byte in the buffer.
                    (call $set (local.get $buf)
                               (local.get $i)
                               (i32.add (call $get (local.get $buf) (local.get $i))
                                        (i32.const 1)))

                    ;; Increment `i` and continue to the next iteration
                    ;; of the loop.
                    (local.set $i (i64.add (local.get $i) (i64.const 1)))
                    (br $loop)
                )
            )
        )

        ;; Lower the imported safe APIs from component functions to core functions.
        (core func $drop (canon resource.drop $host-buf))
        (core func $get' (canon lower (func $host-buf "get")))
        (core func $set' (canon lower (func $host-buf "set")))
        (core func $len' (canon lower (func $host-buf "len")))

        ;; Instantiate our module, providing the lowered safe APIs as imports.
        (core instance $instance
            (instantiate $main-impl
                (with "" (instance
                    (export "drop" (func $drop))
                    (export "get" (func $get'))
                    (export "set" (func $set'))
                    (export "len" (func $len'))
                ))
            )
        )

        ;; Export the host buffer resource type, so that the host can name
        ;; it, along with a lifted version of the implementation's `main` core
        ;; function.
        (export $host-buf' "host-buf" (type $host-buf))
        (func (export "main") (param "buf" (borrow $host-buf'))
            (canon lift (core func $instance "main"))
        )
    )
    (instance $guest (instantiate $Guest (with "host-buf" (instance $host-buf))))

    (export $host-buf "host-buf" (type $guest "host-buf"))
    (export "main" (func $guest "main") (func (param "buf" (borrow $host-buf))))
)

;; wasm[1]::function[4]:
;;       pushq   %rbp
;;       movq    %rsp, %rbp
;;       movq    8(%rdi), %r10
;;       movq    0x10(%r10), %r10
;;       addq    $0xf0, %r10
;;       cmpq    %rsp, %r10
;;       ja      0x8c7
;;  2fc: subq    $0xe0, %rsp
;;       movq    %rbx, 0xb0(%rsp)
;;       movq    %r12, 0xb8(%rsp)
;;       movq    %r13, 0xc0(%rsp)
;;       movq    %r14, 0xc8(%rsp)
;;       movq    %r15, 0xd0(%rsp)
;;       movq    0x68(%rdi), %rbx
;;       movq    %rdi, 0x60(%rsp)
;;       movq    0x148(%rbx), %rax
;;       movl    (%rax), %ecx
;;       movq    %rax, 0xa8(%rsp)
;;       testl   %ecx, %ecx
;;       je      0x8c9
;;  34d: movq    8(%rbx), %r13
;;       movq    0x80(%r13), %rax
;;       leaq    (%rsp), %r12
;;       movq    %rax, (%rsp)
;;       movl    $0, 8(%rsp)
;;       movl    $1, 0xc(%rsp)
;;       movl    0x78(%r13), %eax
;;       movl    %eax, 0x10(%rsp)
;;       movl    $0, 0x78(%r13)
;;       movl    0x7c(%r13), %eax
;;       movl    %eax, 0x14(%rsp)
;;       movl    $0, 0x7c(%r13)
;;       movq    0x90(%r13), %rcx
;;       movq    %rcx, 0x18(%rsp)
;;       movq    %r12, 0x80(%r13)
;;       cmpq    $8, %rcx
;;       jae     0x818
;;  3ad: movq    0x68(%rbx), %rsi
;;       movq    0x78(%rsi), %r8
;;       leal    -1(%rdx), %edi
;;       movq    %rdx, 0x68(%rsp)
;;       cmpq    %r8, %rdi
;;       jae     0x81d
;;  3c6: movq    0x70(%rsi), %rsi
;;       imulq   $0x18, %rdi, %rdi
;;       movl    $0xffffffff, %r8d
;;       addl    (%rsi, %rdi), %r8d
;;       cmpl    $1, %r8d
;;       ja      0x81d
;;  3e2: movl    4(%rsi, %rdi), %r8d
;;       cmpl    $2, %r8d
;;       jne     0x81d
;;  3f1: movl    8(%rsi, %rdi), %eax
;;       movq    0x88(%r13), %r11
;;       movl    $2, (%r11, %rcx, 8)
;;       movq    0x68(%rsp), %rdx
;;       movl    %edx, 4(%r11, %rcx, 8)
;;       addq    $1, %rcx
;;       movq    %rcx, 0x90(%r13)
;;       movq    0x48(%rbx), %rcx
;;       movq    8(%rcx), %rcx
;;       movq    0x60(%rcx), %rcx
;;       movq    8(%rcx), %rdx
;;       movl    %eax, %eax
;;       cmpq    %rdx, %rax
;;       jae     0x8cb
;;  437: movq    (%rcx), %rdx
;;       movq    %rcx, 0xa0(%rsp)
;;       shlq    $4, %rax
;;       movq    8(%rdx, %rax), %rax
;;       movq    %rax, 0x98(%rsp)
;;       movq    0x80(%r13), %rax
;;       cmpq    %r12, %rax
;;       jne     0x838
;;  463: movq    (%rsp), %rax
;;       movq    %rax, 0x80(%r13)
;;       movq    0x18(%rsp), %rax
;;       movq    %rax, 0x90(%r13)
;;       movl    0x10(%rsp), %eax
;;       movl    %eax, 0x78(%r13)
;;       movl    0x14(%rsp), %eax
;;       movl    %eax, 0x7c(%r13)
;;       xorq    %r12, %r12
;;       leaq    0x20(%rsp), %r14
;;       movq    %r14, 0x90(%rsp)
;;       movq    0x68(%rbx), %rax
;;       movq    %rax, %rsi
;;       movq    0x68(%rsp), %rax
;;       leal    -1(%rax), %r15d
;;       imulq   $0x18, %r15, %r14
;;       movq    %r14, 0x80(%rsp)
;;       movq    %r15, 0x88(%rsp)
;;       movq    0x88(%r13), %r15
;;       movq    %r15, 0x78(%rsp)
;;       leaq    0x40(%rsp), %r15
;;       movq    %r15, 0x70(%rsp)
;;       movq    0x98(%rsp), %rax
;;       cmpq    %rax, %r12
;;       movq    %rax, 0x98(%rsp)
;;       jae     0x7cd
;;  4ed: movq    0xa8(%rsp), %rax
;;       movl    (%rax), %ecx
;;       testl   %ecx, %ecx
;;       je      0x8cd
;;  4ff: movq    0x80(%r13), %rax
;;       movq    %rax, 0x20(%rsp)
;;       movl    $0, 0x28(%rsp)
;;       movl    $1, 0x2c(%rsp)
;;       movl    0x78(%r13), %eax
;;       movl    %eax, 0x30(%rsp)
;;       movl    $0, 0x78(%r13)
;;       movl    0x7c(%r13), %eax
;;       movl    %eax, 0x34(%rsp)
;;       movl    $0, 0x7c(%r13)
;;       movq    0x90(%r13), %rcx
;;       movq    %rcx, 0x38(%rsp)
;;       movq    0x90(%rsp), %rdx
;;       movq    %rdx, 0x80(%r13)
;;       cmpq    $8, %rcx
;;       jae     0x84d
;;  560: movq    %rsi, %r14
;;       movq    0x78(%r14), %rsi
;;       movq    0x88(%rsp), %rax
;;       cmpq    %rsi, %rax
;;       jae     0x85d
;;  578: movq    0x70(%r14), %r9
;;       movl    $0xffffffff, %r8d
;;       movq    0x80(%rsp), %r15
;;       addl    (%r9, %r15), %r8d
;;       cmpl    $1, %r8d
;;       ja      0x865
;;  598: movl    4(%r9, %r15), %eax
;;       cmpl    $2, %eax
;;       jne     0x865
;;  5a6: movl    8(%r9, %r15), %eax
;;       movq    0x78(%rsp), %rdx
;;       movq    %r15, 0x80(%rsp)
;;       movl    $2, (%rdx, %rcx, 8)
;;       movq    0x68(%rsp), %rsi
;;       movl    %esi, 4(%rdx, %rcx, 8)
;;       addq    $1, %rcx
;;       movq    %rcx, 0x90(%r13)
;;       movq    0xa0(%rsp), %rsi
;;       movq    8(%rsi), %rdx
;;       movl    %eax, %ecx
;;       cmpq    %rdx, %rcx
;;       jae     0x8cf
;;  5ed: movq    (%rsi), %rdx
;;       movq    %rsi, 0xa0(%rsp)
;;       shlq    $4, %rcx
;;       movq    8(%rdx, %rcx), %rax
;;       cmpq    %rax, %r12
;;       jae     0x8d1
;;  60a: movq    (%rdx, %rcx), %rax
;;       movzbq  (%rax, %r12), %r15
;;       movq    0x80(%r13), %rax
;;       movq    0x90(%rsp), %rdx
;;       cmpq    %rdx, %rax
;;       jne     0x886
;;  62b: movq    0x20(%rsp), %rcx
;;       movq    %rcx, 0x80(%r13)
;;       movq    0x38(%rsp), %rdx
;;       movq    %rdx, 0x90(%r13)
;;       movl    0x30(%rsp), %esi
;;       movl    %esi, 0x78(%r13)
;;       movl    0x34(%rsp), %esi
;;       movl    %esi, 0x7c(%r13)
;;       movq    0xa8(%rsp), %rax
;;       movl    (%rax), %eax
;;       testl   %eax, %eax
;;       je      0x8d3
;;  665: movq    0x80(%r13), %rax
;;       movq    %rax, 0x40(%rsp)
;;       movl    $0, 0x48(%rsp)
;;       movl    $1, 0x4c(%rsp)
;;       movl    0x78(%r13), %eax
;;       movl    %eax, 0x50(%rsp)
;;       movl    $0, 0x78(%r13)
;;       movl    0x7c(%r13), %eax
;;       movl    %eax, 0x54(%rsp)
;;       movl    $0, 0x7c(%r13)
;;       movq    0x90(%r13), %rcx
;;       movq    %rcx, 0x58(%rsp)
;;       movq    0x70(%rsp), %rdx
;;       movq    %rdx, 0x80(%r13)
;;       cmpq    $8, %rcx
;;       jae     0x89a
;;  6c3: movq    0x78(%r14), %rax
;;       movq    0x88(%rsp), %r11
;;       cmpq    %rax, %r11
;;       jae     0x89a
;;  6d8: movq    0x70(%r14), %rax
;;       movl    $0xffffffff, %edx
;;       movq    0x80(%rsp), %rsi
;;       addl    (%rax, %rsi), %edx
;;       cmpl    $1, %edx
;;       ja      0x89a
;;  6f5: movq    0x80(%rsp), %rdx
;;       movl    4(%rax, %rdx), %edx
;;       cmpl    $2, %edx
;;       jne     0x89a
;;  70a: movq    0x80(%rsp), %rdx
;;       movl    8(%rax, %rdx), %eax
;;       movq    0x78(%rsp), %rdx
;;       movl    $2, (%rdx, %rcx, 8)
;;       movq    0x68(%rsp), %rsi
;;       movl    %esi, 4(%rdx, %rcx, 8)
;;       addq    $1, %rcx
;;       movq    %rcx, 0x90(%r13)
;;       movq    0xa0(%rsp), %rcx
;;       movq    8(%rcx), %rdx
;;       movl    %eax, %ecx
;;       cmpq    %rdx, %rcx
;;       jae     0x8d5
;;  750: movq    0xa0(%rsp), %rdx
;;       movq    (%rdx), %rdx
;;       shlq    $4, %rcx
;;       movq    8(%rdx, %rcx), %rax
;;       cmpq    %rax, %r12
;;       jae     0x8d7
;;  76d: movq    (%rdx, %rcx), %rax
;;       addb    $1, %r15b
;;       movb    %r15b, (%rax, %r12)
;;       movq    0x80(%r13), %rax
;;       movq    0x70(%rsp), %rdx
;;       cmpq    %rdx, %rax
;;       jne     0x8b3
;;  78e: movq    0x40(%rsp), %rax
;;       movq    %rax, 0x80(%r13)
;;       movq    0x58(%rsp), %rax
;;       movq    %rax, 0x90(%r13)
;;       movl    0x50(%rsp), %eax
;;       movl    %eax, 0x78(%r13)
;;       movl    0x54(%rsp), %eax
;;       movl    %eax, 0x7c(%r13)
;;       addq    $1, %r12
;;       movq    0x98(%rsp), %rax
;;       movq    %r14, %rsi
;;       jmp     0x4dc
;;  7cd: movq    0x60(%rsp), %rcx
;;       movq    0x38(%rcx), %rax
;;       movq    0x48(%rcx), %rdi
;;       movq    0x68(%rsp), %rdx
;;       movq    %rcx, %rsi
;;       callq   *%rax
;;       movq    0xb0(%rsp), %rbx
;;       movq    0xb8(%rsp), %r12
;;       movq    0xc0(%rsp), %r13
;;       movq    0xc8(%rsp), %r14
;;       movq    0xd0(%rsp), %r15
;;       addq    $0xe0, %rsp
;;       movq    %rbp, %rsp
;;       popq    %rbp
;;       retq
;;  818: movq    %rdx, 0x68(%rsp)
;;  81d: movq    0x98(%rbx), %r10
;;  824: movq    0x68(%rbx), %rdi
;;  828: movq    0x68(%rsp), %rdx
;;  82d: movq    %rbx, %rsi
;;  830: callq   *%r10
;;  833: jmp     0x41c
;;  838: movq    0xb8(%rbx), %rax
;;  83f: movq    0x68(%rbx), %rdi
;;  843: movq    %rbx, %rsi
;;  846: callq   *%rax
;;  848: jmp     0x48a
;;  84d: movq    %rsi, %r14
;;  850: movq    0x80(%rsp), %r15
;;  858: jmp     0x865
;;  85d: movq    0x80(%rsp), %r15
;;  865: movq    0x98(%rbx), %rax
;;  86c: movq    0x68(%rsp), %rdx
;;  871: movq    %rbx, %rsi
;;  874: movq    %r14, %rdi
;;  877: callq   *%rax
;;  879: movq    %r15, 0x80(%rsp)
;;  881: jmp     0x5d6
;;  886: movq    0xb8(%rbx), %rax
;;  88d: movq    %rbx, %rsi
;;  890: movq    %r14, %rdi
;;  893: callq   *%rax
;;  895: jmp     0x653
;;  89a: movq    0x98(%rbx), %rax
;;  8a1: movq    0x68(%rsp), %rdx
;;  8a6: movq    %rbx, %rsi
;;  8a9: movq    %r14, %rdi
;;  8ac: callq   *%rax
;;  8ae: jmp     0x739
;;  8b3: movq    0xb8(%rbx), %rax
;;  8ba: movq    %rbx, %rsi
;;  8bd: movq    %r14, %rdi
;;  8c0: callq   *%rax
;;  8c2: jmp     0x7b6
;;  8c7: ud2
;;  8c9: ud2
;;  8cb: ud2
;;  8cd: ud2
;;  8cf: ud2
;;  8d1: ud2
;;  8d3: ud2
;;  8d5: ud2
;;  8d7: ud2
