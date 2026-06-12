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
;;     region0 = 8 "VMContext+0x8"
;;     region1 = 2147483648 "GcHeap"
;;     gv0 = vmctx
;;     gv1 = load.i64 notrap aligned readonly region0 gv0+8
;;     gv2 = load.i64 notrap aligned gv1+24
;;     gv3 = vmctx
;;     gv4 = load.i64 notrap aligned readonly can_move region0 gv3+8
;;     gv5 = load.i64 notrap aligned readonly can_move gv4+32
;;     gv6 = load.i64 notrap aligned gv4+40
;;     stack_limit = gv2
;;
;;                                 block0(v0: i64, v1: i64, v2: i32, v3: i32, v4: i32, v5: i32):
;; @002a                               trapz v2, user16
;; @002a                               v77 = load.i64 notrap aligned readonly can_move region0 v0+8
;; @002a                               v8 = load.i64 notrap aligned readonly can_move v77+32
;; @002a                               v7 = uextend.i64 v2
;; @002a                               v9 = iadd v8, v7
;; @002a                               v10 = iconst.i64 16
;; @002a                               v11 = iadd v9, v10  ; v10 = 16
;; @002a                               v12 = load.i32 user2 readonly region1 v11
;; @002a                               v14 = uextend.i64 v3
;;                                     v79 = iconst.i64 7
;; @002a                               v18 = iadd v14, v79  ; v79 = 7
;; @002a                               v13 = uextend.i64 v12
;; @002a                               v19 = icmp ugt v18, v13
;; @002a                               trapnz v19, user17
;; @002a                               trapz v4, user16
;; @002a                               v30 = uextend.i64 v4
;; @002a                               v32 = iadd v8, v30
;; @002a                               v34 = iadd v32, v10  ; v10 = 16
;; @002a                               v35 = load.i32 user2 readonly region1 v34
;; @002a                               v37 = uextend.i64 v5
;; @002a                               v41 = iadd v37, v79  ; v79 = 7
;; @002a                               v36 = uextend.i64 v35
;; @002a                               v42 = icmp ugt v41, v36
;; @002a                               trapnz v42, user17
;; @002a                               v61 = load.i64 notrap aligned v77+40
;; @002a                               v24 = iconst.i64 20
;; @002a                               v25 = iadd v9, v24  ; v24 = 20
;;                                     v87 = iconst.i64 2
;;                                     v88 = ishl v14, v87  ; v87 = 2
;; @002a                               v29 = iadd v25, v88
;;                                     v92 = iconst.i64 28
;; @002a                               v63 = uadd_overflow_trap v29, v92, user2  ; v92 = 28
;; @002a                               v62 = iadd v8, v61
;; @002a                               v64 = icmp ugt v63, v62
;; @002a                               trapnz v64, user2
;; @002a                               v48 = iadd v32, v24  ; v24 = 20
;;                                     v90 = ishl v37, v87  ; v87 = 2
;; @002a                               v52 = iadd v48, v90
;; @002a                               v70 = uadd_overflow_trap v52, v92, user2  ; v92 = 28
;; @002a                               v71 = icmp ugt v70, v62
;; @002a                               trapnz v71, user2
;; @002a                               v72 = load.i8x16 notrap aligned little region1 v52
;; @002a                               v73 = load.i64 notrap aligned little region1 v52+16
;; @002a                               v74 = load.i32 notrap aligned little region1 v52+24
;; @002a                               store notrap aligned little region1 v72, v29
;; @002a                               store notrap aligned little region1 v73, v29+16
;; @002a                               store notrap aligned little region1 v74, v29+24
;; @002e                               jump block1
;;
;;                                 block1:
;; @002e                               return
;; }
