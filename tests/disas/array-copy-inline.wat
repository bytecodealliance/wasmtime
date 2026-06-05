;;! target = 'x86_64'
;;! test = 'optimize'
;;! flags = '-Wgc'

;; A small, constant-length `array.copy` is expanded inline as wide loads
;; followed by stores instead of calling the `memory_copy` libcall. The byte
;; range is covered greedily with the widest convenient access, so 7 `i32`s (28
;; bytes) become an `i8x16` + `i64` + `i32`, and every chunk is loaded before any
;; is stored so overlapping ranges still copy correctly.

(module
  (type $a (array (mut i32)))

  (func $copy (param (ref $a) i32 (ref $a) i32)
    (array.copy $a $a (local.get 0) (local.get 1) (local.get 2) (local.get 3) (i32.const 7))
  )
)
;; function u0:0(i64 vmctx, i64, i32, i32, i32, i32) tail {
;;     region0 = 2147483648 "GcHeap"
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
;; @002a                               v83 = load.i64 notrap aligned readonly can_move v0+8
;; @002a                               v8 = load.i64 notrap aligned readonly can_move v83+32
;; @002a                               v7 = uextend.i64 v2
;; @002a                               v9 = iadd v8, v7
;; @002a                               v10 = iconst.i64 16
;; @002a                               v11 = iadd v9, v10  ; v10 = 16
;; @002a                               v12 = load.i32 user2 readonly region0 v11
;; @002a                               v14 = uextend.i64 v3
;;                                     v85 = iconst.i64 7
;; @002a                               v18 = iadd v14, v85  ; v85 = 7
;; @002a                               v13 = uextend.i64 v12
;; @002a                               v19 = icmp ugt v18, v13
;; @002a                               trapnz v19, user17
;; @002a                               trapz v4, user16
;; @002a                               v29 = uextend.i64 v4
;; @002a                               v31 = iadd v8, v29
;; @002a                               v33 = iadd v31, v10  ; v10 = 16
;; @002a                               v34 = load.i32 user2 readonly region0 v33
;; @002a                               v36 = uextend.i64 v5
;; @002a                               v40 = iadd v36, v85  ; v85 = 7
;; @002a                               v35 = uextend.i64 v34
;; @002a                               v41 = icmp ugt v40, v35
;; @002a                               trapnz v41, user17
;; @002a                               v57 = load.i64 notrap aligned v83+40
;; @002a                               v23 = iconst.i64 20
;; @002a                               v24 = iadd v9, v23  ; v23 = 20
;;                                     v93 = iconst.i64 2
;;                                     v94 = ishl v14, v93  ; v93 = 2
;; @002a                               v28 = iadd v24, v94
;;                                     v98 = iconst.i64 28
;; @002a                               v59 = uadd_overflow_trap v28, v98, user2  ; v98 = 28
;; @002a                               v58 = iadd v8, v57
;; @002a                               v60 = icmp ugt v59, v58
;; @002a                               trapnz v60, user2
;; @002a                               v46 = iadd v31, v23  ; v23 = 20
;;                                     v96 = ishl v36, v93  ; v93 = 2
;; @002a                               v50 = iadd v46, v96
;; @002a                               v64 = uadd_overflow_trap v50, v98, user2  ; v98 = 28
;; @002a                               v65 = icmp ugt v64, v58
;; @002a                               trapnz v65, user2
;; @002a                               v66 = load.i8x16 notrap aligned little v50
;; @002a                               v67 = load.i64 notrap aligned little v50+16
;; @002a                               v68 = load.i32 notrap aligned little v50+24
;; @002a                               store notrap aligned little v66, v28
;; @002a                               store notrap aligned little v67, v28+16
;; @002a                               store notrap aligned little v68, v28+24
;; @002e                               jump block1
;;
;;                                 block1:
;; @002e                               return
;; }
