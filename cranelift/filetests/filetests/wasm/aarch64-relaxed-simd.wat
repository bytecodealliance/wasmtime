;;! target = "aarch64"
;;! compile = true

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

;; function u0:0:
;; block0:
;;   fcvtzs v0.4s, v0.4s
;;   b label1
;; block1:
;;   ret
;;
;; function u0:1:
;; block0:
;;   fcvtzu v0.4s, v0.4s
;;   b label1
;; block1:
;;   ret
;;
;; function u0:2:
;; block0:
;;   fcvtzs v4.2d, v0.2d
;;   sqxtn v0.2s, v4.2d
;;   b label1
;; block1:
;;   ret
;;
;; function u0:3:
;; block0:
;;   fcvtzu v4.2d, v0.2d
;;   uqxtn v0.2s, v4.2d
;;   b label1
;; block1:
;;   ret
;;
;; function u0:4:
;; block0:
;;   smull v6.8h, v0.8b, v1.8b
;;   smull2 v7.8h, v0.16b, v1.16b
;;   addp v0.8h, v6.8h, v7.8h
;;   b label1
;; block1:
;;   ret
;;
;; function u0:5:
;; block0:
;;   smull v17.8h, v0.8b, v1.8b
;;   smull2 v18.8h, v0.16b, v1.16b
;;   addp v17.8h, v17.8h, v18.8h
;;   saddlp v17.4s, v17.8h
;;   add v0.4s, v17.4s, v2.4s
;;   b label1
;; block1:
;;   ret
