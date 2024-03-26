;;! target = "x86_64"
;;! test = "winch"

(module
    (func (param i64) (param i64) (result i64)
        (local.get 0)
        (local.get 1)
        (i64.xor)
    )
)
;; wasm[0]::function[0]:
;;    0: pushq   %rbp
;;    1: movq    %rsp, %rbp
;;    4: movq    8(%rdi), %r11
;;    8: movq    (%r11), %r11
;;    b: addq    $0x20, %r11
;;   12: cmpq    %rsp, %r11
;;   15: ja      0x4a
;;   1b: movq    %rdi, %r14
;;   1e: subq    $0x20, %rsp
;;   22: movq    %rdi, 0x18(%rsp)
;;   27: movq    %rsi, 0x10(%rsp)
;;   2c: movq    %rdx, 8(%rsp)
;;   31: movq    %rcx, (%rsp)
;;   35: movq    (%rsp), %rax
;;   39: movq    8(%rsp), %rcx
;;   3e: xorq    %rax, %rcx
;;   41: movq    %rcx, %rax
;;   44: addq    $0x20, %rsp
;;   48: popq    %rbp
;;   49: retq
;;   4a: ud2
