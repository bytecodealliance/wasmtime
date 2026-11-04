;;! component_model_async = true
;;! reference_types = true
;;! multi_memory = true
;;! bulk_memory = true

;; This test (which was generated during fuzzing) composes a sync lower with an
;; async lift such that the maximum number of flat parameters _and_ a return
;; pointer are needed.
(component
  (core module $libc (;0;)
    (type $#type0 (;0;) (func (param i32 i32 i32 i32) (result i32)))
    (memory $#memory0 (;0;) 1)
    (global $last (;0;) (mut i32) i32.const 8)
    (export "memory" (memory $#memory0))
    (export "realloc" (func $realloc))
    (func $realloc (;0;) (type $#type0) (param $old_ptr i32) (param $old_size i32) (param $align i32) (param $new_size i32) (result i32)
      (local $ret i32)
      local.get $old_ptr
      if $#label0
        local.get $old_size
        local.get $new_size
        i32.gt_u
        if $#label1
          local.get $old_ptr
          return
        end
      end
      global.get $last
      local.get $align
      i32.const -1
      i32.add
      i32.add
      local.get $align
      i32.const -1
      i32.add
      i32.const -1
      i32.xor
      i32.and
      global.set $last
      global.get $last
      local.set $ret
      global.get $last
      local.get $new_size
      i32.add
      global.set $last
      loop $loop
        memory.size
        i32.const 65536
        i32.mul
        global.get $last
        i32.lt_u
        if $#label1
          i32.const 1
          memory.grow
          i32.const -1
          i32.eq
          if $#label2
            unreachable
          end
          br $loop
        end
      end
      local.get $ret
      i32.const 222
      local.get $new_size
      memory.fill
      local.get $old_ptr
      if $#label0
        local.get $ret
        local.get $old_ptr
        local.get $old_size
        memory.copy
      end
      local.get $ret
    )
  )

  (component $caller
    (type $t5 (list u16))
    (type $t0 (tuple $t5 string string bool s16))
    (type $sig (func async (param "p0" $t0) (param "p1" $t0) (result $t0)))
    (import "echo-import" (func $f (type $sig)))

    (core instance $libc (instantiate $libc))
    (core func $f_lower
      (canon lower
        (func $f)
        (memory (core memory $libc "memory"))
        (realloc (core func $libc "realloc"))
        string-encoding=latin1+utf16
      )
    )

    (core module $m
      (type $import (func (param i32 i32 i32 i32 i32 i32 i32 i32 i32 i32 i32 i32 i32 i32 i32 i32 i32)))
      (type $#type2 (func (param i32 i32 i32 i32) (result i32)))
      (import "libc" "realloc" (func $realloc (type $#type2)))
      (import "host" "echo-import" (func $host (type $import)))
      (func (export "echo-export") (param $retptr i32) (param $argptr i32) (param $#local2 i32) (param $#local3 i32) (param $#local4 i32) (param $#local5 i32) (param $#local6 i32) (param $#local7 i32) (param $#local8 i32) (param $#local9 i32) (param $#local10 i32) (param $#local11 i32) (param $#local12 i32) (param $#local13 i32) (param $#local14 i32) (param $#local15 i32) (result i32)
        (local $#local16 i32) (local $#local17 i32)
        local.get $retptr
        local.get $argptr
        local.get $#local2
        local.get $#local3
        local.get $#local4
        local.get $#local5
        local.get $#local6
        local.get $#local7
        local.get $#local8
        local.get $#local9
        local.get $#local10
        local.get $#local11
        local.get $#local12
        local.get $#local13
        local.get $#local14
        local.get $#local15
        i32.const 0
        i32.const 0
        i32.const 4
        i32.const 28
        call $realloc
        local.set $#local16
        local.get $#local16
        call $host
        local.get $#local16
      )
    )
    (core instance $i (instantiate $m
      (with "libc" (instance $libc))
      (with "host" (instance (export "echo-import" (func $f_lower))))
    ))
    (func (export "echo-export") (type $sig)
      (canon lift
        (core func $i "echo-export")
        (memory (core memory $libc "memory"))
        (realloc (core func $libc "realloc"))
        string-encoding=latin1+utf16)
    )
  )

  (component $callee
    (type $t0 (tuple (list u16) string string bool s16))
    (type $export_sig (func async (param "p0" $t0) (param "p1" $t0) (result $t0)))
    (core instance $libc (instantiate $libc))
    (core module $m
      (func (export "echo-export") (param i32 i32 i32 i32 i32 i32 i32 i32 i32 i32 i32 i32 i32 i32 i32 i32) (result i32)
        unreachable
      )
      (func (export "callback") (param i32 i32 i32) (result i32)
        unreachable
      )
    )
    (core instance $i (;3;) (instantiate $m
        (with "libc" (instance $libc))
      )
    )
    (func (export "echo-export") (type $export_sig)
      (canon lift
        (core func $i "echo-export")
        (memory (core memory $libc "memory"))
        (realloc (core func $libc "realloc"))
        string-encoding=utf8
        async
        (callback (core func $i "callback"))
      )
    )
  )

  (instance $c1 (instantiate $callee))
  (instance $c2 (instantiate $caller
    (with "echo-import" (func $c1 "echo-export"))
  ))
  (export "echo-export" (func $c2 "echo-export"))
)

(assert_trap
  (invoke "echo-export"
    (tuple.const
      (list.const)
      (str.const "")
      (str.const "")
      (bool.const false)
      (s16.const 0)
    )
    (tuple.const
      (list.const )
      (str.const "")
      (str.const "")
      (bool.const false)
      (s16.const 0)
    )
  )
  "unreachable")

;; Many parameters of an `async`-typed function lifted without `async`, which
;; are passed in memory. The callee checks the in-memory layout of the
;; parameters and then returns the same memory as its result.
(component
  (core module $m
    (memory (export "memory") 1)
    (global $last (mut i32) (i32.const 8))
    (func (export "realloc") (param i32 i32 i32 i32) (result i32)
      (local $ret i32)
      (local.set $ret
        (i32.and
          (i32.add (global.get $last) (i32.sub (local.get 2) (i32.const 1)))
          (i32.sub (i32.const 0) (local.get 2))))
      (global.set $last (i32.add (local.get $ret) (local.get 3)))
      (local.get $ret))

    (func (export "foo") (param $p i32) (result i32)
      (if (i32.ne (i32.load8_s offset=0 (local.get $p)) (i32.const -100)) (then unreachable))
      (if (i64.ne (i64.load offset=8 (local.get $p)) (i64.const 0x7fffffffffffffff))
        (then unreachable))
      (if (i32.ne (i32.load offset=16 (local.get $p)) (i32.const 0x7fc00001)) (then unreachable))
      (if (i32.ne (i32.load8_u offset=20 (local.get $p)) (i32.const 38)) (then unreachable))
      (if (i32.ne (i32.load16_s offset=22 (local.get $p)) (i32.const 18831)) (then unreachable))
      (if (i32.ne (i32.load offset=28 (local.get $p)) (i32.const 24)) (then unreachable))
      (if (i32.ne (i32.load offset=36 (local.get $p)) (i32.const 8)) (then unreachable))
      (if (i32.ne (i32.load8_u offset=40 (local.get $p)) (i32.const 1)) (then unreachable))
      (if (i32.ne (i32.load8_u offset=41 (local.get $p)) (i32.const 0)) (then unreachable))
      (if (i32.ne (i32.load offset=44 (local.get $p)) (i32.const 0x1f6a9)) (then unreachable))
      (if (i32.ne (i32.load offset=52 (local.get $p)) (i32.const 5)) (then unreachable))
      (if (i32.ne (i32.load offset=60 (local.get $p)) (i32.const 5)) (then unreachable))
      (if (i32.ne (i32.load offset=68 (local.get $p)) (i32.const 5)) (then unreachable))
      (local.get $p))
  )
  (core instance $i (instantiate $m))

  (type $tuple (tuple
    s8                ;; offset  0, size 1
    u64               ;; offset  8, size 8
    f32               ;; offset 16, size 4
    u8                ;; offset 20, size 1
    s16               ;; offset 22, size 2
    string            ;; offset 24, size 8
    (list u32)        ;; offset 32, size 8
    bool              ;; offset 40, size 1
    bool              ;; offset 41, size 1
    char              ;; offset 44, size 4
    (list bool)       ;; offset 48, size 8
    (list char)       ;; offset 56, size 8
    (list string)))   ;; offset 64, size 8
  (type $t (func async
    (param "p1" s8)
    (param "p2" u64)
    (param "p3" f32)
    (param "p4" u8)
    (param "p5" s16)
    (param "p6" string)
    (param "p7" (list u32))
    (param "p8" bool)
    (param "p9" bool)
    (param "p0" char)
    (param "pa" (list bool))
    (param "pb" (list char))
    (param "pc" (list string))
    (result $tuple)))
  (func (export "many-param") (type $t)
    (canon lift (core func $i "foo") (memory (core memory $i "memory"))
      (realloc (core func $i "realloc"))))
)

(assert_return
  (invoke "many-param"
    (s8.const -100)
    (u64.const 0x7fffffffffffffff)
    (f32.const nan:0x400001)
    (u8.const 38)
    (s16.const 18831)
    (str.const "this is the first string")
    (list.const
      (u32.const 1) (u32.const 2) (u32.const 3) (u32.const 4)
      (u32.const 5) (u32.const 6) (u32.const 7) (u32.const 8))
    (bool.const true)
    (bool.const false)
    (char.const "🚩")
    (list.const
      (bool.const false) (bool.const true) (bool.const false)
      (bool.const true) (bool.const true))
    (list.const
      (char.const "🍌") (char.const "🥐") (char.const "🍗")
      (char.const "🍙") (char.const "🍡"))
    (list.const
      (str.const "the quick")
      (str.const "brown fox")
      (str.const "was too lazy")
      (str.const "to jump over the dog")
      (str.const "what a demanding dog")))
  (tuple.const
    (s8.const -100)
    (u64.const 0x7fffffffffffffff)
    (f32.const nan:0x400001)
    (u8.const 38)
    (s16.const 18831)
    (str.const "this is the first string")
    (list.const
      (u32.const 1) (u32.const 2) (u32.const 3) (u32.const 4)
      (u32.const 5) (u32.const 6) (u32.const 7) (u32.const 8))
    (bool.const true)
    (bool.const false)
    (char.const "🚩")
    (list.const
      (bool.const false) (bool.const true) (bool.const false)
      (bool.const true) (bool.const true))
    (list.const
      (char.const "🍌") (char.const "🥐") (char.const "🍗")
      (char.const "🍙") (char.const "🍡"))
    (list.const
      (str.const "the quick")
      (str.const "brown fox")
      (str.const "was too lazy")
      (str.const "to jump over the dog")
      (str.const "what a demanding dog"))))

;; Many results of an `async`-typed function lifted without `async`, which are
;; returned through memory.
(component
  (core module $m
    (memory (export "memory") 1)

    ;; Strings and list contents referenced by the result.
    (data (i32.const 100) "abcdef")
    (data (i32.const 112)
      "\01\00\00\00\02\00\00\00\03\00\00\00\04\00\00\00"
      "\05\00\00\00\06\00\00\00\07\00\00\00\08\00\00\00")
    (data (i32.const 144) "\00\01\00\01\01")
    (data (i32.const 152)
      "\4c\f3\01\00" ;; '🍌'
      "\50\f9\01\00" ;; '🥐'
      "\57\f3\01\00" ;; '🍗'
      "\59\f3\01\00" ;; '🍙'
      "\61\f3\01\00" ;; '🍡'
    )
    (data (i32.const 172)
      "\66\00\00\00\02\00\00\00" ;; "cd"
      "\68\00\00\00\02\00\00\00" ;; "ef"
    )

    (func (export "foo") (result i32)
      (local $base i32)
      (local.set $base (i32.const 8))
      (i32.store8 offset=0 (local.get $base) (i32.const -100))
      (i64.store offset=8 (local.get $base) (i64.const 0x7fffffffffffffff))
      (i32.store offset=16 (local.get $base) (i32.const 0x7fc00001))
      (i32.store8 offset=20 (local.get $base) (i32.const 38))
      (i32.store16 offset=22 (local.get $base) (i32.const 18831))
      (i32.store offset=24 (local.get $base) (i32.const 100))
      (i32.store offset=28 (local.get $base) (i32.const 2))
      (i32.store offset=32 (local.get $base) (i32.const 112))
      (i32.store offset=36 (local.get $base) (i32.const 8))
      (i32.store8 offset=40 (local.get $base) (i32.const 1))
      (i32.store8 offset=41 (local.get $base) (i32.const 0))
      (i32.store offset=44 (local.get $base) (i32.const 0x1f6a9)) ;; '🚩'
      (i32.store offset=48 (local.get $base) (i32.const 144))
      (i32.store offset=52 (local.get $base) (i32.const 5))
      (i32.store offset=56 (local.get $base) (i32.const 152))
      (i32.store offset=60 (local.get $base) (i32.const 5))
      (i32.store offset=64 (local.get $base) (i32.const 172))
      (i32.store offset=68 (local.get $base) (i32.const 2))
      (local.get $base))
  )
  (core instance $i (instantiate $m))

  (type $tuple (tuple
    s8
    u64
    f32
    u8
    s16
    string
    (list u32)
    bool
    bool
    char
    (list bool)
    (list char)
    (list string)))
  (type $t (func async (result $tuple)))
  (func (export "many-results") (type $t)
    (canon lift (core func $i "foo") (memory (core memory $i "memory"))))
)

(assert_return (invoke "many-results")
  (tuple.const
    (s8.const -100)
    (u64.const 0x7fffffffffffffff)
    (f32.const nan:0x400001)
    (u8.const 38)
    (s16.const 18831)
    (str.const "ab")
    (list.const
      (u32.const 1) (u32.const 2) (u32.const 3) (u32.const 4)
      (u32.const 5) (u32.const 6) (u32.const 7) (u32.const 8))
    (bool.const true)
    (bool.const false)
    (char.const "🚩")
    (list.const
      (bool.const false) (bool.const true) (bool.const false)
      (bool.const true) (bool.const true))
    (list.const
      (char.const "🍌") (char.const "🥐") (char.const "🍗")
      (char.const "🍙") (char.const "🍡"))
    (list.const (str.const "cd") (str.const "ef"))))
