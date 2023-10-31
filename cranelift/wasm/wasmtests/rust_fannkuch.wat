(module
 (type $0 (func (param i32 i32 i32) (result i32)))
 (type $1 (func (param i32 i32) (result i32)))
 (type $2 (func (param i32)))
 (type $3 (func (param i32) (result i32)))
 (type $4 (func (param i32 i32)))
 (type $5 (func (param i64 i32) (result i32)))
 (type $6 (func (param i32) (result i64)))
 (type $7 (func))
 (type $8 (func (param i32 i32)))
 (type $9 (func (param i32 i32 i32) (result i32)))
 (memory $0 17)
 (data (i32.const 1048576) "src/lib.rs\00\00\00\00\00\00attempt to divide by zero\00\00\00\00\00\00\00attempt to divide with overflow\00index out of bounds: the len is  but the index is 00010203040506070809101112131415161718192021222324252627282930313233343536373839404142434445464748495051525354555657585960616263646566676869707172737475767778798081828384858687888990919293949596979899called `Option::unwrap()` on a `None` valuesrc/libcore/option.rssrc/lib.rs")
 (data (i32.const 1048982) "\10\00\n\00\00\00%\00\00\00\1d\00\00\00\10\00\10\00\19\00\00\00\00\00\10\00\n\00\00\00&\00\00\00\15\00\00\000\00\10\00\1f\00\00\00\00\00\10\00\n\00\00\00&\00\00\00\15\00\00\00\00\00\10\00\n\00\00\00.\00\00\00\15\00\00\00\00\00\10\00\n\00\00\000\00\00\00\15\00\00\00\00\00\10\00\n\00\00\00-\00\00\00\11\00\00\00\00\00\10\00\n\00\00\00E\00\00\00\17\00\00\00\00\00\10\00\n\00\00\00q\00\00\00\"\00\00\00\00\00\10\00\n\00\00\00s\00\00\00\11\00\00\00P\00\10\00 \00\00\00p\00\10\00\12\00\00\00\02\00\00\00\00\00\00\00\01\00\00\00\03\00\00\00J\01\10\00+\00\00\00u\01\10\00\15\00\00\00Y\01\00\00\15\00\00\00\8a\01\10\00\n\00\00\00\08\00\00\00\t\00\00\00\8a\01\10\00\n\00\00\00\n\00\00\00\14")
 (table $0 4 4 funcref)
 (elem (i32.const 1) $4 $7 $8)
 (global $global$0 (mut i32) (i32.const 1048576))
 (global $global$1 i32 (i32.const 1049244))
 (global $global$2 i32 (i32.const 1049244))
 (export "memory" (memory $0))
 (export "__heap_base" (global $global$1))
 (export "__data_end" (global $global$2))
 (export "run_fannkuch" (func $10))
 (func $0 (; 0 ;) (type $7)
  (local $0 i32)
  (local $1 i32)
  (local.set $0
   (i32.const 1)
  )
  (block $label$1
   (block $label$2
    (block $label$3
     (if
      (i32.eq
       (i32.load
        (i32.const 1049232)
       )
       (i32.const 1)
      )
      (block
       (i32.store
        (i32.const 1049236)
        (local.tee $0
         (i32.add
          (i32.load
           (i32.const 1049236)
          )
          (i32.const 1)
         )
        )
       )
       (br_if $label$3
        (i32.lt_u
         (local.get $0)
         (i32.const 3)
        )
       )
       (br $label$2)
      )
     )
     (i64.store
      (i32.const 1049232)
      (i64.const 4294967297)
     )
    )
    (br_if $label$2
     (i32.le_s
      (local.tee $1
       (i32.load
        (i32.const 1049240)
       )
      )
      (i32.const -1)
     )
    )
    (i32.store
     (i32.const 1049240)
     (local.get $1)
    )
    (br_if $label$1
     (i32.lt_u
      (local.get $0)
      (i32.const 2)
     )
    )
   )
   (unreachable)
  )
  (unreachable)
 )
 (func $1 (; 1 ;) (type $2) (param $0 i32)
  (local $1 i32)
  (global.set $global$0
   (local.tee $1
    (i32.sub
     (global.get $global$0)
     (i32.const 16)
    )
   )
  )
  (if
   (i32.eqz
    (i32.load offset=8
     (local.get $0)
    )
   )
   (block
    (call $2
     (i32.const 1049172)
    )
    (unreachable)
   )
  )
  (i64.store offset=8
   (local.get $1)
   (i64.load align=4
    (i32.add
     (local.get $0)
     (i32.const 20)
    )
   )
  )
  (i64.store
   (local.get $1)
   (i64.load offset=12 align=4
    (local.get $0)
   )
  )
  (call $0)
  (unreachable)
 )
 (func $2 (; 2 ;) (type $2) (param $0 i32)
  (local $1 i32)
  (local $2 i64)
  (local $3 i64)
  (local $4 i64)
  (global.set $global$0
   (local.tee $1
    (i32.sub
     (global.get $global$0)
     (i32.const 48)
    )
   )
  )
  (local.set $2
   (i64.load offset=8 align=4
    (local.get $0)
   )
  )
  (local.set $3
   (i64.load offset=16 align=4
    (local.get $0)
   )
  )
  (local.set $4
   (i64.load align=4
    (local.get $0)
   )
  )
  (i32.store
   (i32.add
    (local.get $1)
    (i32.const 20)
   )
   (i32.const 0)
  )
  (i64.store offset=24
   (local.get $1)
   (local.get $4)
  )
  (i32.store offset=16
   (local.get $1)
   (i32.const 1048656)
  )
  (i64.store offset=4 align=4
   (local.get $1)
   (i64.const 1)
  )
  (i32.store
   (local.get $1)
   (i32.add
    (local.get $1)
    (i32.const 24)
   )
  )
  (i64.store offset=40
   (local.get $1)
   (local.get $3)
  )
  (i64.store offset=32
   (local.get $1)
   (local.get $2)
  )
  (call $5
   (local.get $1)
   (i32.add
    (local.get $1)
    (i32.const 32)
   )
  )
  (unreachable)
 )
 (func $3 (; 3 ;) (type $8) (param $0 i32) (param $1 i32)
  (local $2 i32)
  (global.set $global$0
   (local.tee $2
    (i32.sub
     (global.get $global$0)
     (i32.const 48)
    )
   )
  )
  (i32.store offset=4
   (local.get $2)
   (i32.const 16)
  )
  (i32.store
   (local.get $2)
   (local.get $1)
  )
  (i32.store
   (i32.add
    (local.get $2)
    (i32.const 44)
   )
   (i32.const 1)
  )
  (i32.store
   (i32.add
    (local.get $2)
    (i32.const 28)
   )
   (i32.const 2)
  )
  (i32.store offset=36
   (local.get $2)
   (i32.const 1)
  )
  (i64.store offset=12 align=4
   (local.get $2)
   (i64.const 2)
  )
  (i32.store offset=8
   (local.get $2)
   (i32.const 1049140)
  )
  (i32.store offset=40
   (local.get $2)
   (local.get $2)
  )
  (i32.store offset=32
   (local.get $2)
   (i32.add
    (local.get $2)
    (i32.const 4)
   )
  )
  (i32.store offset=24
   (local.get $2)
   (i32.add
    (local.get $2)
    (i32.const 32)
   )
  )
  (call $5
   (i32.add
    (local.get $2)
    (i32.const 8)
   )
   (local.get $0)
  )
  (unreachable)
 )
 (func $4 (; 4 ;) (type $1) (param $0 i32) (param $1 i32) (result i32)
  (call $6
   (i64.load32_u
    (local.get $0)
   )
   (local.get $1)
  )
 )
 (func $5 (; 5 ;) (type $4) (param $0 i32) (param $1 i32)
  (local $2 i32)
  (local $3 i64)
  (global.set $global$0
   (local.tee $2
    (i32.sub
     (global.get $global$0)
     (i32.const 32)
    )
   )
  )
  (local.set $3
   (i64.load align=4
    (local.get $1)
   )
  )
  (i64.store align=4
   (i32.add
    (local.get $2)
    (i32.const 20)
   )
   (i64.load offset=8 align=4
    (local.get $1)
   )
  )
  (i64.store offset=12 align=4
   (local.get $2)
   (local.get $3)
  )
  (i32.store offset=8
   (local.get $2)
   (local.get $0)
  )
  (i32.store offset=4
   (local.get $2)
   (i32.const 1049156)
  )
  (i32.store
   (local.get $2)
   (i32.const 1048656)
  )
  (call $1
   (local.get $2)
  )
  (unreachable)
 )
 (func $6 (; 6 ;) (type $5) (param $0 i64) (param $1 i32) (result i32)
  (local $2 i32)
  (local $3 i32)
  (local $4 i32)
  (local $5 i32)
  (local $6 i32)
  (local $7 i32)
  (local $8 i32)
  (local $9 i32)
  (local $10 i32)
  (local $11 i32)
  (local $12 i32)
  (local $13 i64)
  (local $14 i32)
  (local $15 i32)
  (global.set $global$0
   (local.tee $6
    (i32.sub
     (global.get $global$0)
     (i32.const 48)
    )
   )
  )
  (local.set $2
   (i32.const 39)
  )
  (block $label$1
   (block $label$2
    (if
     (i64.ge_u
      (local.get $0)
      (i64.const 10000)
     )
     (block
      (loop $label$4
       (i32.store16 align=1
        (i32.add
         (local.tee $3
          (i32.add
           (i32.add
            (local.get $6)
            (i32.const 9)
           )
           (local.get $2)
          )
         )
         (i32.const -4)
        )
        (i32.load16_u align=1
         (i32.add
          (i32.shl
           (local.tee $5
            (i32.div_u
             (local.tee $4
              (i32.wrap_i64
               (i64.add
                (local.get $0)
                (i64.mul
                 (local.tee $13
                  (i64.div_u
                   (local.get $0)
                   (i64.const 10000)
                  )
                 )
                 (i64.const -10000)
                )
               )
              )
             )
             (i32.const 100)
            )
           )
           (i32.const 1)
          )
          (i32.const 1048706)
         )
        )
       )
       (i32.store16 align=1
        (i32.add
         (local.get $3)
         (i32.const -2)
        )
        (i32.load16_u align=1
         (i32.add
          (i32.shl
           (i32.add
            (i32.mul
             (local.get $5)
             (i32.const -100)
            )
            (local.get $4)
           )
           (i32.const 1)
          )
          (i32.const 1048706)
         )
        )
       )
       (local.set $2
        (i32.add
         (local.get $2)
         (i32.const -4)
        )
       )
       (br_if $label$4
        (block (result i32)
         (local.set $14
          (i64.gt_u
           (local.get $0)
           (i64.const 99999999)
          )
         )
         (local.set $0
          (local.get $13)
         )
         (local.get $14)
        )
       )
      )
      (br_if $label$1
       (i32.le_s
        (local.tee $3
         (i32.wrap_i64
          (local.get $13)
         )
        )
        (i32.const 99)
       )
      )
      (br $label$2)
     )
    )
    (br_if $label$1
     (i32.le_s
      (local.tee $3
       (i32.wrap_i64
        (local.tee $13
         (local.get $0)
        )
       )
      )
      (i32.const 99)
     )
    )
   )
   (i32.store16 align=1
    (i32.add
     (local.tee $2
      (i32.add
       (local.get $2)
       (i32.const -2)
      )
     )
     (i32.add
      (local.get $6)
      (i32.const 9)
     )
    )
    (i32.load16_u align=1
     (i32.add
      (i32.shl
       (i32.and
        (i32.add
         (i32.mul
          (local.tee $3
           (i32.div_u
            (i32.and
             (local.tee $4
              (i32.wrap_i64
               (local.get $13)
              )
             )
             (i32.const 65535)
            )
            (i32.const 100)
           )
          )
          (i32.const -100)
         )
         (local.get $4)
        )
        (i32.const 65535)
       )
       (i32.const 1)
      )
      (i32.const 1048706)
     )
    )
   )
  )
  (block $label$5
   (if
    (i32.le_s
     (local.get $3)
     (i32.const 9)
    )
    (block
     (i32.store8
      (i32.add
       (local.tee $2
        (i32.add
         (local.get $2)
         (i32.const -1)
        )
       )
       (i32.add
        (local.get $6)
        (i32.const 9)
       )
      )
      (i32.add
       (local.get $3)
       (i32.const 48)
      )
     )
     (br $label$5)
    )
   )
   (i32.store16 align=1
    (i32.add
     (local.tee $2
      (i32.add
       (local.get $2)
       (i32.const -2)
      )
     )
     (i32.add
      (local.get $6)
      (i32.const 9)
     )
    )
    (i32.load16_u align=1
     (i32.add
      (i32.shl
       (local.get $3)
       (i32.const 1)
      )
      (i32.const 1048706)
     )
    )
   )
  )
  (local.set $7
   (i32.sub
    (i32.const 39)
    (local.get $2)
   )
  )
  (local.set $3
   (i32.const 1)
  )
  (local.set $8
   (select
    (i32.const 43)
    (i32.const 1114112)
    (local.tee $11
     (i32.and
      (local.tee $4
       (i32.load
        (local.get $1)
       )
      )
      (i32.const 1)
     )
    )
   )
  )
  (local.set $9
   (i32.and
    (i32.shr_s
     (i32.shl
      (local.get $4)
      (i32.const 29)
     )
     (i32.const 31)
    )
    (i32.const 1048656)
   )
  )
  (local.set $10
   (i32.add
    (i32.add
     (local.get $6)
     (i32.const 9)
    )
    (local.get $2)
   )
  )
  (block $label$7
   (block $label$8
    (block $label$9
     (block $label$10
      (block $label$11
       (block $label$12
        (block $label$13
         (block $label$14
          (local.set $3
           (block $label$15 (result i32)
            (block $label$16
             (block $label$17
              (block $label$18
               (block $label$19
                (if
                 (i32.eq
                  (i32.load offset=8
                   (local.get $1)
                  )
                  (i32.const 1)
                 )
                 (block
                  (br_if $label$19
                   (i32.le_u
                    (local.tee $5
                     (i32.load
                      (i32.add
                       (local.get $1)
                       (i32.const 12)
                      )
                     )
                    )
                    (local.tee $2
                     (i32.add
                      (local.get $7)
                      (local.get $11)
                     )
                    )
                   )
                  )
                  (br_if $label$18
                   (i32.and
                    (local.get $4)
                    (i32.const 8)
                   )
                  )
                  (local.set $4
                   (i32.sub
                    (local.get $5)
                    (local.get $2)
                   )
                  )
                  (br_if $label$17
                   (i32.eqz
                    (i32.and
                     (local.tee $3
                      (select
                       (i32.const 1)
                       (local.tee $3
                        (i32.load8_u offset=48
                         (local.get $1)
                        )
                       )
                       (i32.eq
                        (local.get $3)
                        (i32.const 3)
                       )
                      )
                     )
                     (i32.const 3)
                    )
                   )
                  )
                  (br_if $label$16
                   (i32.eq
                    (local.get $3)
                    (i32.const 2)
                   )
                  )
                  (local.set $5
                   (i32.const 0)
                  )
                  (br $label$15
                   (local.get $4)
                  )
                 )
                )
                (br_if $label$9
                 (call $9
                  (local.get $1)
                  (local.get $8)
                  (local.get $9)
                 )
                )
                (br $label$8)
               )
               (br_if $label$9
                (call $9
                 (local.get $1)
                 (local.get $8)
                 (local.get $9)
                )
               )
               (br $label$8)
              )
              (i32.store8 offset=48
               (local.get $1)
               (i32.const 1)
              )
              (i32.store offset=4
               (local.get $1)
               (i32.const 48)
              )
              (br_if $label$9
               (call $9
                (local.get $1)
                (local.get $8)
                (local.get $9)
               )
              )
              (local.set $3
               (i32.sub
                (local.get $5)
                (local.get $2)
               )
              )
              (br_if $label$14
               (i32.eqz
                (i32.and
                 (local.tee $4
                  (select
                   (i32.const 1)
                   (local.tee $4
                    (i32.load8_u
                     (i32.add
                      (local.get $1)
                      (i32.const 48)
                     )
                    )
                   )
                   (i32.eq
                    (local.get $4)
                    (i32.const 3)
                   )
                  )
                 )
                 (i32.const 3)
                )
               )
              )
              (br_if $label$13
               (i32.eq
                (local.get $4)
                (i32.const 2)
               )
              )
              (local.set $4
               (i32.const 0)
              )
              (br $label$12)
             )
             (local.set $5
              (local.get $4)
             )
             (br $label$15
              (i32.const 0)
             )
            )
            (local.set $5
             (i32.shr_u
              (i32.add
               (local.get $4)
               (i32.const 1)
              )
              (i32.const 1)
             )
            )
            (i32.shr_u
             (local.get $4)
             (i32.const 1)
            )
           )
          )
          (local.set $2
           (i32.const -1)
          )
          (local.set $4
           (i32.add
            (local.get $1)
            (i32.const 4)
           )
          )
          (local.set $11
           (i32.add
            (local.get $1)
            (i32.const 24)
           )
          )
          (local.set $12
           (i32.add
            (local.get $1)
            (i32.const 28)
           )
          )
          (block $label$21
           (loop $label$22
            (br_if $label$21
             (i32.ge_u
              (local.tee $2
               (i32.add
                (local.get $2)
                (i32.const 1)
               )
              )
              (local.get $3)
             )
            )
            (br_if $label$22
             (i32.eqz
              (call_indirect (type $1)
               (i32.load
                (local.get $11)
               )
               (i32.load
                (local.get $4)
               )
               (i32.load offset=16
                (i32.load
                 (local.get $12)
                )
               )
              )
             )
            )
           )
           (br $label$7)
          )
          (local.set $4
           (i32.load
            (i32.add
             (local.get $1)
             (i32.const 4)
            )
           )
          )
          (local.set $3
           (i32.const 1)
          )
          (br_if $label$9
           (call $9
            (local.get $1)
            (local.get $8)
            (local.get $9)
           )
          )
          (br_if $label$9
           (call_indirect (type $0)
            (i32.load
             (local.tee $2
              (i32.add
               (local.get $1)
               (i32.const 24)
              )
             )
            )
            (local.get $10)
            (local.get $7)
            (i32.load offset=12
             (i32.load
              (local.tee $1
               (i32.add
                (local.get $1)
                (i32.const 28)
               )
              )
             )
            )
           )
          )
          (local.set $7
           (i32.load
            (local.get $2)
           )
          )
          (local.set $2
           (i32.const -1)
          )
          (local.set $1
           (i32.add
            (i32.load
             (local.get $1)
            )
            (i32.const 16)
           )
          )
          (loop $label$23
           (br_if $label$11
            (i32.ge_u
             (local.tee $2
              (i32.add
               (local.get $2)
               (i32.const 1)
              )
             )
             (local.get $5)
            )
           )
           (br_if $label$23
            (i32.eqz
             (call_indirect (type $1)
              (local.get $7)
              (local.get $4)
              (i32.load
               (local.get $1)
              )
             )
            )
           )
          )
          (br $label$9)
         )
         (local.set $4
          (local.get $3)
         )
         (local.set $3
          (i32.const 0)
         )
         (br $label$12)
        )
        (local.set $4
         (i32.shr_u
          (i32.add
           (local.get $3)
           (i32.const 1)
          )
          (i32.const 1)
         )
        )
        (local.set $3
         (i32.shr_u
          (local.get $3)
          (i32.const 1)
         )
        )
       )
       (local.set $2
        (i32.const -1)
       )
       (local.set $5
        (i32.add
         (local.get $1)
         (i32.const 4)
        )
       )
       (local.set $8
        (i32.add
         (local.get $1)
         (i32.const 24)
        )
       )
       (local.set $9
        (i32.add
         (local.get $1)
         (i32.const 28)
        )
       )
       (block $label$24
        (loop $label$25
         (br_if $label$24
          (i32.ge_u
           (local.tee $2
            (i32.add
             (local.get $2)
             (i32.const 1)
            )
           )
           (local.get $3)
          )
         )
         (br_if $label$25
          (i32.eqz
           (call_indirect (type $1)
            (i32.load
             (local.get $8)
            )
            (i32.load
             (local.get $5)
            )
            (i32.load offset=16
             (i32.load
              (local.get $9)
             )
            )
           )
          )
         )
        )
        (br $label$7)
       )
       (local.set $5
        (i32.load
         (i32.add
          (local.get $1)
          (i32.const 4)
         )
        )
       )
       (local.set $3
        (i32.const 1)
       )
       (br_if $label$9
        (call_indirect (type $0)
         (i32.load
          (local.tee $2
           (i32.add
            (local.get $1)
            (i32.const 24)
           )
          )
         )
         (local.get $10)
         (local.get $7)
         (i32.load offset=12
          (i32.load
           (local.tee $1
            (i32.add
             (local.get $1)
             (i32.const 28)
            )
           )
          )
         )
        )
       )
       (local.set $7
        (i32.load
         (local.get $2)
        )
       )
       (local.set $2
        (i32.const -1)
       )
       (local.set $1
        (i32.add
         (i32.load
          (local.get $1)
         )
         (i32.const 16)
        )
       )
       (loop $label$26
        (br_if $label$10
         (i32.ge_u
          (local.tee $2
           (i32.add
            (local.get $2)
            (i32.const 1)
           )
          )
          (local.get $4)
         )
        )
        (br_if $label$26
         (i32.eqz
          (call_indirect (type $1)
           (local.get $7)
           (local.get $5)
           (i32.load
            (local.get $1)
           )
          )
         )
        )
       )
       (br $label$9)
      )
      (global.set $global$0
       (i32.add
        (local.get $6)
        (i32.const 48)
       )
      )
      (return
       (i32.const 0)
      )
     )
     (local.set $3
      (i32.const 0)
     )
    )
    (global.set $global$0
     (i32.add
      (local.get $6)
      (i32.const 48)
     )
    )
    (return
     (local.get $3)
    )
   )
   (return
    (block (result i32)
     (local.set $15
      (call_indirect (type $0)
       (i32.load offset=24
        (local.get $1)
       )
       (local.get $10)
       (local.get $7)
       (i32.load offset=12
        (i32.load
         (i32.add
          (local.get $1)
          (i32.const 28)
         )
        )
       )
      )
     )
     (global.set $global$0
      (i32.add
       (local.get $6)
       (i32.const 48)
      )
     )
     (local.get $15)
    )
   )
  )
  (global.set $global$0
   (i32.add
    (local.get $6)
    (i32.const 48)
   )
  )
  (i32.const 1)
 )
 (func $7 (; 7 ;) (type $2) (param $0 i32)
  (nop)
 )
 (func $8 (; 8 ;) (type $6) (param $0 i32) (result i64)
  (i64.const -2357177763932378009)
 )
 (func $9 (; 9 ;) (type $9) (param $0 i32) (param $1 i32) (param $2 i32) (result i32)
  (block $label$1
   (return
    (block $label$2 (result i32)
     (if
      (i32.ne
       (local.get $1)
       (i32.const 1114112)
      )
      (drop
       (br_if $label$2
        (i32.const 1)
        (call_indirect (type $1)
         (i32.load offset=24
          (local.get $0)
         )
         (local.get $1)
         (i32.load offset=16
          (i32.load
           (i32.add
            (local.get $0)
            (i32.const 28)
           )
          )
         )
        )
       )
      )
     )
     (br_if $label$1
      (i32.eqz
       (local.get $2)
      )
     )
     (call_indirect (type $0)
      (i32.load offset=24
       (local.get $0)
      )
      (local.get $2)
      (i32.const 0)
      (i32.load offset=12
       (i32.load
        (i32.add
         (local.get $0)
         (i32.const 28)
        )
       )
      )
     )
    )
   )
  )
  (i32.const 0)
 )
 (func $10 (; 10 ;) (type $3) (param $0 i32) (result i32)
  (local $1 i32)
  (local $2 i32)
  (local $3 i32)
  (local $4 i32)
  (local $5 i32)
  (local $6 i32)
  (local $7 i32)
  (local $8 i32)
  (local $9 i32)
  (local $10 i32)
  (local $11 i32)
  (local $12 i32)
  (local $13 i32)
  (local $14 i32)
  (local $15 i32)
  (local $16 i32)
  (local $17 i32)
  (local $18 i32)
  (local $19 i32)
  (local $20 i32)
  (local $21 i32)
  (local $22 i32)
  (local $23 i32)
  (local $24 i32)
  (local $25 i32)
  (local $26 i32)
  (local $27 i32)
  (local $28 i32)
  (local $29 i32)
  (local $30 i32)
  (local $31 i32)
  (local $32 i32)
  (local $33 i32)
  (local $34 i32)
  (local $35 i32)
  (local $36 i32)
  (local $37 i32)
  (local $38 i32)
  (local $39 i32)
  (local $40 i32)
  (local $41 i32)
  (local $42 i32)
  (local $43 i32)
  (local $44 i32)
  (local $45 i32)
  (local $46 i32)
  (global.set $global$0
   (local.tee $1
    (i32.sub
     (global.get $global$0)
     (i32.const 256)
    )
   )
  )
  (i64.store offset=56 align=4
   (local.get $1)
   (i64.const 4294967297)
  )
  (i64.store offset=48 align=4
   (local.get $1)
   (i64.const 4294967297)
  )
  (i64.store offset=40 align=4
   (local.get $1)
   (i64.const 4294967297)
  )
  (i64.store offset=32 align=4
   (local.get $1)
   (i64.const 4294967297)
  )
  (i64.store offset=24 align=4
   (local.get $1)
   (i64.const 4294967297)
  )
  (i64.store offset=16 align=4
   (local.get $1)
   (i64.const 4294967297)
  )
  (i64.store offset=8 align=4
   (local.get $1)
   (i64.const 4294967297)
  )
  (i64.store align=4
   (local.get $1)
   (i64.const 4294967297)
  )
  (block $label$1
   (if
    (i32.ge_u
     (local.tee $11
      (i32.add
       (local.get $0)
       (i32.const 1)
      )
     )
     (i32.const 2)
    )
    (block
     (local.set $3
      (local.get $1)
     )
     (local.set $2
      (i32.const 1)
     )
     (loop $label$3
      (br_if $label$1
       (i32.ge_u
        (local.get $2)
        (i32.const 16)
       )
      )
      (i32.store
       (local.tee $4
        (i32.add
         (local.get $3)
         (i32.const 4)
        )
       )
       (i32.mul
        (i32.load
         (local.get $3)
        )
        (local.get $2)
       )
      )
      (local.set $3
       (local.get $4)
      )
      (local.set $2
       (local.tee $4
        (i32.add
         (local.get $2)
         (i32.const 1)
        )
       )
      )
      (br_if $label$3
       (i32.lt_u
        (local.get $4)
        (local.get $11)
       )
      )
     )
    )
   )
   (if
    (i32.lt_u
     (local.get $0)
     (i32.const 16)
    )
    (block
     (local.set $20
      (i32.const 1)
     )
     (local.set $21
      (local.tee $9
       (i32.load
        (i32.add
         (local.get $1)
         (i32.shl
          (local.get $0)
          (i32.const 2)
         )
        )
       )
      )
     )
     (if
      (i32.ge_u
       (local.get $9)
       (i32.const 24)
      )
      (local.set $20
       (select
        (i32.const 24)
        (i32.const 25)
        (i32.eq
         (local.get $9)
         (i32.mul
          (local.tee $21
           (i32.div_u
            (local.get $9)
            (i32.const 24)
           )
          )
          (i32.const 24)
         )
        )
       )
      )
     )
     (local.set $40
      (i32.sub
       (i32.const 0)
       (local.get $0)
      )
     )
     (local.set $12
      (i32.add
       (local.get $1)
       (i32.const 196)
      )
     )
     (local.set $41
      (i32.add
       (local.get $1)
       (i32.const 132)
      )
     )
     (local.set $42
      (i32.add
       (local.get $1)
       (i32.const 124)
      )
     )
     (local.set $11
      (i32.add
       (local.get $1)
       (i32.const 68)
      )
     )
     (local.set $43
      (i32.lt_u
       (local.get $0)
       (i32.const 2)
      )
     )
     (loop $label$6
      (i64.store
       (i32.add
        (local.get $1)
        (i32.const 120)
       )
       (i64.const 0)
      )
      (i64.store
       (i32.add
        (local.get $1)
        (i32.const 112)
       )
       (i64.const 0)
      )
      (i64.store
       (i32.add
        (local.get $1)
        (i32.const 104)
       )
       (i64.const 0)
      )
      (i64.store
       (i32.add
        (local.get $1)
        (i32.const 96)
       )
       (i64.const 0)
      )
      (i64.store
       (i32.add
        (local.get $1)
        (i32.const 88)
       )
       (i64.const 0)
      )
      (i64.store
       (i32.add
        (local.get $1)
        (i32.const 80)
       )
       (i64.const 0)
      )
      (i64.store
       (i32.add
        (local.get $1)
        (i32.const 72)
       )
       (i64.const 0)
      )
      (i64.store offset=64
       (local.get $1)
       (i64.const 0)
      )
      (i64.store
       (local.tee $26
        (i32.add
         (local.get $1)
         (i32.const 184)
        )
       )
       (i64.const 0)
      )
      (i64.store
       (local.tee $27
        (i32.add
         (local.get $1)
         (i32.const 176)
        )
       )
       (i64.const 0)
      )
      (i64.store
       (local.tee $28
        (i32.add
         (local.get $1)
         (i32.const 168)
        )
       )
       (i64.const 0)
      )
      (i64.store
       (local.tee $29
        (i32.add
         (local.get $1)
         (i32.const 160)
        )
       )
       (i64.const 0)
      )
      (i64.store
       (local.tee $30
        (i32.add
         (local.get $1)
         (i32.const 152)
        )
       )
       (i64.const 0)
      )
      (i64.store
       (local.tee $31
        (i32.add
         (local.get $1)
         (i32.const 144)
        )
       )
       (i64.const 0)
      )
      (i64.store
       (local.tee $32
        (i32.add
         (local.get $1)
         (i32.const 136)
        )
       )
       (i64.const 0)
      )
      (i64.store offset=128
       (local.get $1)
       (i64.const 0)
      )
      (i64.store align=4
       (local.tee $33
        (i32.add
         (local.get $1)
         (i32.const 248)
        )
       )
       (i64.const 64424509454)
      )
      (i64.store align=4
       (local.tee $34
        (i32.add
         (local.get $1)
         (i32.const 240)
        )
       )
       (i64.const 55834574860)
      )
      (i64.store align=4
       (local.tee $35
        (i32.add
         (local.get $1)
         (i32.const 232)
        )
       )
       (i64.const 47244640266)
      )
      (i64.store align=4
       (local.tee $36
        (i32.add
         (local.get $1)
         (i32.const 224)
        )
       )
       (i64.const 38654705672)
      )
      (i64.store align=4
       (local.tee $37
        (i32.add
         (local.get $1)
         (i32.const 216)
        )
       )
       (i64.const 30064771078)
      )
      (i64.store align=4
       (local.tee $38
        (i32.add
         (local.get $1)
         (i32.const 208)
        )
       )
       (i64.const 21474836484)
      )
      (i64.store align=4
       (local.tee $39
        (i32.add
         (local.get $1)
         (i32.const 200)
        )
       )
       (i64.const 12884901890)
      )
      (i64.store offset=192 align=4
       (local.get $1)
       (i64.const 4294967296)
      )
      (local.set $7
       (i32.mul
        (local.get $13)
        (local.get $21)
       )
      )
      (local.set $2
       (block $label$7 (result i32)
        (block $label$8
         (if
          (i32.eqz
           (local.get $43)
          )
          (block
           (local.set $23
            (local.get $40)
           )
           (local.set $14
            (local.get $7)
           )
           (local.set $15
            (local.get $0)
           )
           (local.set $5
            (i32.const 0)
           )
           (br $label$8)
          )
         )
         (br $label$7
          (i32.const 0)
         )
        )
        (i32.const 1)
       )
      )
      (loop $label$10
       (block $label$11
        (block $label$12
         (local.set $2
          (block $label$13 (result i32)
           (block $label$14
            (block $label$15
             (block $label$16
              (block $label$17
               (block $label$18
                (block $label$19
                 (if
                  (i32.eqz
                   (local.get $2)
                  )
                  (block
                   (local.set $13
                    (i32.add
                     (local.get $13)
                     (i32.const 1)
                    )
                   )
                   (local.set $44
                    (i32.add
                     (select
                      (local.get $9)
                      (local.tee $3
                       (i32.add
                        (local.get $7)
                        (local.get $21)
                       )
                      )
                      (i32.gt_u
                       (local.get $3)
                       (local.get $9)
                      )
                     )
                     (i32.const -1)
                    )
                   )
                   (local.set $24
                    (i32.const 0)
                   )
                   (br_if $label$19
                    (i32.ge_s
                     (local.tee $6
                      (i32.load offset=192
                       (local.get $1)
                      )
                     )
                     (i32.const 1)
                    )
                   )
                   (br $label$18)
                  )
                 )
                 (block $label$21
                  (block $label$22
                   (block $label$23
                    (block $label$24
                     (block $label$25
                      (block $label$26
                       (block $label$27
                        (block $label$28
                         (block $label$29
                          (block $label$30
                           (block $label$31
                            (br_table $label$31 $label$30 $label$29
                             (local.get $5)
                            )
                           )
                           (br_if $label$24
                            (i32.ge_u
                             (local.tee $4
                              (i32.add
                               (local.get $15)
                               (i32.const -1)
                              )
                             )
                             (i32.const 16)
                            )
                           )
                           (br_if $label$23
                            (i32.eqz
                             (local.tee $3
                              (i32.load
                               (i32.add
                                (local.get $1)
                                (local.tee $2
                                 (i32.shl
                                  (local.get $4)
                                  (i32.const 2)
                                 )
                                )
                               )
                              )
                             )
                            )
                           )
                           (if
                            (i32.eq
                             (local.get $14)
                             (i32.const -2147483648)
                            )
                            (br_if $label$22
                             (i32.eq
                              (local.get $3)
                              (i32.const -1)
                             )
                            )
                           )
                           (i32.store
                            (i32.add
                             (i32.sub
                              (local.get $1)
                              (i32.const -64)
                             )
                             (local.get $2)
                            )
                            (local.tee $16
                             (i32.div_s
                              (local.get $14)
                              (local.get $3)
                             )
                            )
                           )
                           (i64.store
                            (local.get $32)
                            (i64.load align=4
                             (local.get $39)
                            )
                           )
                           (i64.store
                            (local.get $31)
                            (i64.load align=4
                             (local.get $38)
                            )
                           )
                           (i64.store
                            (local.get $30)
                            (i64.load align=4
                             (local.get $37)
                            )
                           )
                           (i64.store
                            (local.get $29)
                            (i64.load align=4
                             (local.get $36)
                            )
                           )
                           (i64.store
                            (local.get $28)
                            (i64.load align=4
                             (local.get $35)
                            )
                           )
                           (i64.store
                            (local.get $27)
                            (i64.load align=4
                             (local.get $34)
                            )
                           )
                           (i64.store
                            (local.get $26)
                            (i64.load align=4
                             (local.get $33)
                            )
                           )
                           (i64.store offset=128
                            (local.get $1)
                            (i64.load offset=192 align=4
                             (local.get $1)
                            )
                           )
                           (local.set $45
                            (i32.add
                             (local.get $16)
                             (local.get $23)
                            )
                           )
                           (local.set $14
                            (i32.sub
                             (local.get $14)
                             (i32.mul
                              (local.get $3)
                              (local.get $16)
                             )
                            )
                           )
                           (local.set $2
                            (i32.const 0)
                           )
                           (local.set $8
                            (i32.add
                             (local.get $1)
                             (i32.const 192)
                            )
                           )
                           (loop $label$33
                            (block $label$34
                             (if
                              (i32.gt_u
                               (local.tee $3
                                (i32.add
                                 (local.get $2)
                                 (local.get $16)
                                )
                               )
                               (local.get $4)
                              )
                              (block
                               (br_if $label$27
                                (i32.gt_u
                                 (local.tee $46
                                  (i32.add
                                   (local.get $2)
                                   (local.get $45)
                                  )
                                 )
                                 (i32.const 15)
                                )
                               )
                               (local.set $3
                                (i32.sub
                                 (local.get $3)
                                 (local.get $15)
                                )
                               )
                               (br_if $label$34
                                (i32.le_u
                                 (local.get $2)
                                 (i32.const 15)
                                )
                               )
                               (br $label$28)
                              )
                             )
                             (br_if $label$26
                              (i32.ge_u
                               (local.get $3)
                               (i32.const 16)
                              )
                             )
                             (br_if $label$28
                              (i32.gt_u
                               (local.get $2)
                               (i32.const 15)
                              )
                             )
                            )
                            (i32.store
                             (local.get $8)
                             (i32.load
                              (i32.add
                               (i32.add
                                (local.get $1)
                                (i32.const 128)
                               )
                               (i32.shl
                                (local.get $3)
                                (i32.const 2)
                               )
                              )
                             )
                            )
                            (local.set $8
                             (i32.add
                              (local.get $8)
                              (i32.const 4)
                             )
                            )
                            (br_if $label$33
                             (i32.lt_u
                              (local.tee $2
                               (i32.add
                                (local.get $2)
                                (i32.const 1)
                               )
                              )
                              (local.get $15)
                             )
                            )
                           )
                           (local.set $23
                            (i32.add
                             (local.get $23)
                             (i32.const 1)
                            )
                           )
                           (br_if $label$21
                            (i32.gt_u
                             (local.tee $15
                              (local.get $4)
                             )
                             (i32.const 1)
                            )
                           )
                           (local.set $2
                            (i32.const 0)
                           )
                           (br $label$10)
                          )
                          (i64.store
                           (local.get $26)
                           (i64.load align=4
                            (local.get $33)
                           )
                          )
                          (i64.store
                           (local.get $27)
                           (i64.load align=4
                            (local.get $34)
                           )
                          )
                          (i64.store
                           (local.get $28)
                           (i64.load align=4
                            (local.get $35)
                           )
                          )
                          (i64.store
                           (local.get $29)
                           (i64.load align=4
                            (local.get $36)
                           )
                          )
                          (i64.store
                           (local.get $30)
                           (i64.load align=4
                            (local.get $37)
                           )
                          )
                          (i64.store
                           (local.get $31)
                           (i64.load align=4
                            (local.get $38)
                           )
                          )
                          (i64.store
                           (local.get $32)
                           (i64.load align=4
                            (local.get $39)
                           )
                          )
                          (i64.store offset=128
                           (local.get $1)
                           (i64.load offset=192 align=4
                            (local.get $1)
                           )
                          )
                          (br_if $label$25
                           (i32.gt_u
                            (local.get $6)
                            (i32.const 15)
                           )
                          )
                          (local.set $17
                           (i32.const 1)
                          )
                          (local.set $10
                           (local.get $6)
                          )
                          (br $label$13
                           (i32.const 0)
                          )
                         )
                         (if
                          (i32.lt_u
                           (local.get $7)
                           (local.get $44)
                          )
                          (block
                           (local.set $25
                            (i32.load
                             (local.get $12)
                            )
                           )
                           (i32.store
                            (local.get $12)
                            (local.get $6)
                           )
                           (i32.store offset=192
                            (local.get $1)
                            (local.get $25)
                           )
                           (local.set $18
                            (local.get $11)
                           )
                           (br_if $label$11
                            (i32.lt_s
                             (local.tee $2
                              (i32.load offset=68
                               (local.get $1)
                              )
                             )
                             (i32.const 1)
                            )
                           )
                           (local.set $19
                            (i32.const 1)
                           )
                           (br $label$14)
                          )
                         )
                         (local.set $22
                          (i32.add
                           (local.get $22)
                           (local.get $24)
                          )
                         )
                         (br_if $label$6
                          (i32.lt_u
                           (local.get $13)
                           (local.get $20)
                          )
                         )
                         (global.set $global$0
                          (i32.add
                           (local.get $1)
                           (i32.const 256)
                          )
                         )
                         (return
                          (local.get $22)
                         )
                        )
                        (call $3
                         (i32.const 1049076)
                         (local.get $2)
                        )
                        (unreachable)
                       )
                       (call $3
                        (i32.const 1049060)
                        (local.get $46)
                       )
                       (unreachable)
                      )
                      (call $3
                       (i32.const 1049044)
                       (i32.add
                        (local.get $2)
                        (local.get $16)
                       )
                      )
                      (unreachable)
                     )
                     (local.set $10
                      (local.get $6)
                     )
                     (br $label$12)
                    )
                    (call $3
                     (i32.const 1048980)
                     (local.get $4)
                    )
                    (unreachable)
                   )
                   (call $2
                    (i32.const 1048996)
                   )
                   (unreachable)
                  )
                  (call $2
                   (i32.const 1049020)
                  )
                  (unreachable)
                 )
                 (local.set $5
                  (i32.const 0)
                 )
                 (br $label$17)
                )
                (local.set $5
                 (i32.const 1)
                )
                (br $label$16)
               )
               (local.set $5
                (i32.const 2)
               )
               (br $label$15)
              )
              (local.set $2
               (i32.const 1)
              )
              (br $label$10)
             )
             (local.set $2
              (i32.const 1)
             )
             (br $label$10)
            )
            (local.set $2
             (i32.const 1)
            )
            (br $label$10)
           )
           (i32.const 1)
          )
         )
         (loop $label$37
          (block $label$38
           (block $label$39
            (if
             (i32.eqz
              (local.get $2)
             )
             (block
              (if
               (local.tee $10
                (i32.load
                 (local.tee $5
                  (i32.add
                   (local.tee $4
                    (i32.shl
                     (local.tee $3
                      (local.get $10)
                     )
                     (i32.const 2)
                    )
                   )
                   (i32.add
                    (local.get $1)
                    (i32.const 128)
                   )
                  )
                 )
                )
               )
               (block
                (i32.store
                 (local.get $5)
                 (local.get $3)
                )
                (block $label$42
                 (br_if $label$42
                  (i32.lt_u
                   (local.get $3)
                   (i32.const 3)
                  )
                 )
                 (br_if $label$42
                  (i32.eqz
                   (local.tee $8
                    (i32.shr_u
                     (i32.add
                      (local.get $3)
                      (i32.const -1)
                     )
                     (i32.const 1)
                    )
                   )
                  )
                 )
                 (local.set $2
                  (i32.add
                   (local.get $4)
                   (local.get $42)
                  )
                 )
                 (local.set $3
                  (local.get $41)
                 )
                 (loop $label$43
                  (local.set $4
                   (i32.load
                    (local.get $3)
                   )
                  )
                  (i32.store
                   (local.get $3)
                   (i32.load
                    (local.get $2)
                   )
                  )
                  (i32.store
                   (local.get $2)
                   (local.get $4)
                  )
                  (local.set $3
                   (i32.add
                    (local.get $3)
                    (i32.const 4)
                   )
                  )
                  (local.set $2
                   (i32.add
                    (local.get $2)
                    (i32.const -4)
                   )
                  )
                  (br_if $label$43
                   (local.tee $8
                    (i32.add
                     (local.get $8)
                     (i32.const -1)
                    )
                   )
                  )
                 )
                )
                (local.set $17
                 (i32.add
                  (local.get $17)
                  (i32.const 1)
                 )
                )
                (br_if $label$38
                 (i32.lt_u
                  (local.get $10)
                  (i32.const 16)
                 )
                )
                (br $label$12)
               )
              )
              (local.set $24
               (i32.add
                (select
                 (i32.sub
                  (i32.const 0)
                  (local.get $17)
                 )
                 (local.get $17)
                 (i32.and
                  (local.get $7)
                  (i32.const 1)
                 )
                )
                (local.get $24)
               )
              )
              (local.set $5
               (i32.const 2)
              )
              (br $label$39)
             )
            )
            (local.set $2
             (i32.const 0)
            )
            (i32.store
             (local.get $18)
             (i32.const 0)
            )
            (i32.store offset=192
             (local.get $1)
             (local.tee $4
              (local.get $6)
             )
            )
            (local.set $5
             (i32.add
              (local.get $19)
              (i32.const 1)
             )
            )
            (local.set $3
             (local.get $12)
            )
            (block $label$44
             (block $label$45
              (loop $label$46
               (br_if $label$45
                (i32.ge_u
                 (i32.add
                  (local.get $2)
                  (i32.const 2)
                 )
                 (i32.const 16)
                )
               )
               (i32.store
                (local.get $3)
                (i32.load
                 (local.tee $3
                  (i32.add
                   (local.get $3)
                   (i32.const 4)
                  )
                 )
                )
               )
               (br_if $label$46
                (i32.lt_u
                 (local.tee $2
                  (i32.add
                   (local.get $2)
                   (i32.const 1)
                  )
                 )
                 (local.get $19)
                )
               )
              )
              (br_if $label$44
               (i32.ge_u
                (local.get $5)
                (i32.const 16)
               )
              )
              (i32.store
               (i32.add
                (local.tee $3
                 (i32.shl
                  (local.get $5)
                  (i32.const 2)
                 )
                )
                (i32.add
                 (local.get $1)
                 (i32.const 192)
                )
               )
               (local.get $25)
              )
              (br_if $label$11
               (i32.le_s
                (local.tee $2
                 (i32.load
                  (local.tee $18
                   (i32.add
                    (i32.sub
                     (local.get $1)
                     (i32.const -64)
                    )
                    (local.get $3)
                   )
                  )
                 )
                )
                (local.get $19)
               )
              )
              (local.set $6
               (i32.load
                (local.get $12)
               )
              )
              (local.set $19
               (local.get $5)
              )
              (local.set $25
               (local.get $4)
              )
              (local.set $2
               (i32.const 1)
              )
              (br $label$37)
             )
             (call $3
              (i32.const 1049108)
              (i32.add
               (local.get $2)
               (i32.const 2)
              )
             )
             (unreachable)
            )
            (call $3
             (i32.const 1049124)
             (local.get $5)
            )
            (unreachable)
           )
           (local.set $2
            (i32.const 1)
           )
           (br $label$10)
          )
          (local.set $2
           (i32.const 0)
          )
          (br $label$37)
         )
        )
        (call $3
         (i32.const 1049092)
         (local.get $10)
        )
        (unreachable)
       )
       (local.set $7
        (i32.add
         (local.get $7)
         (i32.const 1)
        )
       )
       (i32.store
        (local.get $18)
        (i32.add
         (local.get $2)
         (i32.const 1)
        )
       )
       (block $label$47
        (block $label$48
         (if
          (i32.ge_s
           (local.tee $6
            (i32.load offset=192
             (local.get $1)
            )
           )
           (i32.const 1)
          )
          (block
           (local.set $5
            (i32.const 1)
           )
           (br $label$48)
          )
         )
         (local.set $5
          (i32.const 2)
         )
         (br $label$47)
        )
        (local.set $2
         (i32.const 1)
        )
        (br $label$10)
       )
       (local.set $2
        (i32.const 1)
       )
       (br $label$10)
      )
     )
    )
   )
   (call $3
    (i32.const 1049212)
    (local.get $0)
   )
   (unreachable)
  )
  (call $3
   (i32.const 1049196)
   (local.get $2)
  )
  (unreachable)
 )
)

