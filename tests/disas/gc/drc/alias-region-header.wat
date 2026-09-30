;;! target = "x86_64"
;;! test = "optimize"
;;! flags = "-W function-references,gc -C collector=drc"

;; The DRC collector's `VMDrcHeader::ref_count` and
;; `next_over_approximated_stack_root` each get their own alias region.

(module
  (type $s (struct (field (mut anyref))))
  (type $a (array (mut i32)))

  (func (export "f") (param $s (ref $s)) (param $a (ref $a)) (param $x anyref) (result anyref)
    ;; A GC-ref store emits DRC write barriers, which touch the ref counts of
    ;; both the old and the new value.
    (struct.set $s 0 (local.get $s) (local.get $x))

    ;; A GC-ref load emits a DRC read barrier, which touches the ref count, the
    ;; kind word's reserved bits, and the over-approximated-stack-roots link.
    (struct.get $s 0 (local.get $s))
  )
)
;; function u0:0(i64 vmctx, i64, i32, i32, i32) -> i32 tail {
;;     ss0 = explicit_slot 4, align = 4
;;     region0 = 123 ""
;;     region1 = 160 ""
;;     region2 = 196 ""
;;     region3 = 206 ""
;;     region4 = 147 ""
;;     region5 = 175 ""
;;     region6 = 239 ""
;;     region7 = 65 ""
;;     region8 = 210 ""
;;     region9 = 121 ""
;;     region10 = 115 ""
;;     region11 = 137 ""
;;     region12 = 135 ""
;;     gv0 = vmctx
;;     gv1 = load.i64 notrap aligned readonly can_move region0 gv0+8
;;     gv2 = load.i64 notrap aligned region1 gv1+24
;;     sig0 = (i64 vmctx, i32) tail
;;     sig1 = (i64 vmctx) -> i8 tail
;;     fn0 = colocated u805306368:22 sig0
;;     fn1 = colocated u805306368:45 sig1
;;     stack_limit = gv2
;;
;;                                 block0(v0: i64, v1: i64, v2: i32, v3: i32, v4: i32):
;;                                     v145 = stack_addr.i64 ss0
;;                                     store notrap aligned region12 v2, v145
;; @002f                               trapz v2, user16
;; @002f                               v6 = load.i64 notrap aligned readonly can_move region0 v0+8
;; @002f                               v7 = load.i64 notrap aligned readonly can_move region2 v6+32
;; @002f                               v5 = uextend.i64 v2
;; @002f                               v8 = iadd v7, v5
;; @002f                               v9 = iconst.i64 24
;; @002f                               v10 = iadd v8, v9  ; v9 = 24
;; @002f                               v11 = load.i32 user2 little region4 v10
;; @002f                               v12 = iconst.i32 1
;; @002f                               v13 = band v4, v12  ; v12 = 1
;; @002f                               v14 = iconst.i32 0
;; @002f                               v15 = icmp eq v4, v14  ; v14 = 0
;; @002f                               v16 = uextend.i32 v15
;; @002f                               v17 = bor v13, v16
;; @002f                               brif v17, block3, block2
;;
;;                                 block2:
;; @002f                               v18 = uextend.i64 v4
;; @002f                               v21 = iadd.i64 v7, v18
;; @002f                               v22 = iconst.i64 8
;; @002f                               v23 = iadd v21, v22  ; v22 = 8
;; @002f                               v24 = load.i64 user2 region5 v23
;; @002f                               v25 = iconst.i64 1
;; @002f                               v26 = iadd v24, v25  ; v25 = 1
;; @002f                               store user2 region5 v26, v23
;; @002f                               jump block3
;;
;;                                 block3:
;;                                     v159 = iadd.i64 v8, v9  ; v9 = 24
;; @002f                               store.i32 user2 little region4 v4, v159
;;                                     v160 = iconst.i32 1
;;                                     v161 = band.i32 v11, v160  ; v160 = 1
;;                                     v162 = iconst.i32 0
;;                                     v163 = icmp.i32 eq v11, v162  ; v162 = 0
;; @002f                               v37 = uextend.i32 v163
;; @002f                               v38 = bor v161, v37
;; @002f                               brif v38, block7, block4
;;
;;                                 block4:
;; @002f                               v39 = uextend.i64 v11
;; @002f                               v42 = iadd.i64 v7, v39
;;                                     v164 = iconst.i64 8
;; @002f                               v44 = iadd v42, v164  ; v164 = 8
;; @002f                               v45 = load.i64 user2 region5 v44
;;                                     v165 = iconst.i64 1
;;                                     v157 = icmp eq v45, v165  ; v165 = 1
;; @002f                               brif v157, block5, block6
;;
;;                                 block5 cold:
;; @002f                               call fn0(v0, v11), stack_map=[i32 @ ss0+0]
;; @002f                               jump block7
;;
;;                                 block6:
;; @002f                               v46 = iconst.i64 -1
;; @002f                               v47 = iadd.i64 v45, v46  ; v46 = -1
;;                                     v166 = iadd.i64 v42, v164  ; v164 = 8
;; @002f                               store user2 region5 v47, v166
;; @002f                               jump block7
;;
;;                                 block7:
;;                                     v140 = load.i32 notrap aligned region12 v145
;; @0035                               trapz v140, user16
;; @0035                               v58 = uextend.i64 v140
;; @0035                               v61 = iadd.i64 v7, v58
;;                                     v167 = iconst.i64 24
;;                                     v168 = iadd v61, v167  ; v167 = 24
;; @0035                               v64 = load.i32 user2 little region4 v168
;;                                     store notrap aligned region12 v64, v145
;;                                     v169 = iconst.i32 1
;;                                     v170 = band v64, v169  ; v169 = 1
;;                                     v171 = iconst.i32 0
;;                                     v172 = icmp eq v64, v171  ; v171 = 0
;; @0035                               v69 = uextend.i32 v172
;; @0035                               v70 = bor v170, v69
;; @0035                               brif v70, block10, block8
;;
;;                                 block8:
;; @0035                               v71 = uextend.i64 v64
;; @0035                               v74 = iadd.i64 v7, v71
;; @0035                               v75 = load.i32 user2 region6 v74
;; @0035                               v76 = iconst.i32 2
;; @0035                               v77 = band v75, v76  ; v76 = 2
;; @0035                               brif v77, block10, block9
;;
;;                                 block9:
;; @0035                               v78 = load.i64 notrap aligned readonly can_move region7 v0+32
;; @0035                               v79 = load.i32 notrap aligned region8 v78
;; @0035                               v84 = iconst.i64 16
;; @0035                               v85 = iadd.i64 v74, v84  ; v84 = 16
;; @0035                               store user2 region9 v79, v85
;;                                     v173 = iconst.i32 2
;;                                     v174 = bor.i32 v75, v173  ; v173 = 2
;; @0035                               store user2 region6 v174, v74
;;                                     v175 = iconst.i64 8
;; @0035                               v97 = iadd.i64 v74, v175  ; v175 = 8
;; @0035                               v98 = load.i64 user2 region5 v97
;;                                     v176 = iconst.i64 1
;; @0035                               v100 = iadd v98, v176  ; v176 = 1
;; @0035                               store user2 region5 v100, v97
;; @0035                               store.i32 notrap aligned region8 v64, v78
;; @0035                               v107 = load.i32 notrap aligned region10 v78+4
;;                                     v177 = iconst.i32 1
;;                                     v178 = iadd v107, v177  ; v177 = 1
;; @0035                               store notrap aligned region10 v178, v78+4
;; @0035                               v112 = load.i32 notrap aligned region11 v78+8
;; @0035                               v113 = iadd v112, v112
;; @0035                               v114 = iconst.i32 1024
;; @0035                               v115 = umax v113, v114  ; v114 = 1024
;; @0035                               v116 = icmp uge v178, v115
;; @0035                               brif v116, block11, block12
;;
;;                                 block11 cold:
;; @0035                               v117 = call fn1(v0), stack_map=[i32 @ ss0+0]
;; @0035                               jump block12
;;
;;                                 block12:
;; @0035                               jump block10
;;
;;                                 block10:
;; @0039                               jump block1
;;
;;                                 block1:
;;                                     v119 = load.i32 notrap aligned region12 v145
;; @0039                               return v119
;; }
