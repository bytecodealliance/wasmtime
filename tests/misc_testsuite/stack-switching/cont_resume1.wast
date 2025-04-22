;;! stack_switching = true
(module
  (type $ft_init (func))
  (type $ct_init (cont $ft_init))
  (type $ft (func (param i32)))
  (type $ct (cont $ft))
  (tag $yield (result i32))

  (global $i (mut i32) (i32.const 0))

  (func $g
    (suspend $yield)
    (global.set $i))
  (elem declare func $g)

  (func $f (export "f") (result i32)
    (local $k (ref null $ct))
    (global.set $i (i32.const 99))
    (block $on_yield (result (ref $ct))
      (resume $ct_init (on $yield $on_yield) (cont.new $ct_init (ref.func $g)))
      (unreachable))
    ;; on_yield
    (local.set $k)
    (resume $ct (i32.const 42) (local.get $k))
    (global.get $i))
)

(assert_return (invoke "f") (i32.const 42))