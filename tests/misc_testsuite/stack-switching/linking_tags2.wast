;;! stack_switching = true
(module $foo
  (type $ft (func (result i32)))
  (type $ct (cont $ft))
  (type $ft-2 (func (param i32) (result i32)))
  (type $ct-2 (cont $ft-2))

  (tag $foo (export "foo") (result i32)) ;; occupies defined tag entry 0

  (func $do_foo (export "do_foo") (result i32)
     (suspend $foo))
  (func $handle_foo (export "handle_foo") (param $f (ref $ft)) (result i32)
    (block $on_foo (result (ref $ct-2))
      (resume $ct (on $foo $on_foo) (cont.new $ct (local.get $f)))
      (return)
    ) ;; on_foo
    (drop)
    (return (i32.const 1))
  )
  (func (export "test_foo") (result i32)
    (call $handle_foo (ref.func $do_foo)))
  (elem declare func $do_foo)
)
(register "foo")
(assert_return (invoke "test_foo") (i32.const 1))

(module $baz
  (type $ft (func (result i32)))
  (type $ct (cont $ft))

  (type $ft-2 (func (param i32) (result i32)))
  (type $ct-2 (cont $ft-2))

  (func $handle_foo (import "foo" "handle_foo") (param (ref $ft)) (result i32))
  (func $do_foo (import "foo" "do_foo") (result i32))

  (tag $baz (result i32)) ;; unused, but occupies defined tag entry 0

  (func $handle_baz (param $f (ref $ft)) (result i32)
    (block $on_baz (result (ref $ct-2))
      (resume $ct (on $baz $on_baz) (cont.new $ct (local.get $f)))
      (return)
    ) ;; on_baz
    (drop)
    (return (i32.const 3))
  )

  (func $inner-baz (result i32)
    (call $handle_baz (ref.func $do_foo)))
  (func (export "compose-handle-foo-baz") (result i32)
    (call $handle_foo (ref.func $inner-baz)))

  (func $inner-foo (result i32)
    (call $handle_foo (ref.func $do_foo)))
  (func (export "compose-handle-baz-foo") (result i32)
    (call $handle_baz (ref.func $inner-foo)))
  (elem declare func $do_foo $inner-baz $inner-foo)
)
(register "baz")
(assert_return (invoke "compose-handle-baz-foo") (i32.const 1))
(assert_return (invoke "compose-handle-foo-baz") (i32.const 1))

(module $quux
  (type $ft (func (result i32)))
  (type $ct (cont $ft))

  (type $ft-2 (func (param i32) (result i32)))
  (type $ct-2 (cont $ft-2))

  (func $handle_foo (import "foo" "handle_foo") (param (ref $ft)) (result i32))
  (tag $foo (import "foo" "foo") (result i32))

  (func $do_foo (result i32)
    (suspend $foo))

  (func $my_handle_foo (param $f (ref $ft)) (result i32)
    (block $on_foo (result (ref $ct-2))
      (resume $ct (on $foo $on_foo) (cont.new $ct (local.get $f)))
      (return)
    ) ;; on_foo
    (drop)
    (return (i32.const 4))
  )

  (func $inner-my-foo (result i32)
    (call $my_handle_foo (ref.func $do_foo)))
  (func (export "compose-handle-foo-my-foo") (result i32)
    (call $handle_foo (ref.func $inner-my-foo)))

  (func $inner-foo (result i32)
    (call $handle_foo (ref.func $do_foo)))
  (func (export "compose-handle-my-foo-foo") (result i32)
    (call $my_handle_foo (ref.func $inner-foo)))
  (elem declare func $do_foo $inner-my-foo $inner-foo)
)
(register "quux")
(assert_return (invoke "compose-handle-foo-my-foo") (i32.const 4))
(assert_return (invoke "compose-handle-my-foo-foo") (i32.const 1))

(module $bar
  (type $ft (func (result i32)))
  (type $ct (cont $ft))

  (type $ft-2 (func (param i32) (result i32)))
  (type $ct-2 (cont $ft-2))

  (tag $foo (import "foo" "foo") (result i32))
  (tag $bar (result i32))
  (func $do_foo (result i32)
    (suspend $foo))

  ;; Don't handle the imported foo.
  (func (export "skip-imported-foo") (result i32)
    (block $on_bar (result (ref $ct-2))
      (resume $ct (on $bar $on_bar) (cont.new $ct (ref.func $do_foo)))
      (unreachable)
    )
    (unreachable))

  ;; Handle the imported foo.
  (func (export "handle-imported-foo") (result i32)
    (block $on_foo (result (ref $ct-2))
      (resume $ct (on $foo $on_foo) (cont.new $ct (ref.func $do_foo)))
      (unreachable)
    )
    (drop)
    (return (i32.const 2))
  )

  (elem declare func $do_foo)
)
(register "bar")
(assert_return (invoke "handle-imported-foo") (i32.const 2))
;; Due to issue #253, we need to make sure that nothing happens afterwards in
;; the test:
(assert_suspension (invoke "skip-imported-foo") "unhandled")
