;;! target = "x86_64"
;;! test = "winch"
;;! flags = [ "-Ccranelift-has-avx" ]

(module
  (func (export "_start") (result v128)
        (v128.bitselect
          (v128.const i64x2 0x3298472837385628 0x58212382347A3994)
          (v128.const i64x2 0x7483929592465832 0x1285837491823847)
          (v128.const i64x2 0xFFFFFF0FFFFFFFFF 0xFFFFFF0FFFFFFFFF)
          )))
;; wasm[0]::function[0]:
;;       pushq   %rbp
;;       movq    %rsp, %rbp
;;       movq    8(%rdi), %r11
;;       movq    0x10(%r11), %r11
;;       addq    $0x10, %r11
;;       cmpq    %rsp, %r11
;;       ja      0x5a
;;   1c: movq    %rdi, %r14
;;       subq    $0x10, %rsp
;;       movq    %rdi, 8(%rsp)
;;       movq    %rsi, (%rsp)
;;       movdqu  0x2c(%rip), %xmm0
;;       movdqu  0x34(%rip), %xmm1
;;       movdqu  0x3c(%rip), %xmm2
;;       vpand   %xmm0, %xmm2, %xmm15
;;       vpandn  %xmm1, %xmm0, %xmm3
;;       vpor    %xmm3, %xmm15, %xmm3
;;       movdqa  %xmm3, %xmm0
;;       addq    $0x10, %rsp
;;       popq    %rbp
;;       retq
;;   5a: ud2
;;   5c: addb    %al, (%rax)
;;   5e: addb    %al, (%rax)
