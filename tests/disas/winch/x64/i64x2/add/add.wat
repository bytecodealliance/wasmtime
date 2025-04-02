;;! target = "x86_64"
;;! test = "winch"
;;! flags = [ "-Ccranelift-has-avx" ]

(module
  (memory 1 1)
  (func (result v128)
        (i64x2.add
          (i64x2.splat (i64.const 10))
          (i64x2.splat (i64.const 10))
          )))
;; wasm[0]::function[0]:
;;       pushq   %rbp
;;       movq    %rsp, %rbp
;;       movq    8(%rdi), %r11
;;       movq    0x10(%r11), %r11
;;       addq    $0x10, %r11
;;       cmpq    %rsp, %r11
;;       ja      0x48
;;   1c: movq    %rdi, %r14
;;       subq    $0x10, %rsp
;;       movq    %rdi, 8(%rsp)
;;       movq    %rsi, (%rsp)
;;       vpshufd $0x44, 0x1b(%rip), %xmm0
;;       vpshufd $0x44, 0x12(%rip), %xmm1
;;       vpaddq  %xmm1, %xmm0, %xmm0
;;       addq    $0x10, %rsp
;;       popq    %rbp
;;       retq
;;   48: ud2
;;   4a: addb    %al, (%rax)
;;   4c: addb    %al, (%rax)
;;   4e: addb    %al, (%rax)
;;   50: orb     (%rax), %al
;;   52: addb    %al, (%rax)
;;   54: addb    %al, (%rax)
;;   56: addb    %al, (%rax)
