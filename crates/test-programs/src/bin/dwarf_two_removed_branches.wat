(module
  (type (;0;) (func))
  (type (;1;) (func (param i32 i32 i32) (result i32)))
  (func (;0;) (type 1) (param i32 i32 i32) (result i32)
    (local i32 i32 i32 i32 i32 i32 i32 i32 i32 i32 i32 i32 i32 i32 i32 i32 i32 i32 i32)
    i32.const -1
    local.set 6
    local.get 5
    i32.load offset=12
    local.set 8
    local.get 6
    local.set 9
    local.get 8
    local.get 9
    i32.eq
    local.set 10
    i32.const 1
    local.set 11
    local.get 10
    local.get 11
    i32.and
    local.set 12
    block  ;; label = @1
      block  ;; label = @2
        local.get 12
        i32.eqz
        br_if 0 (;@2;)
        i32.const 1
        local.set 13
        local.get 13
        local.set 14
        br 1 (;@1;)
      end
      local.get 5
      i32.load offset=12
      local.set 15
      local.get 15
      local.set 14
    end
    local.get 14
    local.set 16
    local.get 5
    i32.load offset=8
    local.set 17
    local.get 5
    i32.load offset=4
    local.set 18
    local.get 16
    local.get 17
    local.get 18
    call 1
    return)
  (func (;1;) (type 1) (param i32 i32 i32) (result i32)
    i32.const -1)
  (func (;2;) (type 0))
  (table (;0;) 1368 1368 funcref)
  (memory (;0;) 800 8000)
  (export "memory" (memory 0))
  (export "_start" (func 2)))
