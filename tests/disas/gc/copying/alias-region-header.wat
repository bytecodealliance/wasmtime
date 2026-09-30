;;! target = "x86_64"
;;! test = "optimize"
;;! flags = "-W function-references,gc -C collector=copying"

;; The copying collector's `VMCopyingHeader::object_size` gets its own alias
;; region.

(module
  (type $s (struct (field (mut i32))))
  (type $a (array (mut i32)))

  (func (export "f") (param $a (ref $a)) (param $x anyref) (result i32)
    ;; Writes the kind word, the type-index word, the object size, and the
    ;; field.
    (struct.get $s 0 (struct.new $s (i32.const 1)))
  )
)
;; function u0:0(i64 vmctx, i64, i32, i32) -> i32 tail {
;;     region0 = 123 ""
;;     region1 = 160 ""
;;     region2 = 65 ""
;;     region3 = 177 ""
;;     region4 = 98 ""
;;     region5 = 130 ""
;;     region6 = 6 ""
;;     region7 = 196 ""
;;     region8 = 239 ""
;;     region9 = 134 ""
;;     region10 = 90 ""
;;     region11 = 147 ""
;;     region12 = 206 ""
;;     gv0 = vmctx
;;     gv1 = load.i64 notrap aligned readonly can_move region0 gv0+8
;;     gv2 = load.i64 notrap aligned region1 gv1+24
;;     sig0 = (i64 vmctx, i32, i32, i32, i32) -> i32 tail
;;     fn0 = colocated u805306368:24 sig0
;;     stack_limit = gv2
;;
;;                                 block0(v0: i64, v1: i64, v2: i32, v3: i32):
;; @002b                               v6 = load.i64 notrap aligned readonly can_move region2 v0+32
;; @002b                               v7 = load.i32 notrap aligned region3 v6
;; @002b                               v8 = load.i32 notrap aligned region4 v6+4
;; @002b                               v14 = uextend.i64 v7
;;                                     v46 = iconst.i64 32
;; @002b                               v15 = iadd v14, v46  ; v46 = 32
;; @002b                               v16 = uextend.i64 v8
;; @002b                               v17 = icmp ule v15, v16
;; @002b                               brif v17, block2, block3
;;
;;                                 block2:
;;                                     v62 = iconst.i32 32
;;                                     v60 = iadd.i32 v7, v62  ; v62 = 32
;; @002b                               store notrap aligned region3 v60, v6
;;                                     v63 = iconst.i32 -1342177278
;;                                     v64 = load.i64 notrap aligned readonly can_move region0 v0+8
;;                                     v65 = load.i64 notrap aligned readonly can_move region7 v64+32
;; @002b                               v31 = iadd v65, v14
;; @002b                               store user2 region8 v63, v31  ; v63 = -1342177278
;;                                     v66 = load.i64 notrap aligned readonly can_move region5 v0+40
;;                                     v67 = load.i32 notrap aligned readonly can_move region6 v66
;; @002b                               store user2 region9 v67, v31+4
;;                                     v68 = iconst.i64 32
;; @002b                               istore32 user2 region10 v68, v31+8  ; v68 = 32
;; @002b                               jump block4(v7, v31)
;;
;;                                 block3 cold:
;; @002b                               v18 = iconst.i32 -1342177278
;; @002b                               v19 = load.i64 notrap aligned readonly can_move region5 v0+40
;; @002b                               v20 = load.i32 notrap aligned readonly can_move region6 v19
;; @002b                               v5 = iconst.i32 32
;; @002b                               v21 = iconst.i32 16
;; @002b                               v22 = call fn0(v0, v18, v20, v5, v21)  ; v18 = -1342177278, v5 = 32, v21 = 16
;; @002b                               v23 = load.i64 notrap aligned readonly can_move region0 v0+8
;; @002b                               v24 = load.i64 notrap aligned readonly can_move region7 v23+32
;; @002b                               v25 = uextend.i64 v22
;; @002b                               v26 = iadd v24, v25
;; @002b                               jump block4(v22, v26)
;;
;;                                 block4(v35: i32, v36: i64):
;; @0029                               v4 = iconst.i32 1
;; @002b                               v37 = iconst.i64 16
;; @002b                               v38 = iadd v36, v37  ; v37 = 16
;; @002b                               store user2 little region11 v4, v38  ; v4 = 1
;; @002e                               trapz v35, user16
;;                                     v69 = load.i64 notrap aligned readonly can_move region0 v0+8
;;                                     v70 = load.i64 notrap aligned readonly can_move region7 v69+32
;; @002e                               v39 = uextend.i64 v35
;; @002e                               v42 = iadd v70, v39
;; @002e                               v44 = iadd v42, v37  ; v37 = 16
;; @002e                               v45 = load.i32 user2 little region11 v44
;; @0032                               jump block1
;;
;;                                 block1:
;; @0032                               return v45
;; }
