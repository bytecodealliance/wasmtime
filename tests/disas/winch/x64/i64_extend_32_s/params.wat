;;! target = "x86_64"
;;! test = "winch"

(module
    (func (param i64) (result i64)
        (local.get 0)
        (i64.extend32_s)
    )
)
;; wasm[0]::function[0]:
;;    0: pushq   %rbp
;;    1: movq    %rsp, %rbp
;;    4: movq    8(%rdi), %r11
;;    8: movq    (%r11), %r11
;;    b: addq    $0x18, %r11
;;   12: cmpq    %rsp, %r11
;;   15: ja      0x3d
;;   1b: movq    %rdi, %r14
;;   1e: subq    $0x18, %rsp
;;   22: movq    %rdi, 0x10(%rsp)
;;   27: movq    %rsi, 8(%rsp)
;;   2c: movq    %rdx, (%rsp)
;;   30: movq    (%rsp), %rax
;;   34: movslq  %eax, %rax
;;   37: addq    $0x18, %rsp
;;   3b: popq    %rbp
;;   3c: retq
;;   3d: ud2
