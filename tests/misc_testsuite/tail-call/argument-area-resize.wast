;;! tail_call = true
;;! reference_types = true

;; Direct tail calls grow and shrink the incoming stack-argument area.
(module
  (func $small (param i32 i32 i32 i32 i32) (result i32)
    local.get 0
    i32.eqz
    if (result i32)
      local.get 1
    else
      local.get 0
      i32.const 1
      i32.sub
      local.get 1
      local.get 2
      local.get 3
      local.get 4
      i32.const 5
      i32.const 6
      i32.const 7
      i32.const 8
      return_call $large
    end)

  (func $large (param i32 i32 i32 i32 i32 i32 i32 i32 i32) (result i32)
    local.get 0
    i32.eqz
    if (result i32)
      local.get 1
    else
      local.get 0
      i32.const 1
      i32.sub
      local.get 1
      local.get 2
      local.get 3
      local.get 4
      return_call $small
    end)

  (func (export "run") (param i32) (result i32)
    local.get 0
    i32.const 42
    i32.const 2
    i32.const 3
    i32.const 4
    call $small))

(assert_return (invoke "run" (i32.const 100000)) (i32.const 42))

;; The same resizing through indirect tail calls.
(module
  (type $small-ty (func (param i32 i32 i32 i32 i32) (result i32)))
  (type $large-ty
    (func (param i32 i32 i32 i32 i32 i32 i32 i32 i32) (result i32)))

  (table funcref (elem $small $large))

  (func $small (type $small-ty)
    local.get 0
    i32.eqz
    if (result i32)
      local.get 1
    else
      local.get 0
      i32.const 1
      i32.sub
      local.get 1
      local.get 2
      local.get 3
      local.get 4
      i32.const 5
      i32.const 6
      i32.const 7
      i32.const 8
      i32.const 1
      return_call_indirect (type $large-ty)
    end)

  (func $large (type $large-ty)
    local.get 0
    i32.eqz
    if (result i32)
      local.get 1
    else
      local.get 0
      i32.const 1
      i32.sub
      local.get 1
      local.get 2
      local.get 3
      local.get 4
      i32.const 0
      return_call_indirect (type $small-ty)
    end)

  (func (export "run") (param i32) (result i32)
    local.get 0
    i32.const 42
    i32.const 2
    i32.const 3
    i32.const 4
    call $small))

(assert_return (invoke "run" (i32.const 100000)) (i32.const 42))

;; Forward mixed register and stack results through direct and indirect tails.
(module
  (type $small-ty
    (func (param i32 i32 i64 i32 i64) (result i32 i64 i32 i64)))
  (type $large-ty
    (func
      (param i32 i32 i64 i32 i64 i32 i32 i32 i32)
      (result i32 i64 i32 i64)))

  (table funcref (elem $small $large))

  (func $small (type $small-ty)
    local.get 0
    i32.eqz
    if (result i32 i64 i32 i64)
      local.get 1
      local.get 2
      local.get 3
      local.get 4
    else
      local.get 0
      i32.const 1
      i32.sub
      local.get 1
      local.get 2
      local.get 3
      local.get 4
      i32.const 5
      i32.const 6
      i32.const 7
      i32.const 8
      i32.const 1
      return_call_indirect (type $large-ty)
    end)

  (func $large (type $large-ty)
    local.get 0
    i32.eqz
    if (result i32 i64 i32 i64)
      local.get 1
      local.get 2
      local.get 3
      local.get 4
    else
      local.get 0
      i32.const 1
      i32.sub
      local.get 1
      local.get 2
      local.get 3
      local.get 4
      return_call $small
    end)

  (func (export "run") (param i32) (result i32 i64 i32 i64)
    local.get 0
    i32.const 42
    i64.const 43
    i32.const 44
    i64.const 45
    return_call $small))

(assert_return (invoke "run" (i32.const 100000))
  (i32.const 42) (i64.const 43) (i32.const 44) (i64.const 45))
