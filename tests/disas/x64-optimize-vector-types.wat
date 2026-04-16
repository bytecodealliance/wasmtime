;;! target = "x86_64"
;;! test = "compile"

(module
  (memory 1)

  (func (export "i32x4_sge_smax") (param v128 v128) (result v128)
    local.get 0
    local.get 1
    i32x4.max_s
    local.get 0
    i32x4.ge_s
  )

  (func (export "i32x4_sle_smax") (param v128 v128) (result v128)
    local.get 0
    local.get 0
    local.get 1
    i32x4.max_s
    i32x4.le_s
  )

  (func (export "i32x4_sge_smin") (param v128 v128) (result v128)
    local.get 0
    local.get 0
    local.get 1
    i32x4.min_s
    i32x4.ge_s
  )

  (func (export "i32x4_sle_smin") (param v128 v128) (result v128)
    local.get 0
    local.get 1
    i32x4.min_s
    local.get 0
    i32x4.le_s
  )

  (func (export "i16x8_uge_umax") (param v128 v128) (result v128)
    local.get 0
    local.get 1
    i16x8.max_u
    local.get 0
    i16x8.ge_u
  )

  (func (export "i8x16_ule_umin") (param v128 v128) (result v128)
    local.get 0
    local.get 1
    i8x16.min_u
    local.get 0
    i8x16.le_u
  )

  (func (export "i32x4_sgt_smax") (param v128 v128) (result v128)
    local.get 0
    local.get 0
    local.get 1
    i32x4.max_s
    i32x4.gt_s
  )

  (func (export "i64x2_ugt_umax") (param v128 v128) (result v128)
    local.get 0
    local.get 0
    local.get 1
    i32x4.max_u
    i32x4.gt_u
  )

  (func (export "v128_band_bnot_eq") (param v128 v128) (result v128)
    local.get 0
    local.get 1
    v128.not
    v128.and
    local.get 1
    v128.not
    i8x16.eq
  )

  (func (export "ne_isub_i32x4") (param v128 v128) (result v128)
    local.get 0
    local.get 1
    i32x4.sub
    local.get 0
    i32x4.ne
  )

  (func (export "eq_isub_i32x4") (param v128 v128) (result v128)
    local.get 0
    local.get 1
    i32x4.sub
    local.get 0
    i32x4.eq
  )

  (func (export "eq_iadd_i32x4") (param v128 v128) (result v128)
    local.get 0
    local.get 1
    i32x4.add
    local.get 1
    i32x4.eq
  )

  (func (export "ne_isub_i16x8") (param v128 v128) (result v128)
    local.get 0
    local.get 1
    i16x8.sub
    local.get 0
    i16x8.ne
  )

  (func (export "eq_iadd_i8x16") (param v128 v128) (result v128)
    local.get 0
    local.get 1
    i8x16.add
    local.get 1
    i8x16.eq
  )

  (func (export "eq_isub_rev") (param v128 v128) (result v128)
    local.get 0
    local.get 0
    local.get 1
    i32x4.sub
    i32x4.eq
  )

  (func (export "eq_iadd_rev") (param v128 v128) (result v128)
    local.get 1
    local.get 0
    local.get 1
    i32x4.add
    i32x4.eq
  )
)
;; wasm[0]::function[0]:
;;       pushq   %rbp
;;       movq    %rsp, %rbp
;;       pcmpeqd %xmm0, %xmm0
;;       movq    %rbp, %rsp
;;       popq    %rbp
;;       retq
;;
;; wasm[0]::function[1]:
;;       pushq   %rbp
;;       movq    %rsp, %rbp
;;       pcmpeqd %xmm0, %xmm0
;;       movq    %rbp, %rsp
;;       popq    %rbp
;;       retq
;;
;; wasm[0]::function[2]:
;;       pushq   %rbp
;;       movq    %rsp, %rbp
;;       pcmpeqd %xmm0, %xmm0
;;       movq    %rbp, %rsp
;;       popq    %rbp
;;       retq
;;
;; wasm[0]::function[3]:
;;       pushq   %rbp
;;       movq    %rsp, %rbp
;;       pcmpeqd %xmm0, %xmm0
;;       movq    %rbp, %rsp
;;       popq    %rbp
;;       retq
;;
;; wasm[0]::function[4]:
;;       pushq   %rbp
;;       movq    %rsp, %rbp
;;       pcmpeqd %xmm0, %xmm0
;;       movq    %rbp, %rsp
;;       popq    %rbp
;;       retq
;;
;; wasm[0]::function[5]:
;;       pushq   %rbp
;;       movq    %rsp, %rbp
;;       pcmpeqd %xmm0, %xmm0
;;       movq    %rbp, %rsp
;;       popq    %rbp
;;       retq
;;
;; wasm[0]::function[6]:
;;       pushq   %rbp
;;       movq    %rsp, %rbp
;;       pxor    %xmm0, %xmm0
;;       movq    %rbp, %rsp
;;       popq    %rbp
;;       retq
;;
;; wasm[0]::function[7]:
;;       pushq   %rbp
;;       movq    %rsp, %rbp
;;       pxor    %xmm0, %xmm0
;;       movq    %rbp, %rsp
;;       popq    %rbp
;;       retq
;;
;; wasm[0]::function[8]:
;;       pushq   %rbp
;;       movq    %rsp, %rbp
;;       por     %xmm0, %xmm1
;;       pcmpeqd %xmm7, %xmm7
;;       movdqa  %xmm1, %xmm0
;;       pcmpeqb %xmm7, %xmm0
;;       movq    %rbp, %rsp
;;       popq    %rbp
;;       retq
;;
;; wasm[0]::function[9]:
;;       pushq   %rbp
;;       movq    %rsp, %rbp
;;       pxor    %xmm0, %xmm0
;;       pcmpeqd %xmm0, %xmm1
;;       pcmpeqd %xmm2, %xmm2
;;       movdqa  %xmm1, %xmm0
;;       pxor    %xmm2, %xmm0
;;       movq    %rbp, %rsp
;;       popq    %rbp
;;       retq
;;
;; wasm[0]::function[10]:
;;       pushq   %rbp
;;       movq    %rsp, %rbp
;;       pxor    %xmm5, %xmm5
;;       movdqa  %xmm1, %xmm0
;;       pcmpeqd %xmm5, %xmm0
;;       movq    %rbp, %rsp
;;       popq    %rbp
;;       retq
;;
;; wasm[0]::function[11]:
;;       pushq   %rbp
;;       movq    %rsp, %rbp
;;       pxor    %xmm5, %xmm5
;;       pcmpeqd %xmm5, %xmm0
;;       movq    %rbp, %rsp
;;       popq    %rbp
;;       retq
;;
;; wasm[0]::function[12]:
;;       pushq   %rbp
;;       movq    %rsp, %rbp
;;       pxor    %xmm0, %xmm0
;;       pcmpeqw %xmm0, %xmm1
;;       pcmpeqd %xmm2, %xmm2
;;       movdqa  %xmm1, %xmm0
;;       pxor    %xmm2, %xmm0
;;       movq    %rbp, %rsp
;;       popq    %rbp
;;       retq
;;
;; wasm[0]::function[13]:
;;       pushq   %rbp
;;       movq    %rsp, %rbp
;;       paddb   %xmm1, %xmm0
;;       pcmpeqb %xmm1, %xmm0
;;       movq    %rbp, %rsp
;;       popq    %rbp
;;       retq
;;
;; wasm[0]::function[14]:
;;       pushq   %rbp
;;       movq    %rsp, %rbp
;;       pxor    %xmm5, %xmm5
;;       movdqa  %xmm1, %xmm0
;;       pcmpeqd %xmm5, %xmm0
;;       movq    %rbp, %rsp
;;       popq    %rbp
;;       retq
;;
;; wasm[0]::function[15]:
;;       pushq   %rbp
;;       movq    %rsp, %rbp
;;       pxor    %xmm5, %xmm5
;;       pcmpeqd %xmm5, %xmm0
;;       movq    %rbp, %rsp
;;       popq    %rbp
;;       retq
