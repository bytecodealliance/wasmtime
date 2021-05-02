(module
  (func $fibonacci (param i32) (result i32)
    (local i32)
    local.get 0
    i32.const 2
    i32.ge_s
    if (result i32)  ;; label = @1
      local.get 0
      i32.const 1
      i32.add
      local.set 0
      loop  ;; label = @2
        local.get 0
        i32.const -3
        i32.add
        call 0
        local.get 1
        i32.add
        local.set 1
        local.get 0
        i32.const -1
        i32.add
        local.tee 0
        i32.const 2
        i32.gt_s
        br_if 0 (;@2;)
      end
      i32.const 1
    else
      local.get 0
    end
    local.get 1
    i32.add
  )
  (export "fibonacci" (func $fibonacci))
)
