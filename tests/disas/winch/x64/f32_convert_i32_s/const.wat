;;! target = "x86_64"
;;! test = "winch"

(module
    (func (result f32)
        (i32.const 1)
        (f32.convert_i32_s)
    )
)
;; wasm[0]::function[0]:
;;    0: pushq   %rbp
;;    1: movq    %rsp, %rbp
;;    4: movq    8(%rdi), %r11
;;    8: movq    (%r11), %r11
;;    b: addq    $0x10, %r11
;;   12: cmpq    %rsp, %r11
;;   15: ja      0x3a
;;   1b: movq    %rdi, %r14
;;   1e: subq    $0x10, %rsp
;;   22: movq    %rdi, 8(%rsp)
;;   27: movq    %rsi, (%rsp)
;;   2b: movl    $1, %eax
;;   30: cvtsi2ssl %eax, %xmm0
;;   34: addq    $0x10, %rsp
;;   38: popq    %rbp
;;   39: retq
;;   3a: ud2
