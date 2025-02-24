;;! target = "x86_64"
;;! test = "winch"
;;! flags = [ "-Ccranelift-has-avx=true" ]

(module
  (memory (data "\00\00\00\00\00\00\00\00\00\00\00\00\00\00\a0\7f"))

  (func (result v128) (v128.load64_zero (i32.const 0)))
)
;; wasm[0]::function[0]:
;;       pushq   %rbp
;;       movq    %rsp, %rbp
;;       movq    8(%rdi), %r11
;;       movq    0x10(%r11), %r11
;;       addq    $0x10, %r11
;;       cmpq    %rsp, %r11
;;       ja      0x46
;;   1c: movq    %rdi, %r14
;;       subq    $0x10, %rsp
;;       movq    %rdi, 8(%rsp)
;;       movq    %rsi, (%rsp)
;;       movl    $0, %eax
;;       movq    0x58(%r14), %rcx
;;       addq    %rax, %rcx
;;       movq    (%rcx), %r11
;;       vmovq   %r11, %xmm0
;;       addq    $0x10, %rsp
;;       popq    %rbp
;;       retq
;;   46: ud2
