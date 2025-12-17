;;! target = "x86_64"
;;! test = "winch"
;;! flags = [ "-Ccranelift-has-avx=true", "-Omemory-reservation=0" ]

(module
  (memory (data "\00\00\00\00\00\00\00\00\00\00\00\00\00\00\a0\7f"))

  (func (export "v128.load32x2_u") (result v128) (v128.load32x2_u (i32.const 65529)))
)
;; wasm[0]::function[0]:
;;       pushq   %rbp
;;       movq    %rsp, %rbp
;;       movq    8(%rdi), %r11
;;       movq    0x18(%r11), %r11
;;       addq    $0x10, %r11
;;       cmpq    %rsp, %r11
;;       ja      0x64
;;   1c: movq    %rdi, %r14
;;       subq    $0x10, %rsp
;;       movq    %rdi, 8(%rsp)
;;       movq    %rsi, (%rsp)
;;       movl    $0xfff9, %eax
;;       cmpq    $0xfff8, %rax
;;       ja      0x66
;;   40: movq    0x38(%r14), %rcx
;;       addq    %rax, %rcx
;;       movl    $0, %edx
;;       cmpq    $0xfff8, %rax
;;       cmovaq  %rdx, %rcx
;;       vpmovzxdq (%rcx), %xmm0
;;       addq    $0x10, %rsp
;;       popq    %rbp
;;       retq
;;   64: ud2
;;   66: ud2
