;;! target = "x86_64"
;;! test = "winch"

(module
    (func (result f32)
        (f32.const -1.32)
        (f32.abs)
    )
)
;; wasm[0]::function[0]:
;;    0: pushq   %rbp
;;    1: movq    %rsp, %rbp
;;    4: movq    8(%rdi), %r11
;;    8: movq    (%r11), %r11
;;    b: addq    $0x10, %r11
;;   12: cmpq    %rsp, %r11
;;   15: ja      0x48
;;   1b: movq    %rdi, %r14
;;   1e: subq    $0x10, %rsp
;;   22: movq    %rdi, 8(%rsp)
;;   27: movq    %rsi, (%rsp)
;;   2b: movss   0x1d(%rip), %xmm0
;;   33: movl    $0x7fffffff, %r11d
;;   39: movd    %r11d, %xmm15
;;   3e: andps   %xmm15, %xmm0
;;   42: addq    $0x10, %rsp
;;   46: popq    %rbp
;;   47: retq
;;   48: ud2
;;   4a: addb    %al, (%rax)
;;   4c: addb    %al, (%rax)
;;   4e: addb    %al, (%rax)
;;   50: retq
;;   51: cmc
;;   52: testb   $0xbf, %al
