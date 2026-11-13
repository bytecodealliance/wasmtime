;; A component exporting functions that pass `map<u32, u32>` values in and out.
(component
  (core module $m
    (memory (export "memory") 1)

    ;; Bump allocator for lowering arguments, starting past the return area.
    (global $next (mut i32) (i32.const 16))
    (func (export "realloc") (param i32 i32 i32 i32) (result i32)
      (local $ptr i32)
      ;; Align up `$next` to the requested alignment.
      (local.set $ptr
        (i32.and
          (i32.add (global.get $next) (i32.sub (local.get 2) (i32.const 1)))
          (i32.sub (i32.const 0) (local.get 2))))
      (global.set $next (i32.add (local.get $ptr) (local.get 3)))
      (local.get $ptr))

    ;; Returns its map argument unchanged.
    (func (export "echo") (param $ptr i32) (param $len i32) (result i32)
      (i32.store (i32.const 0) (local.get $ptr))
      (i32.store (i32.const 4) (local.get $len))
      (i32.const 0))

    ;; Returns the entries 1: 10, 2: 20 and 1: 30, with a duplicate key.
    (data (i32.const 8192) "\01\00\00\00\0a\00\00\00\02\00\00\00\14\00\00\00\01\00\00\00\1e\00\00\00")
    (func (export "duplicates") (result i32)
      (i32.store (i32.const 0) (i32.const 8192))
      (i32.store (i32.const 4) (i32.const 3))
      (i32.const 0))
  )
  (core instance $i (instantiate $m))

  (type $map (map u32 u32))
  (func (export "echo") (param "m" $map) (result $map)
    (canon lift (core func $i "echo")
      (memory (core memory $i "memory"))
      (realloc (core func $i "realloc"))))
  (func (export "duplicates") (result $map)
    (canon lift (core func $i "duplicates")
      (memory (core memory $i "memory"))))
)
