;;! target = "x86_64"
;;! test = "winch"
;;! flags = [ "-Ccranelift-has-avx512vl", "-Ccranelift-has-avx", "-Ccranelift-has-avx512dq", ]

(module
  (memory 1 1)
  (func (result v128)
        (i64x2.mul
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
;;       ja      0x50
;;   1c: movq    %rdi, %r14
;;       subq    $0x10, %rsp
;;       movq    %rdi, 8(%rsp)
;;       movq    %rsi, (%rsp)
;;       vpshufd $0x44, 0x20(%rip), %xmm0
;;       vpshufd $0x44, 0x17(%rip), %xmm1
;;       vpmullq %xmm1, %xmm0, %xmm0
;;       addq    $0x10, %rsp
;;       popq    %rbp
;;       retq
;;   50: ud2
;;   52: addb    %al, (%rax)
;;   54: addb    %al, (%rax)
;;   56: addb    %al, (%rax)
;;   58: orb     (%rax), %al
;;   5a: addb    %al, (%rax)
;;   5c: addb    %al, (%rax)
;;   5e: addb    %al, (%rax)
