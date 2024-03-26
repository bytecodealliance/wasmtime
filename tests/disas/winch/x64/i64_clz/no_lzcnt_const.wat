;;! target = "x86_64"
;;! test = "winch"

(module
    (func (result i64)
        (i64.const 1)
        (i64.clz)
    )
)
;; wasm[0]::function[0]:
;;    0: pushq   %rbp
;;    1: movq    %rsp, %rbp
;;    4: movq    8(%rdi), %r11
;;    8: movq    (%r11), %r11
;;    b: addq    $0x10, %r11
;;   12: cmpq    %rsp, %r11
;;   15: ja      0x50
;;   1b: movq    %rdi, %r14
;;   1e: subq    $0x10, %rsp
;;   22: movq    %rdi, 8(%rsp)
;;   27: movq    %rsi, (%rsp)
;;   2b: movq    $1, %rax
;;   32: bsrq    %rax, %rax
;;   36: movl    $0, %r11d
;;   3c: setne   %r11b
;;   40: negq    %rax
;;   43: addq    $0x40, %rax
;;   47: subq    %r11, %rax
;;   4a: addq    $0x10, %rsp
;;   4e: popq    %rbp
;;   4f: retq
;;   50: ud2
