;;! target = "x86_64"
;;! test = "winch"
;;! flags = [ "-Ccranelift-has-avx" ]

(module
  (func (export "_start") (result v128)
        (v128.and 
          (v128.const i64x2 0 0xFFFFFFFFFFFFFFFF)
          (v128.const i64x2 0xFFFFFFFFFFFFFFFF 0)
          )))
;; wasm[0]::function[0]:
;;       pushq   %rbp
;;       movq    %rsp, %rbp
;;       movq    8(%rdi), %r11
;;       movq    0x10(%r11), %r11
;;       addq    $0x10, %r11
;;       cmpq    %rsp, %r11
;;       ja      0x4a
;;   1c: movq    %rdi, %r14
;;       subq    $0x10, %rsp
;;       movq    %rdi, 8(%rsp)
;;       movq    %rsi, (%rsp)
;;       movdqu  0x1c(%rip), %xmm0
;;       movdqu  0x24(%rip), %xmm1
;;       vpand   %xmm0, %xmm1, %xmm1
;;       movdqa  %xmm1, %xmm0
;;       addq    $0x10, %rsp
;;       popq    %rbp
;;       retq
;;   4a: ud2
;;   4c: addb    %al, (%rax)
;;   4e: addb    %al, (%rax)
