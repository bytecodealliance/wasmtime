;;! target = "x86_64"
;;! test = "winch"

(module
    (func (result i32)
        (i64.const 2)
        (i64.const 3)
        (i64.eq)
    )
)
;; wasm[0]::function[0]:
;;    0: pushq   %rbp
;;    1: movq    %rsp, %rbp
;;    4: movq    8(%rdi), %r11
;;    8: movq    (%r11), %r11
;;    b: addq    $0x10, %r11
;;   12: cmpq    %rsp, %r11
;;   15: ja      0x45
;;   1b: movq    %rdi, %r14
;;   1e: subq    $0x10, %rsp
;;   22: movq    %rdi, 8(%rsp)
;;   27: movq    %rsi, (%rsp)
;;   2b: movq    $2, %rax
;;   32: cmpq    $3, %rax
;;   36: movl    $0, %eax
;;   3b: sete    %al
;;   3f: addq    $0x10, %rsp
;;   43: popq    %rbp
;;   44: retq
;;   45: ud2
