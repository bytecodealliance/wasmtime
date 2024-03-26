;;! target = "x86_64"
;;! test = "winch"

(module
    (func (result f32)
        f32.const 1.0
        i32.reinterpret_f32
        drop
        f32.const 1.0
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
;;   2b: movss   0x15(%rip), %xmm0
;;   33: movd    %xmm0, %eax
;;   37: movss   9(%rip), %xmm0
;;   3f: addq    $0x10, %rsp
;;   43: popq    %rbp
;;   44: retq
;;   45: ud2
;;   47: addb    %al, (%rax)
