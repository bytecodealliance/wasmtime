;;! target = "x86_64"
;;! flags = "-W function-references,gc -C collector=null"
;;! test = "optimize"

(module
  (type $ty (struct (field (mut f32))
                    (field (mut i8))
                    (field (mut anyref))))

  (func (param f32 i32 anyref) (result (ref $ty))
    (struct.new $ty (local.get 0) (local.get 1) (local.get 2))
  )
)
;; function u0:0(i64 vmctx, i64, f32, i32, i32) -> i32 tail {
;;     gv0 = vmctx
;;     gv1 = load.i64 notrap aligned readonly gv0+8
;;     gv2 = load.i64 notrap aligned gv1
;;     gv3 = vmctx
;;     stack_limit = gv2
;;
;;                                 block0(v0: i64, v1: i64, v2: f32, v3: i32, v4: i32):
;;                                     v38 = iconst.i32 0
;; @002a                               trapnz v38, user18  ; v38 = 0
;; @002a                               v11 = load.i64 notrap aligned readonly v0+56
;; @002a                               v12 = load.i32 notrap aligned v11
;;                                     v45 = iconst.i32 7
;; @002a                               v15 = uadd_overflow_trap v12, v45, user18  ; v45 = 7
;;                                     v52 = iconst.i32 -8
;; @002a                               v17 = band v15, v52  ; v52 = -8
;; @002a                               v6 = iconst.i32 24
;; @002a                               v18 = uadd_overflow_trap v17, v6, user18  ; v6 = 24
;; @002a                               v19 = uextend.i64 v18
;; @002a                               v23 = load.i64 notrap aligned readonly v0+48
;; @002a                               v24 = icmp ule v19, v23
;; @002a                               trapz v24, user18
;;                                     v53 = iconst.i32 -1342177256
;; @002a                               v21 = load.i64 notrap aligned readonly v0+40
;; @002a                               v25 = uextend.i64 v17
;; @002a                               v26 = iadd v21, v25
;; @002a                               store notrap aligned v53, v26  ; v53 = -1342177256
;; @002a                               v30 = load.i64 notrap aligned readonly v0+80
;; @002a                               v31 = load.i32 notrap aligned readonly v30
;; @002a                               store notrap aligned v31, v26+4
;; @002a                               store notrap aligned v18, v11
;;                                     v35 = iconst.i64 8
;; @002a                               v32 = iadd v26, v35  ; v35 = 8
;; @002a                               store notrap aligned little v2, v32
;;                                     v36 = iconst.i64 12
;; @002a                               v33 = iadd v26, v36  ; v36 = 12
;; @002a                               istore8 notrap aligned little v3, v33
;;                                     v37 = iconst.i64 16
;; @002a                               v34 = iadd v26, v37  ; v37 = 16
;; @002a                               store notrap aligned little v4, v34
;; @002d                               jump block1
;;
;;                                 block1:
;;                                     v60 = band.i32 v15, v52  ; v52 = -8
;; @002d                               return v60
;; }
