;;! target = "x86_64"
;;! test = "winch"
;;! flags = [ "-Ccranelift-has-avx" ]

(module
  (func (param i64)
    f64.const 0
    f64.neg
    f64.const 0
    f64.neg
    f64.const 0
    f64.neg
    f64.const 0
    f64.neg
    f64.const 0
    f64.neg
    f64.const 0
    f64.neg
    f64.const 0
    f64.neg
    f64.const 0
    f64.neg
    f64.const 0
    f64.neg
    f64.const 0
    f64.neg
    f64.const 0
    f64.neg
    f64.const 0
    f64.neg
    f64.const 0
    f64.neg
    f64.const 0
    f64.neg

    local.get 0

    v128.const i64x2 0 0
    i32.const 0
    i64x2.shr_s
    unreachable
  )
)
;; wasm[0]::function[0]:
;;       pushq   %rbp
;;       movq    %rsp, %rbp
;;       movq    8(%rdi), %r11
;;       movq    0x18(%r11), %r11
;;       addq    $0x98, %r11
;;       cmpq    %rsp, %r11
;;       ja      0x2b4
;;   1c: movq    %rdi, %r14
;;       subq    $0x20, %rsp
;;       movq    %rdi, 0x18(%rsp)
;;       movq    %rsi, 0x10(%rsp)
;;       movq    %rdx, 8(%rsp)
;;       movsd   0x27b(%rip), %xmm0
;;       movabsq $9223372036854775808, %r11
;;       movq    %r11, %xmm15
;;       xorpd   %xmm15, %xmm0
;;       movsd   0x25f(%rip), %xmm1
;;       movabsq $9223372036854775808, %r11
;;       movq    %r11, %xmm15
;;       xorpd   %xmm15, %xmm1
;;       movsd   0x243(%rip), %xmm2
;;       movabsq $9223372036854775808, %r11
;;       movq    %r11, %xmm15
;;       xorpd   %xmm15, %xmm2
;;       movsd   0x227(%rip), %xmm3
;;       movabsq $9223372036854775808, %r11
;;       movq    %r11, %xmm15
;;       xorpd   %xmm15, %xmm3
;;       movsd   0x20b(%rip), %xmm4
;;       movabsq $9223372036854775808, %r11
;;       movq    %r11, %xmm15
;;       xorpd   %xmm15, %xmm4
;;       movsd   0x1ef(%rip), %xmm5
;;       movabsq $9223372036854775808, %r11
;;       movq    %r11, %xmm15
;;       xorpd   %xmm15, %xmm5
;;       movsd   0x1d3(%rip), %xmm6
;;       movabsq $9223372036854775808, %r11
;;       movq    %r11, %xmm15
;;       xorpd   %xmm15, %xmm6
;;       movsd   0x1b7(%rip), %xmm7
;;       movabsq $9223372036854775808, %r11
;;       movq    %r11, %xmm15
;;       xorpd   %xmm15, %xmm7
;;       movsd   0x19a(%rip), %xmm8
;;       movabsq $9223372036854775808, %r11
;;       movq    %r11, %xmm15
;;       xorpd   %xmm15, %xmm8
;;       movsd   0x17d(%rip), %xmm9
;;       movabsq $9223372036854775808, %r11
;;       movq    %r11, %xmm15
;;       xorpd   %xmm15, %xmm9
;;       movsd   0x160(%rip), %xmm10
;;       movabsq $9223372036854775808, %r11
;;       movq    %r11, %xmm15
;;       xorpd   %xmm15, %xmm10
;;       movsd   0x143(%rip), %xmm11
;;       movabsq $9223372036854775808, %r11
;;       movq    %r11, %xmm15
;;       xorpd   %xmm15, %xmm11
;;       movsd   0x126(%rip), %xmm12
;;       movabsq $9223372036854775808, %r11
;;       movq    %r11, %xmm15
;;       xorpd   %xmm15, %xmm12
;;       movsd   0x109(%rip), %xmm13
;;       movabsq $9223372036854775808, %r11
;;       movq    %r11, %xmm15
;;       xorpd   %xmm15, %xmm13
;;       movl    $0, %eax
;;       movdqu  0xef(%rip), %xmm14
;;       andl    $0x3f, %eax
;;       subq    $8, %rsp
;;       movsd   %xmm0, (%rsp)
;;       subq    $8, %rsp
;;       movsd   %xmm1, (%rsp)
;;       subq    $8, %rsp
;;       movsd   %xmm2, (%rsp)
;;       subq    $8, %rsp
;;       movsd   %xmm3, (%rsp)
;;       subq    $8, %rsp
;;       movsd   %xmm4, (%rsp)
;;       subq    $8, %rsp
;;       movsd   %xmm5, (%rsp)
;;       subq    $8, %rsp
;;       movsd   %xmm6, (%rsp)
;;       subq    $8, %rsp
;;       movsd   %xmm7, (%rsp)
;;       subq    $8, %rsp
;;       movsd   %xmm8, (%rsp)
;;       subq    $8, %rsp
;;       movsd   %xmm9, (%rsp)
;;       subq    $8, %rsp
;;       movsd   %xmm10, (%rsp)
;;       subq    $8, %rsp
;;       movsd   %xmm11, (%rsp)
;;       subq    $8, %rsp
;;       movsd   %xmm12, (%rsp)
;;       subq    $8, %rsp
;;       movsd   %xmm13, (%rsp)
;;       movq    0x78(%rsp), %r11
;;       pushq   %r11
;;       vmovd   %eax, %xmm15
;;       vmovdqu 0x39(%rip), %xmm0
;;       vpsrlq  %xmm15, %xmm0, %xmm0
;;       vpsrlq  %xmm15, %xmm14, %xmm14
;;       vpxor   %xmm0, %xmm14, %xmm14
;;       vpsubq  %xmm0, %xmm14, %xmm14
;;       ud2
;;       addq    $0x20, %rsp
;;       popq    %rbp
;;       retq
;;  2b4: ud2
;;  2b6: addb    %al, (%rax)
;;  2b8: addb    %al, (%rax)
;;  2ba: addb    %al, (%rax)
;;  2bc: addb    %al, (%rax)
;;  2be: addb    %al, (%rax)
;;  2c0: addb    %al, (%rax)
;;  2c2: addb    %al, (%rax)
;;  2c4: addb    %al, (%rax)
;;  2c6: addb    %al, (%rax)
;;  2c8: addb    %al, (%rax)
;;  2ca: addb    %al, (%rax)
;;  2cc: addb    %al, (%rax)
;;  2ce: addb    %al, (%rax)
;;  2d0: addb    %al, (%rax)
;;  2d2: addb    %al, (%rax)
;;  2d4: addb    %al, (%rax)
;;  2d6: addb    %al, (%rax)
;;  2dc: addb    %al, (%rax)
