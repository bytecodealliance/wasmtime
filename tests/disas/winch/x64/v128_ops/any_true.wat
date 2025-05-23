;;! target = "x86_64"
;;! test = "winch"
;;! flags = [ "-Ccranelift-has-avx" ]

(module
  (func (export "_start") (result i32)
        (v128.any_true
          (v128.const i64x2 0 0xFFFFFFFFFFFFFFFF)
          )))
;; wasm[0]::function[0]:
;;       pushq   %rbp
;;       movq    %rsp, %rbp
;;       movq    8(%rdi), %r11
;;       movq    0x10(%r11), %r11
;;       addq    $0x10, %r11
;;       cmpq    %rsp, %r11
;;       ja      0x4e
;;   1c: movq    %rdi, %r14
;;       subq    $0x10, %rsp
;;       movq    %rdi, 8(%rsp)
;;       movq    %rsi, (%rsp)
;;       movdqu  0x19(%rip), %xmm0
;;       vptest  %xmm0, %xmm0
;;       movl    $0, %eax
;;       setne   %al
;;       addq    $0x10, %rsp
;;       popq    %rbp
;;       retq
;;   4e: ud2
;;   50: addb    %al, (%rax)
;;   52: addb    %al, (%rax)
;;   54: addb    %al, (%rax)
;;   56: addb    %al, (%rax)
