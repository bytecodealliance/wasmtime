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
;;       ja      0x2b7
;;   1c: movq    %rdi, %r14
;;       subq    $0x20, %rsp
;;       movq    %rdi, 0x18(%rsp)
;;       movq    %rsi, 0x10(%rsp)
;;       movq    %rdx, 8(%rsp)
;;       movsd   0x283(%rip), %xmm0
;;       movabsq $9223372036854775808, %r11
;;       movq    %r11, %xmm15
;;       xorpd   %xmm15, %xmm0
;;       movsd   0x267(%rip), %xmm1
;;       movabsq $9223372036854775808, %r11
;;       movq    %r11, %xmm15
;;       xorpd   %xmm15, %xmm1
;;       movsd   0x24b(%rip), %xmm2
;;       movabsq $9223372036854775808, %r11
;;       movq    %r11, %xmm15
;;       xorpd   %xmm15, %xmm2
;;       movsd   0x22f(%rip), %xmm3
;;       movabsq $9223372036854775808, %r11
;;       movq    %r11, %xmm15
;;       xorpd   %xmm15, %xmm3
;;       movsd   0x213(%rip), %xmm4
;;       movabsq $9223372036854775808, %r11
;;       movq    %r11, %xmm15
;;       xorpd   %xmm15, %xmm4
;;       movsd   0x1f7(%rip), %xmm5
;;       movabsq $9223372036854775808, %r11
;;       movq    %r11, %xmm15
;;       xorpd   %xmm15, %xmm5
;;       movsd   0x1db(%rip), %xmm6
;;       movabsq $9223372036854775808, %r11
;;       movq    %r11, %xmm15
;;       xorpd   %xmm15, %xmm6
;;       movsd   0x1bf(%rip), %xmm7
;;       movabsq $9223372036854775808, %r11
;;       movq    %r11, %xmm15
;;       xorpd   %xmm15, %xmm7
;;       movsd   0x1a2(%rip), %xmm8
;;       movabsq $9223372036854775808, %r11
;;       movq    %r11, %xmm15
;;       xorpd   %xmm15, %xmm8
;;       movsd   0x185(%rip), %xmm9
;;       movabsq $9223372036854775808, %r11
;;       movq    %r11, %xmm15
;;       xorpd   %xmm15, %xmm9
;;       movsd   0x168(%rip), %xmm10
;;       movabsq $9223372036854775808, %r11
;;       movq    %r11, %xmm15
;;       xorpd   %xmm15, %xmm10
;;       movsd   0x14b(%rip), %xmm11
;;       movabsq $9223372036854775808, %r11
;;       movq    %r11, %xmm15
;;       xorpd   %xmm15, %xmm11
;;       movsd   0x12e(%rip), %xmm12
;;       movabsq $9223372036854775808, %r11
;;       movq    %r11, %xmm15
;;       xorpd   %xmm15, %xmm12
;;       movsd   0x111(%rip), %xmm13
;;       movabsq $9223372036854775808, %r11
;;       movq    %r11, %xmm15
;;       xorpd   %xmm15, %xmm13
;;       movl    $0, %eax
;;       movdqu  0xff(%rip), %xmm14
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
;;       movq    0x78(%rsp), %r11
;;       pushq   %r11
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
;;  2b7: ud2
;;  2b9: addb    %al, (%rax)
;;  2bb: addb    %al, (%rax)
;;  2bd: addb    %al, (%rax)
;;  2bf: addb    %al, (%rax)
;;  2c1: addb    %al, (%rax)
;;  2c3: addb    %al, (%rax)
;;  2c5: addb    %al, (%rax)
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
