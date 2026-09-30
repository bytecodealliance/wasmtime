;;! target = "x86_64"
;;! test = "optimize"
;;! flags = "-W function-references,gc -C collector=null"

;; Every word of a GC object's header gets its own alias region: the kind word,
;; the type-index word, an array's length, and a struct field should all be
;; different regions here.

(module
  (type $s (struct (field (mut i32))))
  (type $a (array (mut i32)))

  (func (export "f") (param $a (ref $a)) (param $x anyref) (result i32 i32 i32)
    ;; Writes the kind word, the type-index word, and the field.
    (struct.get $s 0 (struct.new $s (i32.const 1)))

    ;; Reads the length word.
    (array.len (local.get $a))

    ;; Reads the kind word and the type-index word.
    (ref.test (ref $s) (local.get $x))
  )
)
;; function u0:0(i64 vmctx, i64, i32, i32) -> i32, i32, i32 tail {
;;     ss0 = explicit_slot 4, align = 4
;;     ss1 = explicit_slot 4, align = 4
;;     region0 = 123 ""
;;     region1 = 160 ""
;;     region2 = 65 ""
;;     region3 = 237 ""
;;     region4 = 206 ""
;;     region5 = 196 ""
;;     region6 = 130 ""
;;     region7 = 6 ""
;;     region8 = 239 ""
;;     region9 = 134 ""
;;     region10 = 147 ""
;;     region11 = 108 ""
;;     region12 = 135 ""
;;     region13 = 187 ""
;;     gv0 = vmctx
;;     gv1 = load.i64 notrap aligned readonly can_move region0 gv0+8
;;     gv2 = load.i64 notrap aligned region1 gv1+24
;;     sig0 = (i64 vmctx, i64) -> i8 tail
;;     fn0 = colocated u805306368:23 sig0
;;     stack_limit = gv2
;;
;;                                 block0(v0: i64, v1: i64, v2: i32, v3: i32):
;;                                     v92 = stack_addr.i64 ss1
;;                                     store notrap aligned region13 v2, v92
;;                                     v93 = stack_addr.i64 ss0
;;                                     store notrap aligned region12 v3, v93
;; @002d                               v9 = load.i64 notrap aligned readonly can_move region2 v0+32
;; @002d                               v10 = load.i32 notrap aligned region3 v9
;;                                     v100 = iconst.i32 7
;; @002d                               v13 = uadd_overflow_trap v10, v100, user18  ; v100 = 7
;;                                     v106 = iconst.i32 -8
;; @002d                               v15 = band v13, v106  ; v106 = -8
;; @002d                               v5 = iconst.i32 16
;; @002d                               v16 = uadd_overflow_trap v15, v5, user18  ; v5 = 16
;; @002d                               v18 = load.i64 notrap aligned readonly can_move region0 v0+8
;; @002d                               v19 = load.i64 notrap aligned region4 v18+40
;; @002d                               v17 = uextend.i64 v16
;; @002d                               v20 = icmp ule v17, v19
;; @002d                               brif v20, block2, block3
;;
;;                                 block2:
;;                                     v107 = iconst.i32 -1342177264
;; @002d                               v24 = load.i64 notrap aligned readonly can_move region5 v18+32
;;                                     v113 = band.i32 v13, v106  ; v106 = -8
;;                                     v114 = uextend.i64 v113
;; @002d                               v26 = iadd v24, v114
;; @002d                               store user2 region8 v107, v26  ; v107 = -1342177264
;; @002d                               v29 = load.i64 notrap aligned readonly can_move region6 v0+40
;; @002d                               v30 = load.i32 notrap aligned readonly can_move region7 v29
;; @002d                               store user2 region9 v30, v26+4
;; @002d                               store.i32 notrap aligned region3 v16, v9
;; @002b                               v4 = iconst.i32 1
;; @002d                               v31 = iconst.i64 8
;; @002d                               v32 = iadd v26, v31  ; v31 = 8
;; @002d                               store user2 little region10 v4, v32  ; v4 = 1
;; @0030                               trapz v113, user16
;;                                     v91 = load.i32 notrap aligned region13 v92
;; @0036                               trapz v91, user16
;; @0036                               v41 = uextend.i64 v91
;; @0036                               v44 = iadd v24, v41
;; @0036                               v46 = iadd v44, v31  ; v31 = 8
;; @0036                               v47 = load.i32 user2 readonly region11 v46
;;                                     v87 = load.i32 notrap aligned region12 v93
;;                                     v94 = iconst.i32 0
;; @003a                               v50 = icmp eq v87, v94  ; v94 = 0
;; @003a                               brif v50, block6(v94), block4  ; v94 = 0
;;
;;                                 block3 cold:
;; @002d                               v21 = isub.i64 v17, v19
;; @002d                               v22 = call fn0(v0, v21), stack_map=[i32 @ ss1+0, i32 @ ss0+0]
;; @002d                               jump block2
;;
;;                                 block4:
;;                                     v115 = iconst.i32 1
;;                                     v116 = band.i32 v87, v115  ; v115 = 1
;;                                     v117 = iconst.i32 0
;; @003a                               brif v116, block6(v117), block5  ; v117 = 0
;;
;;                                 block5:
;; @003a                               v56 = uextend.i64 v87
;; @003a                               v59 = iadd.i64 v24, v56
;; @003a                               v62 = load.i32 user2 readonly region8 v59
;; @002d                               v27 = iconst.i32 -1342177280
;; @003a                               v64 = band v62, v27  ; v27 = -1342177280
;; @003a                               v65 = icmp eq v64, v27  ; v27 = -1342177280
;;                                     v118 = iconst.i32 0
;; @003a                               brif v65, block7, block6(v118)  ; v118 = 0
;;
;;                                 block7:
;; @003a                               v74 = iconst.i64 4
;; @003a                               v75 = iadd.i64 v59, v74  ; v74 = 4
;; @003a                               v76 = load.i32 user2 readonly region9 v75
;; @003a                               v77 = icmp eq v76, v30
;; @003a                               v78 = uextend.i32 v77
;; @003a                               jump block6(v78)
;;
;;                                 block6(v79: i32):
;; @003d                               jump block1
;;
;;                                 block1:
;;                                     v119 = iconst.i32 1
;; @003d                               return v119, v47, v79  ; v119 = 1
;; }
