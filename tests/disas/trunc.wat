;;! target = "x86_64-unknown-linux-gnu"
;;! test = "compile"
;;! flags = "-Ccompiler=cranelift -Ccranelift-has_sse41=false  -Osignals-based-traps=n"

(module
  (func (export "i64.trunc_f64_u") (param f64) (result i64)
    (i64.trunc_f64_u (local.get 0))
  )
)
;; wasm[0]::function[0]:
;;       movq    8(%rdi), %r10
;;       movq    0x10(%r10), %r10
;;       addq    $0x40, %r10
;;       cmpq    %rsp, %r10
;;       jbe     0x1a
;;       jmp     0x2fa
;;   1a: pushq   %rbp
;;       movq    %rsp, %rbp
;;       subq    $0x20, %rsp
;;       movq    %r13, 0x10(%rsp)
;;       movq    %rdi, %r13
;;       ucomisd %xmm0, %xmm0
;;       movdqu  %xmm0, (%rsp)
;;       jp      0x107
;;       jne     0x107
;;   3f: movq    %r13, %rdi
;;       movdqu  (%rsp), %xmm0
;;       callq   0x26f
;;       movabsq $13830554455654793216, %rax
;;       movq    %rax, %xmm1
;;       ucomisd %xmm0, %xmm1
;;       jae     0xf0
;;   65: ucomisd 0xc3(%rip), %xmm0
;;       jae     0xd9
;;   73: movdqu  (%rsp), %xmm2
;;       movabsq $0x43e0000000000000, %rcx
;;       movq    %rcx, %xmm1
;;       ucomisd %xmm1, %xmm2
;;       jae     0xa8
;;       jp      0x11e
;;   97: cvttsd2si %xmm2, %rax
;;       cmpq    $0, %rax
;;       jge     0xcb
;;   a6: ud2
;;       movaps  %xmm2, %xmm0
;;       subsd   %xmm1, %xmm0
;;       cvttsd2si %xmm0, %rax
;;       cmpq    $0, %rax
;;       jl      0x120
;;   be: movabsq $9223372036854775808, %rcx
;;       addq    %rcx, %rax
;;       movq    0x10(%rsp), %r13
;;       addq    $0x20, %rsp
;;       movq    %rbp, %rsp
;;       popq    %rbp
;;       retq
;;   d9: movl    $6, %esi
;;   de: movq    %r13, %rdi
;;   e1: callq   0x29c
;;   e6: movq    %r13, %rdi
;;   e9: callq   0x2cd
;;   ee: ud2
;;   f0: movl    $6, %esi
;;   f5: movq    %r13, %rdi
;;   f8: callq   0x29c
;;   fd: movq    %r13, %rdi
;;  100: callq   0x2cd
;;  105: ud2
;;  107: movl    $8, %esi
;;  10c: movq    %r13, %rdi
;;  10f: callq   0x29c
;;  114: movq    %r13, %rdi
;;  117: callq   0x2cd
;;  11c: ud2
;;  11e: ud2
;;  120: ud2
;;  122: addb    %al, (%rax)
;;  124: addb    %al, (%rax)
;;  126: addb    %al, (%rax)
;;  128: addb    %al, (%rax)
;;  12a: addb    %al, (%rax)
;;  12c: addb    %al, (%rax)
;;  12e: addb    %al, (%rax)
;;  130: addb    %al, (%rax)
;;  132: addb    %al, (%rax)
;;  134: addb    %al, (%rax)
;;  136: lock addb %al, (%r8)
;;  13a: addb    %al, (%rax)
;;  13c: addb    %al, (%rax)
;;  13e: addb    %al, (%rax)
