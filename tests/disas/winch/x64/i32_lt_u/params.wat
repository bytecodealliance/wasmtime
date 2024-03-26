;;! target = "x86_64"
;;! test = "winch"

(module
    (func (param i32) (param i32) (result i32)
        (local.get 0)
        (local.get 1)
        (i32.lt_u)
    )
)
;; wasm[0]::function[0]:
;;    0: pushq   %rbp
;;    1: movq    %rsp, %rbp
;;    4: movq    8(%rdi), %r11
;;    8: movq    (%r11), %r11
;;    b: addq    $0x18, %r11
;;   12: cmpq    %rsp, %r11
;;   15: ja      0x4d
;;   1b: movq    %rdi, %r14
;;   1e: subq    $0x18, %rsp
;;   22: movq    %rdi, 0x10(%rsp)
;;   27: movq    %rsi, 8(%rsp)
;;   2c: movl    %edx, 4(%rsp)
;;   30: movl    %ecx, (%rsp)
;;   33: movl    (%rsp), %eax
;;   36: movl    4(%rsp), %ecx
;;   3a: cmpl    %eax, %ecx
;;   3c: movl    $0, %ecx
;;   41: setb    %cl
;;   45: movl    %ecx, %eax
;;   47: addq    $0x18, %rsp
;;   4b: popq    %rbp
;;   4c: retq
;;   4d: ud2
