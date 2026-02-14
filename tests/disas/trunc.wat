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
;;       movq    %rbx, 0x10(%rsp)
;;       movq    8(%rdi), %rax
;;       movq    %rdi, %rbx
;;       movq    0x18(%rax), %rax
;;       movq    %rsp, %rcx
;;       cmpq    %rax, %rcx
;;       jb      0x118
;;   24: ucomisd %xmm0, %xmm0
;;       movdqu  %xmm0, (%rsp)
;;       jp      0x101
;;       jne     0x101
;;   39: movq    %rbx, %rdi
;;       movdqu  (%rsp), %xmm0
;;       callq   0x244
;;       movabsq $13830554455654793216, %rax
;;       movq    %rax, %xmm1
;;       ucomisd %xmm0, %xmm1
;;       jae     0xea
;;   5f: ucomisd 0xc9(%rip), %xmm0
;;       jae     0xd3
;;   6d: movdqu  (%rsp), %xmm2
;;       movabsq $0x43e0000000000000, %rcx
;;       movq    %rcx, %xmm1
;;       ucomisd %xmm1, %xmm2
;;       jae     0xa2
;;       jp      0x12c
;;   91: cvttsd2si %xmm2, %rax
;;       cmpq    $0, %rax
;;       jge     0xc5
;;   a0: ud2
;;       movaps  %xmm2, %xmm0
;;       subsd   %xmm1, %xmm0
;;       cvttsd2si %xmm0, %rax
;;       cmpq    $0, %rax
;;       jl      0x12e
;;   b8: movabsq $9223372036854775808, %rcx
;;       addq    %rcx, %rax
;;       movq    0x10(%rsp), %rbx
;;       addq    $0x20, %rsp
;;       movq    %rbp, %rsp
;;       popq    %rbp
;;       retq
;;   d3: movl    $6, %esi
;;   d8: movq    %rbx, %rdi
;;   db: callq   0x271
;;   e0: movq    %rbx, %rdi
;;   e3: callq   0x2a2
;;   e8: ud2
;;   ea: movl    $6, %esi
;;   ef: movq    %rbx, %rdi
;;   f2: callq   0x271
;;   f7: movq    %rbx, %rdi
;;   fa: callq   0x2a2
;;   ff: ud2
;;  101: movl    $8, %esi
;;  106: movq    %rbx, %rdi
;;  109: callq   0x271
;;  10e: movq    %rbx, %rdi
;;  111: callq   0x2a2
;;  116: ud2
;;  118: xorl    %esi, %esi
;;  11a: movq    %rbx, %rdi
;;  11d: callq   0x271
;;  122: movq    %rbx, %rdi
;;  125: callq   0x2a2
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
