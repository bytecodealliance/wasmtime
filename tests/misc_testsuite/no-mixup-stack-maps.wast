(module
  (global $g (mut externref) (ref.null extern))

  ;; This function will have a stack map, notably one that's a bit
  ;; different than the one below.
  (func $has_a_stack_map
      (local externref)
      global.get $g
      local.tee 0
      global.set $g

      local.get 0
      global.set $g
      ref.null extern
      global.set $g
  )

  ;; This function also has a stack map, but it's only applicable after
  ;; the call to the `$gc` import, so when we gc during that we shouldn't
  ;; accidentally read the previous function's stack maps and use that
  ;; for our own.
  (func (export "run") (result i32)
      call $gc

      ref.null extern
      global.set $g
      i32.const 0
  )

  (func (export "init") (param externref)
      local.get 0
      global.set $g
  )

  ;; A small function which when run triggers a gc in wasmtime
  (func $gc
    (local $i i32)
    i32.const 10000
    local.set $i
    (loop $continue
      (global.set $g (global.get $g))
      (local.tee $i (i32.sub (local.get $i) (i32.const 1)))
      br_if $continue
    )
  )
)

(invoke "init" (ref.extern 1))
(assert_return (invoke "run") (i32.const 0))
