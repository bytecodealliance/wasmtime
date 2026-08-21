;;! stack_switching = true
;;! gc = true
;;! function_references = true
;;! bulk_memory = true

;; A GC reference in a parent frame must remain live while a child
;; continuation is running.
(module
  (type $box (struct (field i32)))
  (type $f (func))
  (type $c (cont $f))
  (tag $suspend)

  (import "wasmtime" "gc" (func $gc))

  (func $child
    (call $gc)
    (suspend $suspend))
  (elem declare func $child)

  (func (export "parent-frame") (result i32)
    (local $box (ref null $box))
    (local.set $box (struct.new $box (i32.const 42)))

    (drop
      (block $on-suspend (result (ref $c))
        (resume $c
          (on $suspend $on-suspend)
          (cont.new $c (ref.func $child)))
        (unreachable)))

    (struct.get $box 0 (local.get $box)))
)

(assert_return (invoke "parent-frame") (i32.const 42))

;; Stack-map tracing follows the full parent chain, and execution can
;; subsequently re-enter every parent frame.
(module
  (type $box (struct (field i32)))
  (type $f (func (result i32)))
  (type $c (cont $f))
  (tag $suspend)

  (import "wasmtime" "gc" (func $gc))

  (func $child (result i32)
    (call $gc)
    (suspend $suspend)
    (i32.const 1))
  (elem declare func $child)

  (func $middle (result i32)
    (local $box (ref null $box))
    (local.set $box (struct.new $box (i32.const 20)))
    (i32.add
      (resume $c (cont.new $c (ref.func $child)))
      (struct.get $box 0 (local.get $box))))
  (elem declare func $middle)

  (func (export "parent-chain") (result i32)
    (local $box (ref null $box))
    (local $suspended (ref null $c))
    (local.set $box (struct.new $box (i32.const 100)))

    (local.set $suspended
      (block $on-suspend (result (ref $c))
        (resume $c
          (on $suspend $on-suspend)
          (cont.new $c (ref.func $middle)))
        (unreachable)))

    (i32.add
      (resume $c (local.get $suspended))
      (struct.get $box 0 (local.get $box))))
)

(assert_return (invoke "parent-chain") (i32.const 121))

;; A bound GC reference is retained only by a fresh continuation's
;; argument buffer when collection runs before the continuation is
;; first resumed.
(module
  (type $box (struct (field i32)))
  (type $with-arg-f (func (param (ref $box)) (result i32)))
  (type $without-arg-f (func (result i32)))
  (type $with-arg-c (cont $with-arg-f))
  (type $without-arg-c (cont $without-arg-f))

  (import "wasmtime" "gc" (func $gc))

  (func $read-box (param $box (ref $box)) (result i32)
    (struct.get $box 0 (local.get $box)))
  (elem declare func $read-box)

  (func (export "bound-argument") (result i32)
    (local $continuation (ref null $without-arg-c))

    (local.set $continuation
      (cont.bind $with-arg-c $without-arg-c
        (struct.new $box (i32.const 73))
        (cont.new $with-arg-c (ref.func $read-box))))

    (call $gc)
    (resume $without-arg-c
      (local.get $continuation)))
)

(assert_return (invoke "bound-argument") (i32.const 73))

;; The same root metadata is used for the `values` buffer after a
;; continuation has suspended.
(module
  (type $box (struct (field i32)))
  (type $initial-f (func (result i32)))
  (type $suspended-f (func (param (ref $box)) (result i32)))
  (type $initial-c (cont $initial-f))
  (type $suspended-c (cont $suspended-f))
  (tag $yield (result (ref $box)))

  (import "wasmtime" "gc" (func $gc))

  (func $suspend-then-read (result i32)
    (struct.get $box 0 (suspend $yield)))
  (elem declare func $suspend-then-read)

  (func (export "bound-suspended-argument") (result i32)
    (local $suspended (ref null $suspended-c))
    (local $ready (ref null $initial-c))

    (local.set $suspended
      (block $on-yield (result (ref $suspended-c))
        (resume $initial-c
          (on $yield $on-yield)
          (cont.new $initial-c (ref.func $suspend-then-read)))
        (unreachable)))

    (local.set $ready
      (cont.bind $suspended-c $initial-c
        (struct.new $box (i32.const 91))
        (local.get $suspended)))

    (call $gc)
    (resume $initial-c (ref.as_non_null (local.get $ready))))
)

(assert_return (invoke "bound-suspended-argument") (i32.const 91))

