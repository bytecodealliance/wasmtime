;;! target = 'x86_64'
;;! test = 'optimize'
;;! flags = '-Wgc'

;; A statically-sized small `array.copy` is expanded inline as a sequence of
;; loads followed by stores (every element is loaded before any is stored so
;; overlapping ranges still copy correctly), instead of calling the
;; `memory_copy` libcall.

(module
  (type $a (array (mut i32)))

  (func $copy (param (ref $a) i32 (ref $a) i32)
    (array.copy $a $a (local.get 0) (local.get 1) (local.get 2) (local.get 3) (i32.const 4))
  )
)
;; function u0:0(i64 vmctx, i64, i32, i32, i32, i32) tail {
;;     gv0 = vmctx
;;     gv1 = load.i64 notrap aligned readonly gv0+8
;;     gv2 = load.i64 notrap aligned gv1+24
;;     gv3 = vmctx
;;     gv4 = load.i64 notrap aligned readonly can_move gv3+8
;;     gv5 = load.i64 notrap aligned readonly can_move gv4+32
;;     gv6 = load.i64 notrap aligned gv4+40
;;     stack_limit = gv2
;;
;;                                 block0(v0: i64, v1: i64, v2: i32, v3: i32, v4: i32, v5: i32):
;; @002a                               trapz v2, user16
;; @002a                               v84 = load.i64 notrap aligned readonly can_move v0+8
;; @002a                               v8 = load.i64 notrap aligned readonly can_move v84+32
;; @002a                               v7 = uextend.i64 v2
;; @002a                               v9 = iadd v8, v7
;; @002a                               v10 = iconst.i64 16
;; @002a                               v11 = iadd v9, v10  ; v10 = 16
;; @002a                               v12 = load.i32 user2 readonly v11
;; @002a                               v14 = uextend.i64 v3
;;                                     v86 = iconst.i64 4
;; @002a                               v17 = iadd v14, v86  ; v86 = 4
;; @002a                               v13 = uextend.i64 v12
;; @002a                               v18 = icmp ugt v17, v13
;; @002a                               trapnz v18, user17
;; @002a                               trapz v4, user16
;; @002a                               v26 = uextend.i64 v4
;; @002a                               v28 = iadd v8, v26
;; @002a                               v30 = iadd v28, v10  ; v10 = 16
;; @002a                               v31 = load.i32 user2 readonly v30
;; @002a                               v33 = uextend.i64 v5
;; @002a                               v36 = iadd v33, v86  ; v86 = 4
;; @002a                               v32 = uextend.i64 v31
;; @002a                               v37 = icmp ugt v36, v32
;; @002a                               trapnz v37, user17
;; @002a                               v49 = load.i64 notrap aligned v84+40
;;                                     v80 = iconst.i64 20
;; @002a                               v22 = iadd v9, v80  ; v80 = 20
;;                                     v89 = iconst.i64 2
;;                                     v96 = ishl v14, v89  ; v89 = 2
;; @002a                               v25 = iadd v22, v96
;; @002a                               v51 = uadd_overflow_trap v25, v10, user2  ; v10 = 16
;; @002a                               v50 = iadd v8, v49
;; @002a                               v52 = icmp ugt v51, v50
;; @002a                               trapnz v52, user2
;; @002a                               v41 = iadd v28, v80  ; v80 = 20
;;                                     v98 = ishl v33, v89  ; v89 = 2
;; @002a                               v44 = iadd v41, v98
;; @002a                               v56 = uadd_overflow_trap v44, v10, user2  ; v10 = 16
;; @002a                               v57 = icmp ugt v56, v50
;; @002a                               trapnz v57, user2
;; @002a                               v58 = load.i32 user2 v44
;; @002a                               v59 = load.i32 user2 v44+4
;; @002a                               v60 = load.i32 user2 v44+8
;; @002a                               v61 = load.i32 user2 v44+12
;; @002a                               store user2 v58, v25
;; @002a                               store user2 v59, v25+4
;; @002a                               store user2 v60, v25+8
;; @002a                               store user2 v61, v25+12
;; @002e                               jump block1
;;
;;                                 block1:
;; @002e                               return
;; }
