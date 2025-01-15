;;! target = "x86_64"
;;! flags = "-W function-references,gc -C collector=null"
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
;;     gv2 = load.i64 notrap aligned gv1+16
;;     gv3 = vmctx
;;     stack_limit = gv2
;;
;;                                 block0(v0: i64, v1: i64, v2: i32, v3: i32, v4: i32):
;; @0024                               trapz v2, user16
;; @0024                               v11 = uextend.i64 v2
;; @0024                               v12 = iconst.i64 8
;; @0024                               v13 = uadd_overflow_trap v11, v12, user1  ; v12 = 8
;; @0024                               v14 = iconst.i64 4
;; @0024                               v15 = uadd_overflow_trap v13, v14, user1  ; v14 = 4
;; @0024                               v10 = load.i64 notrap aligned readonly v0+48
;; @0024                               v16 = icmp ule v15, v10
;; @0024                               trapz v16, user1
;; @0024                               v8 = load.i64 notrap aligned readonly v0+40
;; @0024                               v17 = iadd v8, v13
;; @0024                               v18 = load.i32 notrap aligned v17
;; @0024                               v19 = icmp ult v3, v18
;; @0024                               trapz v19, user17
;; @0024                               v21 = uextend.i64 v18
;;                                     v79 = iconst.i64 3
;;                                     v80 = ishl v21, v79  ; v79 = 3
;;                                     v77 = iconst.i64 32
;; @0024                               v23 = ushr v80, v77  ; v77 = 32
;; @0024                               trapnz v23, user1
;;                                     v89 = iconst.i32 3
;;                                     v90 = ishl v18, v89  ; v89 = 3
;; @0024                               v25 = iconst.i32 16
;; @0024                               v26 = uadd_overflow_trap v90, v25, user1  ; v25 = 16
;;                                     v97 = ishl v3, v89  ; v89 = 3
;; @0024                               v29 = iadd v97, v25  ; v25 = 16
;; @0024                               v35 = uextend.i64 v29
;; @0024                               v36 = uadd_overflow_trap v11, v35, user1
;; @0024                               v37 = uextend.i64 v26
;; @0024                               v38 = uadd_overflow_trap v11, v37, user1
;; @0024                               v39 = icmp ule v38, v10
;; @0024                               trapz v39, user1
;; @0024                               v40 = iadd v8, v36
;; @0024                               v41 = load.i64 notrap aligned little v40
;; @002b                               v54 = icmp ult v4, v18
;; @002b                               trapz v54, user17
;;                                     v99 = ishl v4, v89  ; v89 = 3
;; @002b                               v64 = iadd v99, v25  ; v25 = 16
;; @002b                               v70 = uextend.i64 v64
;; @002b                               v71 = uadd_overflow_trap v11, v70, user1
;; @002b                               v75 = iadd v8, v71
;; @002b                               v76 = load.i64 notrap aligned little v75
;; @002e                               jump block1
;;
;;                                 block1:
;; @002e                               return v41, v76
;; }
