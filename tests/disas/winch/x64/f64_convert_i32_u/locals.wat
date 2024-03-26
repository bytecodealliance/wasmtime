;;! target = "x86_64"
;;! test = "winch"

(module
    (func (result f64)
        (local i32)  

        (local.get 0)
        (f64.convert_i32_u)
    )
)
;; wasm[0]::function[0]:
;;    0: pushq   %rbp
;;    1: movq    %rsp, %rbp
;;    4: movq    8(%rdi), %r11
;;    8: movq    (%r11), %r11
;;    b: addq    $0x18, %r11
;;   12: cmpq    %rsp, %r11
;;   15: ja      0x6e
;;   1b: movq    %rdi, %r14
;;   1e: subq    $0x18, %rsp
;;   22: movq    %rdi, 0x10(%rsp)
;;   27: movq    %rsi, 8(%rsp)
;;   2c: movq    $0, (%rsp)
;;   34: movl    4(%rsp), %ecx
;;   38: movl    %ecx, %ecx
;;   3a: cmpq    $0, %rcx
;;   3e: jl      0x4e
;;   44: cvtsi2sdq %rcx, %xmm0
;;   49: jmp     0x68
;;   4e: movq    %rcx, %r11
;;   51: shrq    $1, %r11
;;   55: movq    %rcx, %rax
;;   58: andq    $1, %rax
;;   5c: orq     %r11, %rax
;;   5f: cvtsi2sdq %rax, %xmm0
;;   64: addsd   %xmm0, %xmm0
;;   68: addq    $0x18, %rsp
;;   6c: popq    %rbp
;;   6d: retq
;;   6e: ud2
