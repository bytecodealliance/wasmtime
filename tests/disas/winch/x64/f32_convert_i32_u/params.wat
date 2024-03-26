;;! target = "x86_64"
;;! test = "winch"

(module
    (func (param i32) (result f32)
        (local.get 0)
        (f32.convert_i32_u)
    )
)
;; wasm[0]::function[0]:
;;    0: pushq   %rbp
;;    1: movq    %rsp, %rbp
;;    4: movq    8(%rdi), %r11
;;    8: movq    (%r11), %r11
;;    b: addq    $0x18, %r11
;;   12: cmpq    %rsp, %r11
;;   15: ja      0x6a
;;   1b: movq    %rdi, %r14
;;   1e: subq    $0x18, %rsp
;;   22: movq    %rdi, 0x10(%rsp)
;;   27: movq    %rsi, 8(%rsp)
;;   2c: movl    %edx, 4(%rsp)
;;   30: movl    4(%rsp), %ecx
;;   34: movl    %ecx, %ecx
;;   36: cmpq    $0, %rcx
;;   3a: jl      0x4a
;;   40: cvtsi2ssq %rcx, %xmm0
;;   45: jmp     0x64
;;   4a: movq    %rcx, %r11
;;   4d: shrq    $1, %r11
;;   51: movq    %rcx, %rax
;;   54: andq    $1, %rax
;;   58: orq     %r11, %rax
;;   5b: cvtsi2ssq %rax, %xmm0
;;   60: addss   %xmm0, %xmm0
;;   64: addq    $0x18, %rsp
;;   68: popq    %rbp
;;   69: retq
;;   6a: ud2
