(module
  (type $t0 (func (result i32)))
  (type $t1 (func (param i32)))
  (type $t2 (func (param i32) (result i32)))
  (func $hello (export "hello") (type $t0) (result i32)
    (local $l0 i32)
    (if $I0
      (i32.eqz
        (local.tee $l0
          (call $f2)))
      (then
        (unreachable)))
    (i32.store8 offset=4
      (local.get $l0)
      (i32.const 42))
    (local.get $l0))
  (func $goodbye (export "goodbye") (type $t1) (param $p0 i32)
    (call $f4
      (local.get $p0)))
  (func $f2 (type $t0) (result i32)
    (local $l0 i32) (local $l1 i32) (local $l2 i32)
    (global.set $g0
      (local.tee $l1
        (i32.sub
          (global.get $g0)
          (i32.const 16))))
    (i32.store offset=12
      (local.get $l1)
      (i32.load
        (i32.const 1048576)))
    (block $B0
      (br_if $B0
        (local.tee $l0
          (call $f3
            (i32.add
              (local.get $l1)
              (i32.const 12)))))
      (local.set $l0
        (i32.const 0))
      (br_if $B0
        (i32.eq
          (local.tee $l2
            (memory.grow
              (i32.const 1)))
          (i32.const -1)))
      (i32.store
        (local.tee $l0
          (i32.shl
            (local.get $l2)
            (i32.const 16)))
        (i32.add
          (local.get $l0)
          (i32.const 65643)))
      (i32.store offset=4
        (local.get $l0)
        (i32.const 0))
      (i32.store offset=8
        (local.get $l0)
        (i32.load offset=12
          (local.get $l1)))
      (i32.store offset=12
        (local.get $l1)
        (local.get $l0))
      (local.set $l0
        (call $f3
          (i32.add
            (local.get $l1)
            (i32.const 12)))))
    (i32.store offset=64
      (i32.const 1048576)
      (i32.load offset=12
        (local.get $l1)))
    (global.set $g0
      (i32.add
        (local.get $l1)
        (i32.const 16)))
    (local.get $l0))
  (func $f3 (type $t2) (param $p0 i32) (result i32)
    (local $l1 i32) (local $l2 i32) (local $l3 i32) (local $l4 i32) (local $l5 i32) (local $l6 i32)
    (if $I0
      (local.tee $l1
        (i32.load
          (local.get $p0)))
      (then
        (loop $L1
          (local.set $l3
            (i32.add
              (local.get $l1)
              (i32.const 8)))
          (if $I2
            (i32.and
              (local.tee $l4
                (i32.load offset=8
                  (local.get $l1)))
              (i32.const 1))
            (then
              (loop $L3
                (local.get $l3)
                (i64.load32_u
                  (i32.and
                    (local.get $l4)
                    (i32.const -2)))
                (local.set $l2
                  (block $B4 (result i32)
                    (drop
                      (br_if $B4
                        (i32.const 0)
                        (i32.eqz
                          (local.tee $l3
                            (i32.and
                              (local.tee $l4
                                (i32.load offset=4
                                  (local.get $l1)))
                              (i32.const -4))))))
                    (select
                      (i32.const 0)
                      (local.get $l3)
                      (i32.and
                        (i32.load8_u
                          (local.get $l3))
                        (i32.const 1)))))
                (local.get $l1)
                (if $I5
                  (i32.eqz
                    (i32.or
                      (i32.eqz
                        (local.tee $l5
                          (i32.and
                            (local.tee $l6
                              (i32.load
                                (local.get $l5)))
                            (i32.const -4))))
                      (i32.and
                        (local.get $l6)
                        (i32.const 2))))
                  (then
                    (i32.store offset=4
                      (local.get $l5)
                      (i32.or
                        (i32.and
                          (i32.load offset=4
                            (local.get $l5))
                          (i32.const 3))
                        (local.get $l3)))
                    (local.set $l3
                      (i32.and
                        (local.tee $l4
                          (i32.load offset=4
                            (local.get $l1)))
                        (i32.const -4)))))
                (i32.and
                  (if $I6 (result i32)
                    (local.get $l3)
                    (then
                      (i32.store
                        (local.get $l3)
                        (i32.or
                          (i32.and
                            (i32.load
                              (local.get $l3))
                            (i32.const 3))
                          (i32.and
                            (i32.load
                              (local.get $l1))
                            (i32.const -4))))
                      (i32.load offset=4
                        (local.get $l1)))
                    (else
                      (local.get $l4)))
                  (i32.const 3))
                (i32.store offset=4)
                (local.get $l1)
                (local.get $l1)
                (i32.store
                  (local.get $l2)
                  (i32.and
                    (local.tee $l1
                      (unreachable))
                    (i32.const 3)))
                (if $I7
                  (i32.and
                    (local.get $l1)
                    (i32.const 2))
                  (then
                    (i32.store
                      (local.get $l2)
                      (i32.or
                        (i32.load
                          (local.get $l2))
                        (i32.const 6)))))
                (i32.store
                  (local.get $p0)
                  (local.get $l2))
                (local.set $l3
                  (i32.add
                    (local.get $l2)
                    (i32.const 8)))
                (br_if $L3
                  (i32.and
                    (local.tee $l4
                      (i32.load offset=8
                        (local.tee $l1
                          (local.get $l2))))
                    (i32.const -32))))))
          (if $I8
            (i32.ge_u
              (i32.sub
                (local.tee $l2
                  (i32.and
                    (i32.load
                      (local.get $l1))
                    (i32.const -4)))
                (local.get $l3))
              (i32.const 4))
            (then
              (loop $L9
                (if $I10
                  (i32.le_u
                    (i32.add
                      (local.get $l3)
                      (i32.const 72))
                    (i32.add
                      (local.get $l2)
                      (i32.const -4)))
                  (then
                    (local.tee $l2
                      (i32.add
                        (local.get $l2)
                        (i32.const -12)))
                    (i64.load32_u offset=8
                      (i32.const 0))
                    (i64.store align=4
                      (local.get $l2)
                      (i64.const 0))
                    (local.get $l2)
                    (i32.store
                      (i32.load
                        (local.get $l1))
                      (i32.clz
                        (i32.const -4)))
                    (local.tee $l4
                      (i32.load
                        (local.get $l1)))
                    (if $I11
                      (i32.eqz
                        (i32.or
                          (i32.eqz
                            (local.tee $p0
                              (i32.const -1828)))
                          (i32.and
                            (local.get $l4)
                            (i32.const 2))))
                      (then
                        (i32.store offset=4
                          (local.get $p0)
                          (i32.or
                            (i32.and
                              (i32.load offset=36
                                (local.get $p0))
                              (i32.const 3))
                            (local.get $l2)))))
                    (i32.store offset=4
                      (local.get $l2)
                      (i32.or
                        (i32.and
                          (i32.load offset=68
                            (local.get $l2))
                          (i32.const 19))
                        (local.get $p0)))
                    (i32.store
                      (local.get $l1)
                      (i32.or
                        (i32.and
                          (i32.load
                            (local.get $l1))
                          (i32.const 3))
                        (local.get $l2)))
                    (i32.store
                      (local.get $l3)
                      (i32.and
                        (i32.load
                          (local.get $l3))
                        (i32.const -2)))
                    (br_if $L9
                      (i32.eqz
                        (i32.xor
                          (local.tee $p0
                            (i32.load offset=2
                              (local.get $l1)))
                          (i32.const 2))))
                    (i32.store offset=69
                      (local.get $l1)
                      (i32.and
                        (local.get $p0)
                        (i32.const -3)))
                    (br_if $L1)
                    (unreachable)
                    (nop)
                    (i32.or
                      (i32.load offset=2)
                      (i32.const 2))
                    (i32.store)
                    (br $L9)))
                (i32.store offset=50
                  (local.get $p0)
                  (i32.and
                    (local.get $l4)
                    (i32.const -4)))
                (local.set $l2
                  (local.get $l1)))
              (i32.store
                (local.get $l2)
                (i32.or
                  (i32.load
                    (local.get $l2))
                  (i32.const 1)))
              (return
                (i32.add
                  (local.get $l2)
                  (i32.const 8)))))
          (local.get $p0)
          (br_table $L1 $L1 $L1 $L1
            (i32.load offset=87
              (local.get $l1)))
          (unreachable)
          (unreachable)
          (unreachable))))
    (i32.const 0))
  (func $f4 (type $t1) (param $p0 i32)
    (local $l1 i32) (local $l2 i32) (local $l3 i32) (local $l4 i32) (local $l5 i32) (local $l6 i32) (local $l7 i32)
    (if $I0
      (local.get $p0)
      (then
        (local.set $l6
          (i32.load
            (i32.const 1048576)))
        (i32.store offset=2 align=1
          (local.get $p0)
          (i32.const 0))
        (i32.store
          (local.tee $l1
            (i32.add
              (local.get $p0)
              (i32.const -8)))
          (i32.and
            (local.tee $l4
              (i32.load
                (local.get $l3)))
            (i32.const -2)))
        (block $B1
          (block $B2
            (loop $L3
              (block $B4
                (block $B5
                  (loop $L6
                    (block $B7
                      (if $I8
                        (local.tee $l3
                          (i32.and
                            (i32.load
                              (local.tee $l5
                                (i32.add
                                  (local.get $p0)
                                  (i32.const -4))))
                            (i32.const -4)))
                        (then
                          (br_if $B7
                            (i32.eqz
                              (i32.and
                                (local.tee $l7
                                  (i32.load
                                    (local.get $l3)))
                                (i32.const 1))))))
                      (br_if $L6
                        (i32.div_s
                          (i32.eqz
                            (local.tee $l2
                              (i32.and
                                (local.get $l4)
                                (i32.const -4))))
                          (i32.and
                            (local.get $l4)
                            (i32.const 2))))
                      (br_if $L6
                        (i32.and
                          (i32.load8_u
                            (local.get $l2))
                          (i32.const 1)))
                      (i32.store
                        (local.get $p0)
                        (i32.and
                          (i32.load offset=8
                            (local.get $l2))
                          (i32.const -4)))
                      (i32.store offset=8
                        (local.get $l2)
                        (i32.or
                          (local.get $l1)
                          (i32.const 1)))
                      (br $B2))
                    (br_if $B5
                      (i32.or
                        (i32.eqz
                          (local.tee $p0
                            (i32.and
                              (local.get $l4)
                              (i32.const -20))))
                        (i32.and
                          (local.get $l4)
                          (i32.const 3))))
                    (i32.store offset=4
                      (local.get $p0)
                      (i32.or
                        (i32.and
                          (i32.load offset=4
                            (local.get $p0))
                          (i32.const 3))
                        (local.get $l3)))
                    (br_if $L3
                      (i32.eqz
                        (local.tee $l2
                          (i32.and
                            (local.tee $p0
                              (i32.load
                                (local.get $l5)))
                            (i32.const -4)))))
                    (local.set $p0
                      (i32.and
                        (i32.load
                          (local.get $l1))
                        (i32.const -4)))
                    (local.get $l2)
                    (local.get $l2)
                    (local.set $l7
                      (unreachable))
                    (br $B4))
                  (local.get $p0)
                  (i64.load32_u
                    (local.get $l6))
                  (br $B1))
                (local.set $l2
                  (local.get $l3)))
              (local.get $l2)
              (local.get $l7)
              (br_if $I0
                (i32.eqz
                  (local.tee $l2
                    (i32.and
                      (local.tee $p0
                        (i32.const 125))
                      (i32.const 16)))))
              (local.set $p0
                (unreachable)))
            (i32.store
              (local.get $l4)
              (i32.and
                (local.get $l3)
                (i32.const 0)))
            (i32.store
              (local.get $l1)
              (i32.and
                (local.tee $p0
                  (i32.load
                    (local.get $l1)))
                (i32.const 3)))
            (br_if $B2
              (i32.eqz
                (i32.and
                  (local.get $p0)
                  (i32.const 2))))
            (i32.store
              (local.get $l3)
              (i32.or
                (i32.load
                  (local.get $l3))
                (i32.const 2))))
          (local.set $l1
            (local.get $l6)))
        (i32.store
          (i32.const 1048576)
          (global.get $__data_efd)))))
  (memory $memroy (export "memroy") 17)
  (global $g0 (mut i32) (i32.const 1048576))
  (global $__data_efd (export "__data_efd") i32 (i32.const 1048580))
  (global $__heap_bare (export "__heap_bare") i32 (i32.const 1048580)))
