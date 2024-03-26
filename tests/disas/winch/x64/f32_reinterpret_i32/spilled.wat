;;! target = "x86_64"
;;! test = "winch"

(module
    (func (result f32)
        i32.const 1
        f32.reinterpret_i32
        block
        end
    )
)
;; wasm[0]::function[0]:
;;    0: pushq   %rbp
;;    1: movq    %rsp, %rbp
;;    4: movq    8(%rdi), %r11
;;    8: movq    (%r11), %r11
;;    b: addq    $0x14, %r11
;;   12: cmpq    %rsp, %r11
;;   15: ja      0x4c
;;   1b: movq    %rdi, %r14
;;   1e: subq    $0x10, %rsp
;;   22: movq    %rdi, 8(%rsp)
;;   27: movq    %rsi, (%rsp)
;;   2b: movl    $1, %eax
;;   30: movd    %eax, %xmm0
;;   34: subq    $4, %rsp
;;   38: movss   %xmm0, (%rsp)
;;   3d: movss   (%rsp), %xmm0
;;   42: addq    $4, %rsp
;;   46: addq    $0x10, %rsp
;;   4a: popq    %rbp
;;   4b: retq
;;   4c: ud2
