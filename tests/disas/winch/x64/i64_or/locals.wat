;;! target = "x86_64"
;;! test = "winch"

(module
    (func (result i64)
        (local $foo i64)
        (local $bar i64)

        (i64.const 2)
        (local.set $foo)
        (i64.const 3)
        (local.set $bar)

        (local.get $foo)
        (local.get $bar)
        (i64.or)
    )
)
;; wasm[0]::function[0]:
;;    0: pushq   %rbp
;;    1: movq    %rsp, %rbp
;;    4: movq    8(%rdi), %r11
;;    8: movq    (%r11), %r11
;;    b: addq    $0x20, %r11
;;   12: cmpq    %rsp, %r11
;;   15: ja      0x64
;;   1b: movq    %rdi, %r14
;;   1e: subq    $0x20, %rsp
;;   22: movq    %rdi, 0x18(%rsp)
;;   27: movq    %rsi, 0x10(%rsp)
;;   2c: xorl    %r11d, %r11d
;;   2f: movq    %r11, 8(%rsp)
;;   34: movq    %r11, (%rsp)
;;   38: movq    $2, %rax
;;   3f: movq    %rax, 8(%rsp)
;;   44: movq    $3, %rax
;;   4b: movq    %rax, (%rsp)
;;   4f: movq    (%rsp), %rax
;;   53: movq    8(%rsp), %rcx
;;   58: orq     %rax, %rcx
;;   5b: movq    %rcx, %rax
;;   5e: addq    $0x20, %rsp
;;   62: popq    %rbp
;;   63: retq
;;   64: ud2
