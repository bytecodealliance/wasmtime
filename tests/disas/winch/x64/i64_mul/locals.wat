;;! target = "x86_64"
;;! test = "winch"

(module
    (func (result i64)
        (local $foo i64)  
        (local $bar i64)

        (i64.const 10)
        (local.set $foo)

        (i64.const 20)
        (local.set $bar)

        (local.get $foo)
        (local.get $bar)
        i64.mul
    )
)
;; wasm[0]::function[0]:
;;    0: pushq   %rbp
;;    1: movq    %rsp, %rbp
;;    4: movq    8(%rdi), %r11
;;    8: movq    (%r11), %r11
;;    b: addq    $0x20, %r11
;;   12: cmpq    %rsp, %r11
;;   15: ja      0x65
;;   1b: movq    %rdi, %r14
;;   1e: subq    $0x20, %rsp
;;   22: movq    %rdi, 0x18(%rsp)
;;   27: movq    %rsi, 0x10(%rsp)
;;   2c: xorl    %r11d, %r11d
;;   2f: movq    %r11, 8(%rsp)
;;   34: movq    %r11, (%rsp)
;;   38: movq    $0xa, %rax
;;   3f: movq    %rax, 8(%rsp)
;;   44: movq    $0x14, %rax
;;   4b: movq    %rax, (%rsp)
;;   4f: movq    (%rsp), %rax
;;   53: movq    8(%rsp), %rcx
;;   58: imulq   %rax, %rcx
;;   5c: movq    %rcx, %rax
;;   5f: addq    $0x20, %rsp
;;   63: popq    %rbp
;;   64: retq
;;   65: ud2
