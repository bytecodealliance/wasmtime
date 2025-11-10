;;! target = "x86_64"
;;! test = "winch"
;;! flags = [ "-Ccranelift-has-avx" ]
(module
  (type (;0;) (func (param v128 i64)))
  (table (;0;) 0 265945 funcref)
  (global (;0;) (mut f32) f32.const -0x1.4f4f4ep-48 (;=-0.000000000000004653358;))
  (global (;1;) (mut f32) f32.const -0x1.cbcb4ep+76 (;=-135707280000000000000000;))
  (global (;2;) (mut v128) v128.const i32x4 0xff500177 0x01bbffff 0x5e010150 0x3b3b0177)
  (func (;0;) (type 0) (param v128 i64)
    local.get 0
    local.get 1
    global.get 1
    global.get 0
    global.get 1
    global.get 1
    global.get 1
    global.get 1
    global.get 0
    global.get 1
    global.get 1
    global.get 0
    global.get 1
    global.get 1
    local.get 0
    i8x16.popcnt
    global.get 0
    global.get 2
    i8x16.popcnt
    unreachable
  )
)
;; wasm[0]::function[0]:
;;       pushq   %rbp
;;       movq    %rsp, %rbp
;;       movq    8(%rdi), %r11
;;       movq    0x10(%r11), %r11
;;       addq    $0x8c, %r11
;;       cmpq    %rsp, %r11
;;       ja      0x1c5
;;   1c: movq    %rdi, %r14
;;       subq    $0x30, %rsp
;;       movq    %rdi, 0x28(%rsp)
;;       movq    %rsi, 0x20(%rsp)
;;       movdqu  %xmm0, 0x10(%rsp)
;;       movq    %rdx, 8(%rsp)
;;       movss   0x50(%r14), %xmm0
;;       movss   0x40(%r14), %xmm1
;;       movss   0x50(%r14), %xmm2
;;       movss   0x50(%r14), %xmm3
;;       movss   0x50(%r14), %xmm4
;;       movss   0x50(%r14), %xmm5
;;       movss   0x40(%r14), %xmm6
;;       movss   0x50(%r14), %xmm7
;;       movss   0x50(%r14), %xmm8
;;       movss   0x40(%r14), %xmm9
;;       movss   0x50(%r14), %xmm10
;;       movss   0x50(%r14), %xmm11
;;       movdqu  0x10(%rsp), %xmm12
;;       vpand   0x13e(%rip), %xmm12, %xmm15
;;       vpsrlw  $4, %xmm12, %xmm12
;;       vpand   0x130(%rip), %xmm12, %xmm12
;;       movdqu  0x137(%rip), %xmm13
;;       vpshufb %xmm12, %xmm13, %xmm12
;;       vpshufb %xmm15, %xmm13, %xmm15
;;       vpaddb  %xmm15, %xmm12, %xmm12
;;       movss   0x40(%r14), %xmm13
;;       movdqu  0x60(%r14), %xmm14
;;       movdqu  0x10(%rsp), %xmm15
;;       subq    $0x10, %rsp
;;       movdqu  %xmm15, (%rsp)
;;       movq    0x18(%rsp), %r11
;;       pushq   %r11
;;       subq    $4, %rsp
;;       movss   %xmm0, (%rsp)
;;       subq    $4, %rsp
;;       movss   %xmm1, (%rsp)
;;       subq    $4, %rsp
;;       movss   %xmm2, (%rsp)
;;       subq    $4, %rsp
;;       movss   %xmm3, (%rsp)
;;       subq    $4, %rsp
;;       movss   %xmm4, (%rsp)
;;       subq    $4, %rsp
;;       movss   %xmm5, (%rsp)
;;       subq    $4, %rsp
;;       movss   %xmm6, (%rsp)
;;       subq    $4, %rsp
;;       movss   %xmm7, (%rsp)
;;       subq    $4, %rsp
;;       movss   %xmm8, (%rsp)
;;       subq    $4, %rsp
;;       movss   %xmm9, (%rsp)
;;       subq    $4, %rsp
;;       movss   %xmm10, (%rsp)
;;       subq    $4, %rsp
;;       movss   %xmm11, (%rsp)
;;       subq    $0x10, %rsp
;;       movdqu  %xmm12, (%rsp)
;;       subq    $4, %rsp
;;       movss   %xmm13, (%rsp)
;;       vpand   0x3b(%rip), %xmm14, %xmm15
;;       vpsrlw  $4, %xmm14, %xmm14
;;       vpand   0x2d(%rip), %xmm14, %xmm14
;;       movdqu  0x35(%rip), %xmm0
;;       vpshufb %xmm14, %xmm0, %xmm14
;;       vpshufb %xmm15, %xmm0, %xmm15
;;       vpaddb  %xmm15, %xmm14, %xmm14
;;       ud2
;;       addq    $0x30, %rsp
;;       popq    %rbp
;;       retq
;;  1c5: ud2
;;  1c7: addb    %al, (%rax)
;;  1c9: addb    %al, (%rax)
;;  1cb: addb    %al, (%rax)
;;  1cd: addb    %al, (%rax)
;;  1cf: addb    %cl, (%rdi)
