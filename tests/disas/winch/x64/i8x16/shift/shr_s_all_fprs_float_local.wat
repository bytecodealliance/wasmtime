;;! target = "x86_64"
;;! test = "winch"
;;! flags = [ "-Ccranelift-has-avx" ]

(module
  (func (param f64)
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
    i8x16.shr_s
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
;;       ja      0x2c5
;;   1c: movq    %rdi, %r14
;;       subq    $0x20, %rsp
;;       movq    %rdi, 0x18(%rsp)
;;       movq    %rsi, 0x10(%rsp)
;;       movsd   %xmm0, 8(%rsp)
;;       movsd   0x28a(%rip), %xmm0
;;       movabsq $9223372036854775808, %r11
;;       movq    %r11, %xmm15
;;       xorpd   %xmm15, %xmm0
;;       movsd   0x26e(%rip), %xmm1
;;       movabsq $9223372036854775808, %r11
;;       movq    %r11, %xmm15
;;       xorpd   %xmm15, %xmm1
;;       movsd   0x252(%rip), %xmm2
;;       movabsq $9223372036854775808, %r11
;;       movq    %r11, %xmm15
;;       xorpd   %xmm15, %xmm2
;;       movsd   0x236(%rip), %xmm3
;;       movabsq $9223372036854775808, %r11
;;       movq    %r11, %xmm15
;;       xorpd   %xmm15, %xmm3
;;       movsd   0x21a(%rip), %xmm4
;;       movabsq $9223372036854775808, %r11
;;       movq    %r11, %xmm15
;;       xorpd   %xmm15, %xmm4
;;       movsd   0x1fe(%rip), %xmm5
;;       movabsq $9223372036854775808, %r11
;;       movq    %r11, %xmm15
;;       xorpd   %xmm15, %xmm5
;;       movsd   0x1e2(%rip), %xmm6
;;       movabsq $9223372036854775808, %r11
;;       movq    %r11, %xmm15
;;       xorpd   %xmm15, %xmm6
;;       movsd   0x1c6(%rip), %xmm7
;;       movabsq $9223372036854775808, %r11
;;       movq    %r11, %xmm15
;;       xorpd   %xmm15, %xmm7
;;       movsd   0x1a9(%rip), %xmm8
;;       movabsq $9223372036854775808, %r11
;;       movq    %r11, %xmm15
;;       xorpd   %xmm15, %xmm8
;;       movsd   0x18c(%rip), %xmm9
;;       movabsq $9223372036854775808, %r11
;;       movq    %r11, %xmm15
;;       xorpd   %xmm15, %xmm9
;;       movsd   0x16f(%rip), %xmm10
;;       movabsq $9223372036854775808, %r11
;;       movq    %r11, %xmm15
;;       xorpd   %xmm15, %xmm10
;;       movsd   0x152(%rip), %xmm11
;;       movabsq $9223372036854775808, %r11
;;       movq    %r11, %xmm15
;;       xorpd   %xmm15, %xmm11
;;       movsd   0x135(%rip), %xmm12
;;       movabsq $9223372036854775808, %r11
;;       movq    %r11, %xmm15
;;       xorpd   %xmm15, %xmm12
;;       movsd   0x118(%rip), %xmm13
;;       movabsq $9223372036854775808, %r11
;;       movq    %r11, %xmm15
;;       xorpd   %xmm15, %xmm13
;;       movl    $0, %eax
;;       movdqu  0xfe(%rip), %xmm14
;;       andl    $7, %eax
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
;;       movsd   0x78(%rsp), %xmm15
;;       subq    $8, %rsp
;;       movsd   %xmm15, (%rsp)
;;       addl    $8, %eax
;;       vmovd   %eax, %xmm15
;;       vpunpcklbw %xmm14, %xmm14, %xmm0
;;       vpunpckhbw %xmm14, %xmm14, %xmm1
;;       vpsraw  %xmm15, %xmm0, %xmm0
;;       vpsraw  %xmm15, %xmm1, %xmm1
;;       vpacksswb %xmm1, %xmm0, %xmm14
;;       ud2
;;       addq    $0x20, %rsp
;;       popq    %rbp
;;       retq
;;  2c5: ud2
;;  2c7: addb    %al, (%rax)
;;  2c9: addb    %al, (%rax)
;;  2cb: addb    %al, (%rax)
;;  2cd: addb    %al, (%rax)
;;  2cf: addb    %al, (%rax)
;;  2d1: addb    %al, (%rax)
;;  2d3: addb    %al, (%rax)
;;  2d5: addb    %al, (%rax)
;;  2d7: addb    %al, (%rax)
;;  2d9: addb    %al, (%rax)
;;  2db: addb    %al, (%rax)
;;  2dd: addb    %al, (%rax)
