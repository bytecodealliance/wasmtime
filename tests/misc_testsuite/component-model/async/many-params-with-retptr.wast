;;! component_model_async = true
;;! reference_types = true
;;! gc_types = true
;;! multi_memory = true

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
        (memory $libc "memory")
        (realloc (func $libc "realloc"))
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
        (memory $libc "memory")
        (realloc (func $libc "realloc"))
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
        (memory $libc "memory")
        (realloc (func $libc "realloc"))
        string-encoding=utf8
        async
        (callback (func $i "callback"))
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
