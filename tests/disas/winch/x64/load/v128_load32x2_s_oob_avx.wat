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
;;       ja      0x76
;;   1c: movq    %rdi, %r14
;;       subq    $0x10, %rsp
;;       movq    %rdi, 8(%rsp)
;;       movq    %rsi, (%rsp)
;;       movl    $0xfff9, %eax
;;       movq    $0x10000, %rcx
;;       movl    %eax, %edx
;;       addq    $8, %rdx
;;       jb      0x78
;;   4a: cmpq    %rcx, %rdx
;;       ja      0x7a
;;   53: movq    0x40(%r14), %rbx
;;       addq    %rax, %rbx
;;       movq    $0, %rsi
;;       cmpq    %rcx, %rdx
;;       cmovaq  %rsi, %rbx
;;       vpmovsxdq (%rbx), %xmm0
;;       addq    $0x10, %rsp
;;       popq    %rbp
;;       retq
;;   76: ud2
;;   78: ud2
;;   7a: ud2
