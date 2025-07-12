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
;;       ja      0x61
;;   1c: movq    %rdi, %r14
;;       subq    $0x10, %rsp
;;       movq    %rdi, 8(%rsp)
;;       movq    %rsi, (%rsp)
;;       movdqu  0x39(%rip), %xmm0
;;       movdqu  0x41(%rip), %xmm1
;;       movdqu  0x49(%rip), %xmm2
;;       vpand   %xmm0, %xmm2, %xmm15
;;       vpandn  %xmm1, %xmm0, %xmm3
;;       vpor    %xmm15, %xmm3, %xmm3
;;       movdqa  %xmm3, %xmm0
;;       addq    $0x10, %rsp
;;       popq    %rbp
;;       retq
;;   61: ud2
;;   63: addb    %al, (%rax)
;;   65: addb    %al, (%rax)
;;   67: addb    %al, (%rax)
;;   69: addb    %al, (%rax)
;;   6b: addb    %al, (%rax)
;;   6d: addb    %al, (%rax)
;;   6f: addb    %bh, %bh
