;;! target = "x86_64"
;;! flags = "-W function-references,gc"
;;! test = "optimize"

(module
  (type $ty (array (mut i64)))

  (func (param (ref $ty) i32 i32) (result i64 i64)
    (array.get $ty (local.get 0) (local.get 1))
    (array.get $ty (local.get 0) (local.get 2))
  )
)
;; function u0:0(i64 vmctx, i64, i32, i32, i32) -> i64, i64 tail {
;;     gv0 = vmctx
;;     gv1 = load.i64 notrap aligned readonly gv0+8
;;     gv2 = load.i64 notrap aligned gv1
;;     gv3 = vmctx
;;     stack_limit = gv2
;;
;;                                 block0(v0: i64, v1: i64, v2: i32, v3: i32, v4: i32):
;; @0024                               trapz v2, user16
;; @0024                               v10 = uextend.i64 v2
;; @0024                               v11 = iconst.i64 16
;; @0024                               v12 = uadd_overflow_trap v10, v11, user1  ; v11 = 16
;; @0024                               v13 = iconst.i64 4
;; @0024                               v14 = uadd_overflow_trap v12, v13, user1  ; v13 = 4
;; @0024                               v9 = load.i64 notrap aligned readonly v0+48
;; @0024                               v15 = icmp ule v14, v9
;; @0024                               trapz v15, user1
;; @0024                               v8 = load.i64 notrap aligned readonly v0+40
;; @0024                               v16 = iadd v8, v12
;; @0024                               v17 = load.i32 notrap aligned v16
;; @0024                               v18 = icmp ult v3, v17
;; @0024                               trapz v18, user17
;; @0024                               v20 = uextend.i64 v17
;;                                     v75 = iconst.i64 3
;;                                     v76 = ishl v20, v75  ; v75 = 3
;;                                     v73 = iconst.i64 32
;; @0024                               v22 = ushr v76, v73  ; v73 = 32
;; @0024                               trapnz v22, user1
;;                                     v85 = iconst.i32 3
;;                                     v86 = ishl v17, v85  ; v85 = 3
;; @0024                               v24 = iconst.i32 24
;; @0024                               v25 = uadd_overflow_trap v86, v24, user1  ; v24 = 24
;;                                     v93 = ishl v3, v85  ; v85 = 3
;; @0024                               v28 = iadd v93, v24  ; v24 = 24
;; @0024                               v33 = uextend.i64 v28
;; @0024                               v34 = uadd_overflow_trap v10, v33, user1
;; @0024                               v35 = uextend.i64 v25
;; @0024                               v36 = uadd_overflow_trap v10, v35, user1
;; @0024                               v37 = icmp ule v36, v9
;; @0024                               trapz v37, user1
;; @0024                               v38 = iadd v8, v34
;; @0024                               v39 = load.i64 notrap aligned little v38
;; @002b                               trapz v2, user16
;; @002b                               trapz v15, user1
;; @002b                               v50 = load.i32 notrap aligned v16
;; @002b                               v51 = icmp ult v4, v50
;; @002b                               trapz v51, user17
;; @002b                               v53 = uextend.i64 v50
;;                                     v95 = ishl v53, v75  ; v75 = 3
;; @002b                               v55 = ushr v95, v73  ; v73 = 32
;; @002b                               trapnz v55, user1
;;                                     v102 = ishl v50, v85  ; v85 = 3
;; @002b                               v58 = uadd_overflow_trap v102, v24, user1  ; v24 = 24
;;                                     v109 = ishl v4, v85  ; v85 = 3
;; @002b                               v61 = iadd v109, v24  ; v24 = 24
;; @002b                               v66 = uextend.i64 v61
;; @002b                               v67 = uadd_overflow_trap v10, v66, user1
;; @002b                               v68 = uextend.i64 v58
;; @002b                               v69 = uadd_overflow_trap v10, v68, user1
;; @002b                               v70 = icmp ule v69, v9
;; @002b                               trapz v70, user1
;; @002b                               v71 = iadd v8, v67
;; @002b                               v72 = load.i64 notrap aligned little v71
;; @002e                               jump block1
;;
;;                                 block1:
;; @002e                               return v39, v72
;; }