;; Struct and array fields store continuation references through a
;; store-local side table. Collection may run while the aggregate is
;; the only place that retains the continuation object.
(module
  (type $f (func (result i32)))
  (type $c (cont $f))
  (type $holder (struct (field (mut (ref null $c)))))
  (type $array (array (mut (ref null $c))))

  (import "wasmtime" "gc" (func $gc))

  (func $forty-two (result i32)
    (i32.const 42))
  (elem declare func $forty-two)

  (func (export "struct-initialize") (result i32)
    (local $holder (ref $holder))
    (local.set $holder
      (struct.new $holder
        (cont.new $c (ref.func $forty-two))))
    (call $gc)
    (resume $c
      (struct.get $holder 0 (local.get $holder))))

  (func (export "struct-update") (result i32)
    (local $holder (ref $holder))
    (local.set $holder (struct.new_default $holder))
    (struct.set $holder 0
      (local.get $holder)
      (cont.new $c (ref.func $forty-two)))
    (call $gc)
    (resume $c
      (struct.get $holder 0 (local.get $holder))))

  (func (export "array-initialize") (result i32)
    (local $array (ref $array))
    (local.set $array
      (array.new_fixed $array 1
        (cont.new $c (ref.func $forty-two))))
    (call $gc)
    (resume $c
      (array.get $array (local.get $array) (i32.const 0))))

  (func (export "array-update") (result i32)
    (local $array (ref $array))
    (local.set $array (array.new_default $array (i32.const 1)))
    (array.set $array
      (local.get $array)
      (i32.const 0)
      (cont.new $c (ref.func $forty-two)))
    (call $gc)
    (resume $c
      (array.get $array (local.get $array) (i32.const 0))))

  (func (export "array-fill-and-copy") (result i32)
    (local $source (ref $array))
    (local $destination (ref $array))
    (local.set $source (array.new_default $array (i32.const 1)))
    (local.set $destination (array.new_default $array (i32.const 1)))
    (array.fill $array
      (local.get $source)
      (i32.const 0)
      (cont.new $c (ref.func $forty-two))
      (i32.const 1))
    (array.copy $array $array
      (local.get $destination)
      (i32.const 0)
      (local.get $source)
      (i32.const 0)
      (i32.const 1))
    (call $gc)
    (resume $c
      (array.get $array (local.get $destination) (i32.const 0))))

  (func (export "null-defaults") (result i32)
    (local $holder (ref $holder))
    (local $array (ref $array))
    (local.set $holder (struct.new_default $holder))
    (local.set $array (array.new_default $array (i32.const 1)))
    (i32.add
      (ref.is_null (struct.get $holder 0 (local.get $holder)))
      (ref.is_null
        (array.get $array (local.get $array) (i32.const 0)))))

  ;; Loading an alias from a GC object must reproduce the interned
  ;; revision witness. Consuming one alias therefore invalidates all
  ;; other loads of the same field.
  (func (export "consume-struct-field-twice")
    (local $holder (ref $holder))
    (local.set $holder
      (struct.new $holder
        (cont.new $c (ref.func $forty-two))))
    (drop
      (resume $c
        (struct.get $holder 0 (local.get $holder))))
    (drop
      (resume $c
        (struct.get $holder 0 (local.get $holder)))))
)

(assert_return (invoke "struct-initialize") (i32.const 42))
(assert_return (invoke "struct-update") (i32.const 42))
(assert_return (invoke "array-initialize") (i32.const 42))
(assert_return (invoke "array-update") (i32.const 42))
(assert_return (invoke "array-fill-and-copy") (i32.const 42))
(assert_return (invoke "null-defaults") (i32.const 2))
(assert_trap
  (invoke "consume-struct-field-twice")
  "continuation already consumed")

;; A GC reference passed to a child continuation must remain live when
;; that child is unwound by `resume_throw`. The parent retains another
;; reference, catches the injected exception, and can still use the
;; object after a second collection.
(module
  (type $box (struct (field i32)))
  (type $child-f (func (param (ref $box)) (result i32)))
  (type $parent-f (func (result i32)))
  (type $child-c (cont $child-f))
  (type $parent-c (cont $parent-f))
  (tag $suspend)
  (tag $exception)

  (import "wasmtime" "gc" (func $gc))

  (func $child (param $box (ref $box)) (result i32)
    ;; At this collection, both the child and its parent have the box
    ;; in live stack slots. Keep the parameter live across the
    ;; suspension point.
    (call $gc)
    (suspend $suspend)
    (struct.get $box 0 (local.get $box)))
  (elem declare func $child)

  (func $parent (result i32)
    (local $box (ref $box))
    (local.set $box (struct.new $box (i32.const 101)))

    (block $caught
      (try_table (catch $exception $caught)
        (drop
          (resume $child-c
            (local.get $box)
            (cont.new $child-c (ref.func $child)))))
      (unreachable))

    ;; The child continuation is terminal, so only the parent's stack
    ;; slot retains the box at this collection.
    (call $gc)
    (struct.get $box 0 (local.get $box)))
  (elem declare func $parent)

  (func (export "gc-ref-survives-resume-throw") (result i32)
    (local $suspended (ref null $parent-c))

    (local.set $suspended
      (block $on-suspend (result (ref $parent-c))
        (resume $parent-c
          (on $suspend $on-suspend)
          (cont.new $parent-c (ref.func $parent)))
        (unreachable)))

    (resume_throw $parent-c $exception
      (local.get $suspended)))
)

(assert_return
  (invoke "gc-ref-survives-resume-throw")
  (i32.const 101))

;; A GC reference carried by an exception injected with `resume_throw`
;; must become a live stack root when the exception is caught inside
;; the resumed continuation.
(module
  (type $box (struct (field i32)))
  (type $f (func (result i32)))
  (type $c (cont $f))
  (tag $suspend)
  (tag $exception (param (ref $box)))

  (import "wasmtime" "gc" (func $gc))

  (func $catch-payload (result i32)
    (local $box (ref null $box))

    (local.set $box
      (block $caught (result (ref $box))
        (try_table (result (ref $box)) (catch $exception $caught)
          (suspend $suspend)
          (unreachable))))

    ;; The caught local is now the only reference to the exception
    ;; payload.
    (call $gc)
    (struct.get $box 0 (ref.as_non_null (local.get $box))))
  (elem declare func $catch-payload)

  (func (export "gc-ref-in-resume-throw-payload") (result i32)
    (local $suspended (ref null $c))

    (local.set $suspended
      (block $on-suspend (result (ref $c))
        (resume $c
          (on $suspend $on-suspend)
          (cont.new $c (ref.func $catch-payload)))
        (unreachable)))

    (resume_throw $c $exception
      (struct.new $box (i32.const 202))
      (local.get $suspended)))
)

(assert_return
  (invoke "gc-ref-in-resume-throw-payload")
  (i32.const 202))
