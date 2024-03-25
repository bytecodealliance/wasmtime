;;! target = "x86_64"
;;! test = "compile"
;;! flags = "-Ccranelift-sse42 -Ccranelift-has-avx -Wrelaxed-simd-deterministic"

(module
  (func (param v128) (result v128)
    local.get 0
    i32x4.relaxed_trunc_f32x4_s
  )

  (func (param v128) (result v128)
    local.get 0
    i32x4.relaxed_trunc_f32x4_u
  )

  (func (param v128) (result v128)
    local.get 0
    i32x4.relaxed_trunc_f64x2_s_zero
  )

  (func (param v128) (result v128)
    local.get 0
    i32x4.relaxed_trunc_f64x2_u_zero
  )

  (func (param v128 v128) (result v128)
    local.get 0
    local.get 1
    i16x8.relaxed_dot_i8x16_i7x16_s
  )

  (func (param v128 v128 v128) (result v128)
    local.get 0
    local.get 1
    local.get 2
    i32x4.relaxed_dot_i8x16_i7x16_add_s
  )
)

;; wasm[0]::function[0]:
;;    0: pushq   %rbp
;;    1: movq    %rsp, %rbp
;;    4: vcmpeqps %xmm0, %xmm0, %xmm5
;;    9: vandps  %xmm5, %xmm0, %xmm7
;;    d: vpxor   %xmm7, %xmm5, %xmm1
;;   11: vcvttps2dq %xmm7, %xmm3
;;   15: vpand   %xmm1, %xmm3, %xmm5
;;   19: vpsrad  $0x1f, %xmm5, %xmm7
;;   1e: vpxor   %xmm3, %xmm7, %xmm0
;;   22: movq    %rbp, %rsp
;;   25: popq    %rbp
;;   26: retq
;;
;; wasm[0]::function[1]:
;;   30: pushq   %rbp
;;   31: movq    %rsp, %rbp
;;   34: vxorps  %xmm5, %xmm5, %xmm7
;;   38: vmaxps  %xmm7, %xmm0, %xmm2
;;   3c: vpcmpeqd %xmm7, %xmm7, %xmm3
;;   40: vpsrld  $1, %xmm3, %xmm5
;;   45: vcvtdq2ps %xmm5, %xmm7
;;   49: vcvttps2dq %xmm2, %xmm1
;;   4d: vsubps  %xmm7, %xmm2, %xmm3
;;   51: vcmpleps %xmm3, %xmm7, %xmm5
;;   56: vcvttps2dq %xmm3, %xmm7
;;   5a: vpxor   %xmm5, %xmm7, %xmm2
;;   5e: vpxor   %xmm3, %xmm3, %xmm5
;;   62: vpmaxsd %xmm5, %xmm2, %xmm7
;;   67: vpaddd  %xmm1, %xmm7, %xmm0
;;   6b: movq    %rbp, %rsp
;;   6e: popq    %rbp
;;   6f: retq
;;
;; wasm[0]::function[2]:
;;   70: pushq   %rbp
;;   71: movq    %rsp, %rbp
;;   74: vcmpeqpd %xmm0, %xmm0, %xmm5
;;   79: vandps  0xf(%rip), %xmm5, %xmm7
;;   81: vminpd  %xmm7, %xmm0, %xmm1
;;   85: vcvttpd2dq %xmm1, %xmm0
;;   89: movq    %rbp, %rsp
;;   8c: popq    %rbp
;;   8d: retq
;;   8e: addb    %al, (%rax)
;;   90: addb    %al, (%rax)
;;   92: sarb    $0xff, %bh
;;
;; wasm[0]::function[3]:
;;   a0: pushq   %rbp
;;   a1: movq    %rsp, %rbp
;;   a4: vxorpd  %xmm5, %xmm5, %xmm7
;;   a8: vmaxpd  %xmm7, %xmm0, %xmm1
;;   ac: vminpd  0x1c(%rip), %xmm1, %xmm3
;;   b4: vroundpd $3, %xmm3, %xmm5
;;   ba: vaddpd  0x1e(%rip), %xmm5, %xmm0
;;   c2: vshufps $0x88, %xmm7, %xmm0, %xmm0
;;   c7: movq    %rbp, %rsp
;;   ca: popq    %rbp
;;   cb: retq
;;   cc: addb    %al, (%rax)
;;   ce: addb    %al, (%rax)
;;   d0: addb    %al, (%rax)
;;   d2: loopne  0xd3
;;
;; wasm[0]::function[4]:
;;   f0: pushq   %rbp
;;   f1: movq    %rsp, %rbp
;;   f4: vpmovsxbw %xmm0, %xmm6
;;   f9: vpmovsxbw %xmm1, %xmm7
;;   fe: vpmullw %xmm7, %xmm6, %xmm6
;;  102: vpalignr $8, %xmm0, %xmm0, %xmm5
;;  108: vpmovsxbw %xmm5, %xmm7
;;  10d: vpalignr $8, %xmm1, %xmm1, %xmm5
;;  113: vpmovsxbw %xmm5, %xmm0
;;  118: vpmullw %xmm0, %xmm7, %xmm7
;;  11c: vphaddw %xmm7, %xmm6, %xmm0
;;  121: movq    %rbp, %rsp
;;  124: popq    %rbp
;;  125: retq
;;
;; wasm[0]::function[5]:
;;  130: pushq   %rbp
;;  131: movq    %rsp, %rbp
;;  134: vpmovsxbw %xmm0, %xmm3
;;  139: vpmovsxbw %xmm1, %xmm4
;;  13e: vpmullw %xmm4, %xmm3, %xmm3
;;  142: vpalignr $8, %xmm0, %xmm0, %xmm0
;;  148: vpmovsxbw %xmm0, %xmm4
;;  14d: vpalignr $8, %xmm1, %xmm1, %xmm0
;;  153: vpmovsxbw %xmm0, %xmm5
;;  158: vpmullw %xmm5, %xmm4, %xmm1
;;  15c: vphaddw %xmm1, %xmm3, %xmm1
;;  161: vpmaddwd 0x17(%rip), %xmm1, %xmm1
;;  169: vpaddd  %xmm2, %xmm1, %xmm0
;;  16d: movq    %rbp, %rsp
;;  170: popq    %rbp
;;  171: retq
;;  172: addb    %al, (%rax)
;;  174: addb    %al, (%rax)
;;  176: addb    %al, (%rax)
;;  178: addb    %al, (%rax)
;;  17a: addb    %al, (%rax)
;;  17c: addb    %al, (%rax)
;;  17e: addb    %al, (%rax)
;;  180: addl    %eax, (%rax)
;;  182: addl    %eax, (%rax)
;;  184: addl    %eax, (%rax)
;;  186: addl    %eax, (%rax)
;;  188: addl    %eax, (%rax)
;;  18a: addl    %eax, (%rax)
;;  18c: addl    %eax, (%rax)
;;  18e: addl    %eax, (%rax)
