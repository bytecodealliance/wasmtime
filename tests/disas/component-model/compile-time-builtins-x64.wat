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
;;       movq    0x18(%r10), %r10
;;       addq    $0x100, %r10
;;       cmpq    %rsp, %r10
;;       ja      0xaf3
;;  2fc: subq    $0xf0, %rsp
;;       movq    %rbx, 0xc0(%rsp)
;;       movq    %r12, 0xc8(%rsp)
;;       movq    %r13, 0xd0(%rsp)
;;       movq    %r14, 0xd8(%rsp)
;;       movq    %r15, 0xe0(%rsp)
;;       movq    0x68(%rdi), %rbx
;;       movq    %rdi, 0x60(%rsp)
;;       movq    0x148(%rbx), %r8
;;       movl    (%r8), %eax
;;       movq    %r8, 0xb0(%rsp)
;;       testl   %eax, %eax
;;       je      0xaf5
;;  34e: movq    8(%rbx), %r13
;;       movq    0x88(%r13), %rax
;;       leaq    (%rsp), %r12
;;       movq    %rax, (%rsp)
;;       movl    $0, 8(%rsp)
;;       movl    $1, 0xc(%rsp)
;;       movl    0x80(%r13), %eax
;;       movl    %eax, 0x10(%rsp)
;;       movl    $0, 0x80(%r13)
;;       movl    0x84(%r13), %eax
;;       movl    %eax, 0x14(%rsp)
;;       movl    $0, 0x84(%r13)
;;       movq    0x98(%r13), %r8
;;       movq    %r8, 0x18(%rsp)
;;       movq    %r12, 0x88(%r13)
;;       cmpq    $8, %r8
;;       jae     0x7ae
;;  3ba: movq    0x68(%rbx), %rcx
;;       movq    0x78(%rcx), %rsi
;;       leal    -1(%rdx), %edi
;;       cmpq    %rsi, %rdi
;;       jae     0x7b8
;;  3ce: movq    0x70(%rcx), %rsi
;;       imulq   $0x18, %rdi, %rdi
;;       movl    $0xffffffff, %r9d
;;       addl    (%rsi, %rdi), %r9d
;;       cmpl    $1, %r9d
;;       ja      0x7c2
;;  3ea: movl    4(%rsi, %rdi), %r9d
;;       cmpl    $2, %r9d
;;       jne     0x7cc
;;  3f9: movl    8(%rsi, %rdi), %eax
;;       movq    %rax, %rdi
;;       movq    0x90(%r13), %r11
;;       movl    $2, (%r11, %r8, 8)
;;       movl    %edx, 4(%r11, %r8, 8)
;;       leaq    1(%r8), %rax
;;       movq    %rax, 0x98(%r13)
;;       movq    0x48(%rbx), %rax
;;       movq    8(%rax), %rax
;;       movq    0x68(%rax), %rsi
;;       movq    8(%rsi), %rcx
;;       movq    %rdi, %rax
;;       movl    %eax, %eax
;;       cmpq    %rcx, %rax
;;       jae     0xaf7
;;  43d: movq    (%rsi), %rcx
;;       shlq    $4, %rax
;;       movq    8(%rcx, %rax), %rcx
;;       movq    %rcx, 0xa0(%rsp)
;;       movq    0x88(%r13), %rax
;;       cmpq    %r12, %rax
;;       jne     0x7f4
;;  461: movq    (%rsp), %rax
;;       movq    %rax, 0x88(%r13)
;;       movq    0x18(%rsp), %rax
;;       movq    %rax, 0x98(%r13)
;;       movl    0x10(%rsp), %eax
;;       movl    %eax, 0x80(%r13)
;;       movl    0x14(%rsp), %eax
;;       movl    %eax, 0x84(%r13)
;;       xorq    %r15, %r15
;;       leaq    0x20(%rsp), %r11
;;       movq    %r11, 0x98(%rsp)
;;       movq    0x68(%rbx), %rdi
;;       leal    -1(%rdx), %r9d
;;       imulq   $0x18, %r9, %r14
;;       movq    0x90(%r13), %r10
;;       leaq    0x40(%rsp), %r11
;;       movq    0xa0(%rsp), %rcx
;;       cmpq    %rcx, %r15
;;       movq    %rcx, 0xa0(%rsp)
;;       jae     0x768
;;  4cf: movq    0xb0(%rsp), %r8
;;       movl    (%r8), %eax
;;       testl   %eax, %eax
;;       je      0xaf9
;;  4e2: movq    0x88(%r13), %rax
;;       movq    %rax, 0x20(%rsp)
;;       movl    $0, 0x28(%rsp)
;;       movl    $1, 0x2c(%rsp)
;;       movl    0x80(%r13), %eax
;;       movl    %eax, 0x30(%rsp)
;;       movl    $0, 0x80(%r13)
;;       movl    0x84(%r13), %eax
;;       movl    %eax, 0x34(%rsp)
;;       movl    $0, 0x84(%r13)
;;       movq    0x98(%r13), %r8
;;       movq    %r8, 0x38(%rsp)
;;       movq    0x98(%rsp), %rax
;;       movq    %rax, 0x88(%r13)
;;       cmpq    $8, %r8
;;       jae     0x823
;;  54f: movq    0x78(%rdi), %rax
;;       cmpq    %rax, %r9
;;       jae     0x852
;;  55c: movq    0x70(%rdi), %rcx
;;       movl    $0xffffffff, %eax
;;       addl    (%rcx, %r14), %eax
;;       cmpl    $1, %eax
;;       ja      0x881
;;  572: movl    4(%rcx, %r14), %eax
;;       cmpl    $2, %eax
;;       jne     0x8b0
;;  580: movl    8(%rcx, %r14), %eax
;;       movq    %rax, %rcx
;;       movl    $2, (%r10, %r8, 8)
;;       movl    %edx, 4(%r10, %r8, 8)
;;       leaq    1(%r8), %rax
;;       movq    %rax, 0x98(%r13)
;;       movq    8(%rsi), %rax
;;       movl    %ecx, %r12d
;;       cmpq    %rax, %r12
;;       jae     0xafb
;;  5b0: movq    (%rsi), %rax
;;       shlq    $4, %r12
;;       movq    8(%rax, %r12), %rcx
;;       cmpq    %rcx, %r15
;;       jae     0xafd
;;  5c5: movq    (%rax, %r12), %rax
;;       movzbq  (%rax, %r15), %r12
;;       movq    0x88(%r13), %rax
;;       movq    0x98(%rsp), %rcx
;;       cmpq    %rcx, %rax
;;       jne     0x925
;;  5e6: movq    0x20(%rsp), %rcx
;;       movq    %rcx, 0x88(%r13)
;;       movq    0x38(%rsp), %r8
;;       movq    %r8, 0x98(%r13)
;;       movl    0x30(%rsp), %r8d
;;       movl    %r8d, 0x80(%r13)
;;       movl    0x34(%rsp), %r8d
;;       movl    %r8d, 0x84(%r13)
;;       movq    0xb0(%rsp), %r8
;;       movl    (%r8), %eax
;;       testl   %eax, %eax
;;       je      0xaff
;;  629: movq    0x88(%r13), %rax
;;       movq    %rax, 0x40(%rsp)
;;       movl    $0, 0x48(%rsp)
;;       movl    $1, 0x4c(%rsp)
;;       movl    0x80(%r13), %eax
;;       movl    %eax, 0x50(%rsp)
;;       movl    $0, 0x80(%r13)
;;       movl    0x84(%r13), %eax
;;       movl    %eax, 0x54(%rsp)
;;       movl    $0, 0x84(%r13)
;;       movq    0x98(%r13), %r8
;;       movq    %r8, 0x58(%rsp)
;;       movq    %r11, 0x88(%r13)
;;       cmpq    $8, %r8
;;       jae     0x98a
;;  68e: movq    0x78(%rdi), %rax
;;       cmpq    %rax, %r9
;;       jae     0x9b9
;;  69b: movq    0x70(%rdi), %rcx
;;       movl    $0xffffffff, %eax
;;       addl    (%rcx, %r14), %eax
;;       cmpl    $1, %eax
;;       ja      0x9e8
;;  6b1: movl    4(%rcx, %r14), %eax
;;       cmpl    $2, %eax
;;       jne     0xa17
;;  6bf: movl    8(%rcx, %r14), %eax
;;       movq    %rax, 0x70(%rsp)
;;       movl    $2, (%r10, %r8, 8)
;;       movl    %edx, 4(%r10, %r8, 8)
;;       leaq    1(%r8), %rax
;;       movq    %rax, 0x98(%r13)
;;       movq    8(%rsi), %rcx
;;       movq    0x70(%rsp), %rax
;;       movl    %eax, %eax
;;       cmpq    %rcx, %rax
;;       jae     0xb01
;;  6f5: movq    (%rsi), %rcx
;;       shlq    $4, %rax
;;       movq    8(%rcx, %rax), %r8
;;       cmpq    %r8, %r15
;;       jae     0xb03
;;  70a: movq    (%rcx, %rax), %rax
;;       addb    $1, %r12b
;;       movb    %r12b, (%rax, %r15)
;;       movq    0x88(%r13), %rax
;;       cmpq    %r11, %rax
;;       jne     0xa8e
;;  726: movq    0x40(%rsp), %rax
;;       movq    %rax, 0x88(%r13)
;;       movq    0x58(%rsp), %rax
;;       movq    %rax, 0x98(%r13)
;;       movl    0x50(%rsp), %eax
;;       movl    %eax, 0x80(%r13)
;;       movl    0x54(%rsp), %eax
;;       movl    %eax, 0x84(%r13)
;;       addq    $1, %r15
;;       movq    0xa0(%rsp), %rcx
;;       jmp     0x4be
;;  768: movq    0x60(%rsp), %rcx
;;       movq    0x38(%rcx), %rax
;;       movq    0x48(%rcx), %rdi
;;       movq    %rcx, %rsi
;;       callq   *%rax
;;       movq    0xc0(%rsp), %rbx
;;       movq    0xc8(%rsp), %r12
;;       movq    0xd0(%rsp), %r13
;;       movq    0xd8(%rsp), %r14
;;       movq    0xe0(%rsp), %r15
;;       addq    $0xf0, %rsp
;;       movq    %rbp, %rsp
;;       popq    %rbp
;;       retq
;;  7ae: movq    %rdx, 0x68(%rsp)
;;  7b3: jmp     0x7d1
;;  7b8: movq    %rdx, 0x68(%rsp)
;;  7bd: jmp     0x7d1
;;  7c2: movq    %rdx, 0x68(%rsp)
;;  7c7: jmp     0x7d1
;;  7cc: movq    %rdx, 0x68(%rsp)
;;  7d1: movq    0x98(%rbx), %r10
;;  7d8: movq    0x68(%rbx), %rdi
;;  7dc: movq    0x68(%rsp), %rdx
;;  7e1: movq    %rbx, %rsi
;;  7e4: callq   *%r10
;;  7e7: movq    0x68(%rsp), %rdx
;;  7ec: movq    %rax, %rdi
;;  7ef: jmp     0x41f
;;  7f4: movq    %rdx, 0x68(%rsp)
;;  7f9: movq    %rsi, 0xa8(%rsp)
;;  801: movq    0xb8(%rbx), %rax
;;  808: movq    0x68(%rbx), %rdi
;;  80c: movq    %rbx, %rsi
;;  80f: callq   *%rax
;;  811: movq    0x68(%rsp), %rdx
;;  816: movq    0xa8(%rsp), %rsi
;;  81e: jmp     0x48e
;;  823: movq    %rdx, 0x68(%rsp)
;;  828: movq    %r11, 0x78(%rsp)
;;  82d: movq    %r10, 0x80(%rsp)
;;  835: movq    %r9, 0x88(%rsp)
;;  83d: movq    %rdi, 0x90(%rsp)
;;  845: movq    %rsi, 0xa8(%rsp)
;;  84d: jmp     0x8da
;;  852: movq    %rdx, 0x68(%rsp)
;;  857: movq    %r11, 0x78(%rsp)
;;  85c: movq    %r10, 0x80(%rsp)
;;  864: movq    %r9, 0x88(%rsp)
;;  86c: movq    %rdi, 0x90(%rsp)
;;  874: movq    %rsi, 0xa8(%rsp)
;;  87c: jmp     0x8da
;;  881: movq    %rdx, 0x68(%rsp)
;;  886: movq    %r11, 0x78(%rsp)
;;  88b: movq    %r10, 0x80(%rsp)
;;  893: movq    %r9, 0x88(%rsp)
;;  89b: movq    %rdi, 0x90(%rsp)
;;  8a3: movq    %rsi, 0xa8(%rsp)
;;  8ab: jmp     0x8da
;;  8b0: movq    %rdx, 0x68(%rsp)
;;  8b5: movq    %r11, 0x78(%rsp)
;;  8ba: movq    %r10, 0x80(%rsp)
;;  8c2: movq    %r9, 0x88(%rsp)
;;  8ca: movq    %rdi, 0x90(%rsp)
;;  8d2: movq    %rsi, 0xa8(%rsp)
;;  8da: movq    0x98(%rbx), %rax
;;  8e1: movq    0x68(%rsp), %rdx
;;  8e6: movq    %rbx, %rsi
;;  8e9: movq    0x90(%rsp), %rdi
;;  8f1: callq   *%rax
;;  8f3: movq    %rax, %rcx
;;  8f6: movq    0x68(%rsp), %rdx
;;  8fb: movq    0xa8(%rsp), %rsi
;;  903: movq    0x90(%rsp), %rdi
;;  90b: movq    0x88(%rsp), %r9
;;  913: movq    0x80(%rsp), %r10
;;  91b: movq    0x78(%rsp), %r11
;;  920: jmp     0x5a0
;;  925: movq    %rdx, 0x68(%rsp)
;;  92a: movq    %r11, 0x78(%rsp)
;;  92f: movq    %r10, 0x80(%rsp)
;;  937: movq    %r9, 0x88(%rsp)
;;  93f: movq    %rdi, 0x90(%rsp)
;;  947: movq    %rsi, 0xa8(%rsp)
;;  94f: movq    0xb8(%rbx), %rax
;;  956: movq    %rbx, %rsi
;;  959: callq   *%rax
;;  95b: movq    0x68(%rsp), %rdx
;;  960: movq    0xa8(%rsp), %rsi
;;  968: movq    0x90(%rsp), %rdi
;;  970: movq    0x88(%rsp), %r9
;;  978: movq    0x80(%rsp), %r10
;;  980: movq    0x78(%rsp), %r11
;;  985: jmp     0x616
;;  98a: movq    %rdx, 0x68(%rsp)
;;  98f: movq    %r11, 0x78(%rsp)
;;  994: movq    %r10, 0x80(%rsp)
;;  99c: movq    %r9, 0x88(%rsp)
;;  9a4: movq    %rdi, 0x90(%rsp)
;;  9ac: movq    %rsi, 0xa8(%rsp)
;;  9b4: jmp     0xa41
;;  9b9: movq    %rdx, 0x68(%rsp)
;;  9be: movq    %r11, 0x78(%rsp)
;;  9c3: movq    %r10, 0x80(%rsp)
;;  9cb: movq    %r9, 0x88(%rsp)
;;  9d3: movq    %rdi, 0x90(%rsp)
;;  9db: movq    %rsi, 0xa8(%rsp)
;;  9e3: jmp     0xa41
;;  9e8: movq    %rdx, 0x68(%rsp)
;;  9ed: movq    %r11, 0x78(%rsp)
;;  9f2: movq    %r10, 0x80(%rsp)
;;  9fa: movq    %r9, 0x88(%rsp)
;;  a02: movq    %rdi, 0x90(%rsp)
;;  a0a: movq    %rsi, 0xa8(%rsp)
;;  a12: jmp     0xa41
;;  a17: movq    %rdx, 0x68(%rsp)
;;  a1c: movq    %r11, 0x78(%rsp)
;;  a21: movq    %r10, 0x80(%rsp)
;;  a29: movq    %r9, 0x88(%rsp)
;;  a31: movq    %rdi, 0x90(%rsp)
;;  a39: movq    %rsi, 0xa8(%rsp)
;;  a41: movq    0x98(%rbx), %rax
;;  a48: movq    0x68(%rsp), %rdx
;;  a4d: movq    %rbx, %rsi
;;  a50: movq    0x90(%rsp), %rdi
;;  a58: callq   *%rax
;;  a5a: movq    0x68(%rsp), %rdx
;;  a5f: movq    0xa8(%rsp), %rsi
;;  a67: movq    0x90(%rsp), %rdi
;;  a6f: movq    0x88(%rsp), %r9
;;  a77: movq    0x80(%rsp), %r10
;;  a7f: movq    0x78(%rsp), %r11
;;  a84: movq    %rax, 0x70(%rsp)
;;  a89: jmp     0x6e1
;;  a8e: movq    %rdx, 0x68(%rsp)
;;  a93: movq    %r11, 0x78(%rsp)
;;  a98: movq    %r10, 0x80(%rsp)
;;  aa0: movq    %r9, 0x88(%rsp)
;;  aa8: movq    %rdi, 0x90(%rsp)
;;  ab0: movq    %rsi, 0xa8(%rsp)
;;  ab8: movq    0xb8(%rbx), %rax
;;  abf: movq    %rbx, %rsi
;;  ac2: callq   *%rax
;;  ac4: movq    0x68(%rsp), %rdx
;;  ac9: movq    0xa8(%rsp), %rsi
;;  ad1: movq    0x90(%rsp), %rdi
;;  ad9: movq    0x88(%rsp), %r9
;;  ae1: movq    0x80(%rsp), %r10
;;  ae9: movq    0x78(%rsp), %r11
;;  aee: jmp     0x754
;;  af3: ud2
;;  af5: ud2
;;  af7: ud2
;;  af9: ud2
;;  afb: ud2
;;  afd: ud2
;;  aff: ud2
;;  b01: ud2
;;  b03: ud2
