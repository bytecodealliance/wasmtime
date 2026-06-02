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
;; @002a                               v17 = iadd v14, v85  ; v85 = 7
;; @002a                               v13 = uextend.i64 v12
;; @002a                               v18 = icmp ugt v17, v13
;; @002a                               trapnz v18, user17
;; @002a                               trapz v4, user16
;; @002a                               v27 = uextend.i64 v4
;; @002a                               v29 = iadd v8, v27
;; @002a                               v31 = iadd v29, v10  ; v10 = 16
;; @002a                               v32 = load.i32 user2 readonly region0 v31
;; @002a                               v34 = uextend.i64 v5
;; @002a                               v37 = iadd v34, v85  ; v85 = 7
;; @002a                               v33 = uextend.i64 v32
;; @002a                               v38 = icmp ugt v37, v33
;; @002a                               trapnz v38, user17
;; @002a                               v51 = load.i64 notrap aligned v83+40
;; @002a                               v22 = iconst.i64 20
;; @002a                               v23 = iadd v9, v22  ; v22 = 20
;;                                     v93 = iconst.i64 2
;;                                     v94 = ishl v14, v93  ; v93 = 2
;; @002a                               v26 = iadd v23, v94
;;                                     v98 = iconst.i64 28
;; @002a                               v53 = uadd_overflow_trap v26, v98, user2  ; v98 = 28
;; @002a                               v52 = iadd v8, v51
;; @002a                               v54 = icmp ugt v53, v52
;; @002a                               trapnz v54, user2
;; @002a                               v43 = iadd v29, v22  ; v22 = 20
;;                                     v96 = ishl v34, v93  ; v93 = 2
;; @002a                               v46 = iadd v43, v96
;; @002a                               v58 = uadd_overflow_trap v46, v98, user2  ; v98 = 28
;; @002a                               v59 = icmp ugt v58, v52
;; @002a                               trapnz v59, user2
;; @002a                               v60 = load.i8x16 notrap aligned little v46
;; @002a                               v61 = load.i64 notrap aligned little v46+16
;; @002a                               v62 = load.i32 notrap aligned little v46+24
;; @002a                               store notrap aligned little v60, v26
;; @002a                               store notrap aligned little v61, v26+16
;; @002a                               store notrap aligned little v62, v26+24
;; @002e                               jump block1
;;
;;                                 block1:
;; @002e                               return
;; }
