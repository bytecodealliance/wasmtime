(module $n
  (table $t (import "m" "t") 6 funcref)

  (func $i (param i32 i32 i32 i32 i32 i32) (result i32) (local.get 3))
  (func $j (param i32 i32 i32 i32 i32 i32) (result i32) (local.get 4))
  (func $k (param i32 i32 i32 i32 i32 i32) (result i32) (local.get 5))

  (table $u (export "u") funcref (elem $i $j $k $i $j $k))

  (func (export "copy_to_t_from_u") (param i32 i32 i32 i32) (result i32)
    local.get 0
    local.get 1
    local.get 2
    local.get 3
    table.copy $t $u)

  (func (export "copy_to_u_from_t") (param i32 i32 i32 i32) (result i32)
    local.get 0
    local.get 1
    local.get 2
    local.get 3
    table.copy $u $t))
