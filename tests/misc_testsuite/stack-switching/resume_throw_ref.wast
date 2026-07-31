;;! bulk_memory = true
;;! function_references = true
;;! stack_switching = true

(module
  (type $ft (func (result i32)))
  (type $ct (cont $ft))

  (type $ft-1 (func (param i32) (result i32)))
  (type $ct-1 (cont $ft-1))

  (tag $suspend)
  (tag $exception)
  (global $entered (mut i32) (i32.const 0))

  (func $make-exception (result exnref)
    (block $caught (result exnref)
      (try_table (catch_ref $exception $caught)
        (throw $exception))
      (unreachable)))

  ;; Catch an exception injected at the suspend instruction on the
  ;; resumed continuation's own stack.
  (func $catch-in-child (result i32)
    (block $caught (result exnref)
      (try_table (result i32) (catch_ref $exception $caught)
        (suspend $suspend)
        (i32.const 0))
      (return))
    (drop)
    (i32.const 42))

  (func (export "catch-in-child") (result i32)
    (local $k (ref null $ct))
    (block $on-suspend (result (ref $ct))
      (resume $ct (on $suspend $on-suspend)
        (cont.new $ct (ref.func $catch-in-child)))
      (unreachable))
    (local.set $k)
    (resume_throw_ref $ct
      (call $make-exception)
      (local.get $k)))

  ;; Let the injected exception escape the resumed stack and catch it in the
  ;; parent stack around resume_throw_ref.
  (func $uncaught-in-child (result i32)
    (suspend $suspend)
    (i32.const 0))

  (func (export "catch-in-parent") (result i32)
    (local $k (ref null $ct))
    (block $on-suspend (result (ref $ct))
      (resume $ct (on $suspend $on-suspend)
        (cont.new $ct (ref.func $uncaught-in-child)))
      (unreachable))
    (local.set $k)
    (block $caught (result exnref)
      (try_table (result i32) (catch_ref $exception $caught)
        (resume_throw_ref $ct
          (call $make-exception)
          (local.get $k)))
      (return))
    (drop)
    (i32.const 43))

  ;; Throwing into a fresh continuation must not enter its function body.
  (func $fresh-body (result i32)
    (global.set $entered
      (i32.add (global.get $entered) (i32.const 1)))
    (i32.const 0))

  (func (export "throw-into-fresh") (result i32)
    (block $caught (result exnref)
      (try_table (result i32) (catch_ref $exception $caught)
        (resume_throw_ref $ct
          (call $make-exception)
          (cont.new $ct (ref.func $fresh-body))))
      (return))
    (drop)
    (global.get $entered))

  ;; This function builds up a deep nesting of stacks. The leaf stack
  ;; suspends to its immediate handler, which injects an exception
  ;; into it, causing it to unwind to the top-most stack, where it is
  ;; caught.
  (func $deep-stack (param $stacks-left i32) (result i32)
    (if (result i32) (i32.gt_u (local.get $stacks-left) (i32.const 0))
      (then
        (local.set $stacks-left
          (i32.sub (local.get $stacks-left) (i32.const 1)))
        (resume $ct-1 (local.get $stacks-left) (cont.new $ct-1 (ref.func $deep-stack))))
      (else
        (suspend $suspend)
        (i32.const 0))))

  (func (export "catch-eight-stacks-deep") (result i32)
    (local $k (ref null $ct))
    ;; The first resume creates one stack and each of these seven iterations
    ;; creates another.
    (block $on-suspend (result (ref $ct))
      (resume $ct-1 (on $suspend $on-suspend)
        (i32.const 8) (cont.new $ct-1 (ref.func $deep-stack)))
      (unreachable))
    (local.set $k)
    (block $caught (result exnref)
      (try_table (result i32) (catch_ref $exception $caught)
        (resume_throw_ref $ct
          (call $make-exception)
          (local.get $k)))
      (return))
    (drop)
    (i32.const 45))

  (elem declare func $catch-in-child $uncaught-in-child $fresh-body $deep-stack)
)

(assert_return (invoke "catch-in-child") (i32.const 42))
(assert_return (invoke "catch-in-parent") (i32.const 43))
(assert_return (invoke "throw-into-fresh") (i32.const 0))
(assert_return (invoke "catch-eight-stacks-deep") (i32.const 45))
