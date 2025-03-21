;;! target = "x86_64"
;;! flags = "-W function-references,gc"
;;! test = "optimize"

(module
  (type $ty (struct (field (mut f32))
                    (field (mut i8))
                    (field (mut anyref))
                    (field (mut v128))))

  (func (result (ref $ty))
    (struct.new_default $ty)
  )
)
;; function u0:0(i64 vmctx, i64) -> i32 tail {
;;     gv0 = vmctx
;;     gv1 = load.i64 notrap aligned readonly gv0+8
;;     gv2 = load.i64 notrap aligned gv1+16
;;     gv3 = vmctx
;;     sig0 = (i64 vmctx, i32, i32, i32, i32) -> i32 tail
;;     fn0 = colocated u1:27 sig0
;;     const0 = 0x00000000000000000000000000000000
;;     stack_limit = gv2
;;
;;                                 block0(v0: i64, v1: i64):
;;                                     v58 = iconst.i32 -1342177279
;; @0023                               v4 = iconst.i32 0
;; @0023                               v7 = iconst.i32 64
;; @0023                               v13 = iconst.i32 16
;; @0023                               v14 = call fn0(v0, v58, v4, v7, v13)  ; v58 = -1342177279, v4 = 0, v7 = 64, v13 = 16
;; @0023                               v3 = f32const 0.0
;; @0023                               v16 = load.i64 notrap aligned readonly can_move v0+40
;; @0023                               v17 = uextend.i64 v14
;; @0023                               v18 = iadd v16, v17
;;                                     v51 = iconst.i64 48
;; @0023                               v19 = iadd v18, v51  ; v51 = 48
;; @0023                               store notrap aligned little v3, v19  ; v3 = 0.0
;;                                     v52 = iconst.i64 52
;; @0023                               v20 = iadd v18, v52  ; v52 = 52
;; @0023                               istore8 notrap aligned little v4, v20  ; v4 = 0
;; @0023                               v8 = iconst.i32 1
;; @0023                               brif v8, block3, block2  ; v8 = 1
;;
;;                                 block2:
;;                                     v84 = iconst.i64 0
;; @0023                               v31 = iconst.i64 8
;; @0023                               v32 = uadd_overflow_trap v84, v31, user1  ; v84 = 0, v31 = 8
;; @0023                               v34 = uadd_overflow_trap v32, v31, user1  ; v31 = 8
;; @0023                               v29 = load.i64 notrap aligned readonly can_move v0+48
;; @0023                               v35 = icmp ule v34, v29
;; @0023                               trapz v35, user1
;; @0023                               v36 = iadd.i64 v16, v32
;; @0023                               v37 = load.i64 notrap aligned v36
;;                                     v56 = iconst.i64 1
;; @0023                               v38 = iadd v37, v56  ; v56 = 1
;; @0023                               store notrap aligned v38, v36
;; @0023                               jump block3
;;
;;                                 block3:
;;                                     v85 = iconst.i32 0
;;                                     v53 = iconst.i64 24
;; @0023                               v21 = iadd.i64 v18, v53  ; v53 = 24
;; @0023                               store notrap aligned little v85, v21  ; v85 = 0
;; @0023                               v6 = vconst.i8x16 const0
;;                                     v57 = iconst.i64 32
;; @0023                               v50 = iadd.i64 v18, v57  ; v57 = 32
;; @0023                               store notrap aligned little v6, v50  ; v6 = const0
;; @0026                               jump block1
;;
;;                                 block1:
;; @0026                               return v14
;; }
