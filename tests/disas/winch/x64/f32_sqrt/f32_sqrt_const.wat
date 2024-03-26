;;! target = "x86_64"
;;! test = "winch"

(module
    (func (result f32)
        (f32.const 1.32)
        (f32.sqrt)
    )
)
;; wasm[0]::function[0]:
;;    0: pushq   %rbp
;;    1: movq    %rsp, %rbp
;;    4: movq    8(%rdi), %r11
;;    8: movq    (%r11), %r11
;;    b: addq    $0x10, %r11
;;   12: cmpq    %rsp, %r11
;;   15: ja      0x3d
;;   1b: movq    %rdi, %r14
;;   1e: subq    $0x10, %rsp
;;   22: movq    %rdi, 8(%rsp)
;;   27: movq    %rsi, (%rsp)
;;   2b: movss   0xd(%rip), %xmm0
;;   33: sqrtss  %xmm0, %xmm0
;;   37: addq    $0x10, %rsp
;;   3b: popq    %rbp
;;   3c: retq
;;   3d: ud2
;;   3f: addb    %al, %bl
;;   41: cmc
;;   42: testb   $0x3f, %al
