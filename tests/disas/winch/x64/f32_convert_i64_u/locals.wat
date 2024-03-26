;;! target = "x86_64"
;;! test = "winch"

(module
    (func (result f32)
        (local i64)  

        (local.get 0)
        (f32.convert_i64_u)
    )
)
;; wasm[0]::function[0]:
;;    0: pushq   %rbp
;;    1: movq    %rsp, %rbp
;;    4: movq    8(%rdi), %r11
;;    8: movq    (%r11), %r11
;;    b: addq    $0x18, %r11
;;   12: cmpq    %rsp, %r11
;;   15: ja      0x6c
;;   1b: movq    %rdi, %r14
;;   1e: subq    $0x18, %rsp
;;   22: movq    %rdi, 0x10(%rsp)
;;   27: movq    %rsi, 8(%rsp)
;;   2c: movq    $0, (%rsp)
;;   34: movq    (%rsp), %rcx
;;   38: cmpq    $0, %rcx
;;   3c: jl      0x4c
;;   42: cvtsi2ssq %rcx, %xmm0
;;   47: jmp     0x66
;;   4c: movq    %rcx, %r11
;;   4f: shrq    $1, %r11
;;   53: movq    %rcx, %rax
;;   56: andq    $1, %rax
;;   5a: orq     %r11, %rax
;;   5d: cvtsi2ssq %rax, %xmm0
;;   62: addss   %xmm0, %xmm0
;;   66: addq    $0x18, %rsp
;;   6a: popq    %rbp
;;   6b: retq
;;   6c: ud2
