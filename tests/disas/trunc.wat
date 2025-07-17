;;! target = "x86_64-unknown-linux-gnu"
;;! test = "compile"
;;! flags = "-Ccompiler=cranelift -Ccranelift-has_sse41=false  -Osignals-based-traps=n"

(module
  (func (export "i64.trunc_f64_u") (param f64) (result i64)
    (i64.trunc_f64_u (local.get 0))
  )
)
;; wasm[0]::function[0]:
;;       pushq   %rbp
;;       movq    %rsp, %rbp
;;       subq    $0x20, %rsp
;;       movq    %r14, 0x10(%rsp)
;;       movq    8(%rdi), %r11
;;       movq    %rdi, %r14
;;       movq    0x10(%r11), %r11
;;       movq    %rsp, %rsi
;;       cmpq    %r11, %rsi
;;       jb      0x125
;;   24: ucomisd %xmm0, %xmm0
;;       movdqu  %xmm0, (%rsp)
;;       jp      0x10e
;;       jne     0x10e
;;   39: movq    %r14, %rdi
;;       movdqu  (%rsp), %xmm0
;;       callq   0x21d
;;       movabsq $13830554455654793216, %rax
;;       movq    %rax, %xmm6
;;       ucomisd %xmm0, %xmm6
;;       jae     0xf7
;;   5f: ucomisd 0x69(%rip), %xmm0
;;       jae     0xe0
;;   6d: movdqu  (%rsp), %xmm1
;;       movabsq $0x43e0000000000000, %r10
;;       movq    %r10, %xmm7
;;       ucomisd %xmm7, %xmm1
;;       jae     0x9a
;;       jp      0xcb
;;   8d: cvttsd2si %xmm1, %rax
;;       cmpq    $0, %rax
;;       jge     0xbd
;;   98: ud2
;;       movaps  %xmm1, %xmm0
;;       subsd   %xmm7, %xmm0
;;       cvttsd2si %xmm0, %rax
;;       cmpq    $0, %rax
;;       jl      0xcd
;;   b0: movabsq $9223372036854775808, %r10
;;       addq    %r10, %rax
;;       movq    0x10(%rsp), %r14
;;       addq    $0x20, %rsp
;;       movq    %rbp, %rsp
;;       popq    %rbp
;;       retq
;;   cb: ud2
;;   cd: ud2
;;   cf: addb    %al, (%rax)
;;   d1: addb    %al, (%rax)
;;   d3: addb    %al, (%rax)
;;   d5: addb    %dh, %al
;;   d7: addb    %al, (%r8)
;;   da: addb    %al, (%rax)
;;   dc: addb    %al, (%rax)
;;   de: addb    %al, (%rax)
;;   e0: movl    $6, %esi
;;   e5: movq    %r14, %rdi
;;   e8: callq   0x25c
;;   ed: movq    %r14, %rdi
;;   f0: callq   0x29f
;;   f5: ud2
;;   f7: movl    $6, %esi
;;   fc: movq    %r14, %rdi
;;   ff: callq   0x25c
;;  104: movq    %r14, %rdi
;;  107: callq   0x29f
;;  10c: ud2
;;  10e: movl    $8, %esi
;;  113: movq    %r14, %rdi
;;  116: callq   0x25c
;;  11b: movq    %r14, %rdi
;;  11e: callq   0x29f
;;  123: ud2
;;  125: xorl    %esi, %esi
;;  127: movq    %r14, %rdi
;;  12a: callq   0x25c
;;  12f: movq    %r14, %rdi
;;  132: callq   0x29f
;;  137: ud2
