;;! target = "x86_64"
;;! test = "winch"
;;! flags = [ "-Ccranelift-has-avx=true", "-Omemory-reservation=0" ]

(module
  (memory (data "\00\00\00\00\00\00\00\00\00\00\00\00\00\00\a0\7f"))

  (func (export "v128.load32x2_s") (result v128) (v128.load32x2_s (i32.const 65529)))
)
;; wasm[0]::function[0]:
;;       pushq   %rbp
;;       movq    %rsp, %rbp
;;       movq    8(%rdi), %r11
;;       movq    0x10(%r11), %r11
;;       addq    $0x10, %r11
;;       cmpq    %rsp, %r11
;;       ja      0x72
;;   1c: movq    %rdi, %r14
;;       subq    $0x10, %rsp
;;       movq    %rdi, 8(%rsp)
;;       movq    %rsi, (%rsp)
;;       movl    $0xfff9, %eax
;;       movl    $0x10000, %ecx
;;       movl    %eax, %edx
;;       addq    $8, %rdx
;;       jb      0x74
;;   48: cmpq    %rcx, %rdx
;;       ja      0x76
;;   51: movq    0x38(%r14), %rbx
;;       addq    %rax, %rbx
;;       movl    $0, %esi
;;       cmpq    %rcx, %rdx
;;       cmovaq  %rsi, %rbx
;;       vpmovsxdq (%rbx), %xmm0
;;       addq    $0x10, %rsp
;;       popq    %rbp
;;       retq
;;   72: ud2
;;   74: ud2
;;   76: ud2
