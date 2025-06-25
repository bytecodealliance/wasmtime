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
;;       jb      0x118
;;   24: ucomisd %xmm0, %xmm0
;;       movdqu  %xmm0, (%rsp)
;;       jp      0x101
;;       jne     0x101
;;   39: movq    %r14, %rdi
;;       movdqu  (%rsp), %xmm0
;;       callq   0x224
;;       movabsq $13830554455654793216, %rax
;;       movq    %rax, %xmm6
;;       ucomisd %xmm0, %xmm6
;;       jae     0xea
;;   5f: ucomisd 0xc9(%rip), %xmm0
;;       jae     0xd3
;;   6d: movdqu  (%rsp), %xmm1
;;       movabsq $0x43e0000000000000, %r10
;;       movq    %r10, %xmm7
;;       ucomisd %xmm7, %xmm1
;;       jae     0xa2
;;       jp      0x12c
;;   91: cvttsd2si %xmm1, %rax
;;       cmpq    $0, %rax
;;       jge     0xc5
;;   a0: ud2
;;       movaps  %xmm1, %xmm0
;;       subsd   %xmm7, %xmm0
;;       cvttsd2si %xmm0, %rax
;;       cmpq    $0, %rax
;;       jl      0x12e
;;   b8: movabsq $9223372036854775808, %r10
;;       addq    %r10, %rax
;;       movq    0x10(%rsp), %r14
;;       addq    $0x20, %rsp
;;       movq    %rbp, %rsp
;;       popq    %rbp
;;       retq
;;   d3: movl    $6, %esi
;;   d8: movq    %r14, %rdi
;;   db: callq   0x263
;;   e0: movq    %r14, %rdi
;;   e3: callq   0x2a6
;;   e8: ud2
;;   ea: movl    $6, %esi
;;   ef: movq    %r14, %rdi
;;   f2: callq   0x263
;;   f7: movq    %r14, %rdi
;;   fa: callq   0x2a6
;;   ff: ud2
;;  101: movl    $8, %esi
;;  106: movq    %r14, %rdi
;;  109: callq   0x263
;;  10e: movq    %r14, %rdi
;;  111: callq   0x2a6
;;  116: ud2
;;  118: xorl    %esi, %esi
;;  11a: movq    %r14, %rdi
;;  11d: callq   0x263
;;  122: movq    %r14, %rdi
;;  125: callq   0x2a6
;;  12a: ud2
;;  12c: ud2
;;  12e: ud2
;;  130: addb    %al, (%rax)
;;  132: addb    %al, (%rax)
;;  134: addb    %al, (%rax)
;;  136: lock addb %al, (%r8)
;;  13a: addb    %al, (%rax)
;;  13c: addb    %al, (%rax)
;;  13e: addb    %al, (%rax)
