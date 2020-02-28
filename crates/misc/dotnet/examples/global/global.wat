(module
  (import "" "print_global" (func $.print_global))
  (import "" "global" (global $.global (mut i32)))
  (func $run (param i32) (local $i i32)
    loop $l1
      call $.print_global
      global.get $.global
      i32.const 2
      i32.mul
      global.set $.global
      local.get $i
      i32.const 1
      i32.add
      local.set $i
      local.get $i
      local.get 0
      i32.le_u
      br_if $l1
    end
  )
  (export "run" (func $run))
)
