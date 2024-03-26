;;! target = "x86_64"
;;! test = "winch"

(module
    (func (result i32)
        (i32.const 1)
        (i32.ctz)
    )
)
;; wasm[0]::function[0]:
;;    0: pushq   %rbp
;;    1: movq    %rsp, %rbp
;;    4: movq    8(%rdi), %r11
;;    8: movq    (%r11), %r11
;;    b: addq    $0x10, %r11
;;   12: cmpq    %rsp, %r11
;;   15: ja      0x4a
;;   1b: movq    %rdi, %r14
;;   1e: subq    $0x10, %rsp
;;   22: movq    %rdi, 8(%rsp)
;;   27: movq    %rsi, (%rsp)
;;   2b: movl    $1, %eax
;;   30: bsfl    %eax, %eax
;;   33: movl    $0, %r11d
;;   39: sete    %r11b
;;   3d: shll    $5, %r11d
;;   41: addl    %r11d, %eax
;;   44: addq    $0x10, %rsp
;;   48: popq    %rbp
;;   49: retq
;;   4a: ud2
