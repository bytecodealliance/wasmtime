;;! bulk_memory = true
;;! function_references = true
;;! stack_switching = true

(module
  (type $ft (func (result i32)))
  (type $ct (cont $ft))

  (type $ft-1 (func (param i32) (result i32)))
  (type $ct-1 (cont $ft-1))

  (tag $suspend)
  (tag $exception (param i32))
  (global $entered (mut i32) (i32.const 0))
  (global $stacks-left (mut i32) (i32.const 0))

  ;; Catch an exception injected at the suspension point on the
  ;; resumed continuation's own stack, including its payload.
  (func $catch-in-child (result i32)
    (block $caught (result i32 exnref)
      (try_table (result i32) (catch_ref $exception $caught)
        (suspend $suspend)
        (i32.const 0))
      (return))
    (drop))

  (func (export "catch-in-child") (result i32)
    (local $k (ref null $ct))
    (block $on-suspend (result (ref $ct))
      (resume $ct (on $suspend $on-suspend)
        (cont.new $ct (ref.func $catch-in-child)))
      (unreachable))
    (local.set $k)
    (resume_throw $ct $exception
      (i32.const 42)
      (local.get $k)))

  ;; Let the exception escape the resumed stack and catch it in the
  ;; parent stack around resume_throw.
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
    (block $caught (result i32)
      (try_table (result i32) (catch $exception $caught)
        (resume_throw $ct $exception
          (i32.const 43)
          (local.get $k)))
      (return)))

  ;; Keep a copy of a continuation reference while throwing into it. Catching
  ;; the injected exception does not undo consumption of the continuation, so
  ;; attempting to resume it again through the retained reference must trap.
  (func (export "resume-after-caught-throw") (result i32)
    (local $k (ref null $ct))
    (block $on-suspend (result (ref $ct))
      (resume $ct (on $suspend $on-suspend)
        (cont.new $ct (ref.func $uncaught-in-child)))
      (unreachable))
    (local.set $k)
    (block $caught (result i32)
      (try_table (result i32) (catch $exception $caught)
        (resume_throw $ct $exception
          (i32.const 46)
          (local.get $k))))
    (drop)
    (resume $ct (local.get $k)))

  ;; Throwing into a fresh continuation must not enter its function body.
  (func $fresh-body (result i32)
    (global.set $entered
      (i32.add (global.get $entered) (i32.const 1)))
    (i32.const 0))

  (func (export "throw-into-fresh") (result i32)
    (block $caught (result i32)
      (try_table (result i32) (catch $exception $caught)
        (resume_throw $ct $exception
          (i32.const 44)
          (cont.new $ct (ref.func $fresh-body))))
      (return))
    (global.get $entered)
    (i32.const 100)
    (i32.mul)
    (i32.add))

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
    (block $caught (result i32)
      (try_table (result i32) (catch $exception $caught)
        (resume_throw $ct $exception
          (i32.const 45)
          (local.get $k)))))

  (elem declare func $catch-in-child $uncaught-in-child $fresh-body $deep-stack)
)

(assert_return (invoke "catch-in-child") (i32.const 42))
(assert_return (invoke "catch-in-parent") (i32.const 43))
(assert_return (invoke "throw-into-fresh") (i32.const 44))
(assert_return (invoke "catch-eight-stacks-deep") (i32.const 45))
(assert_trap (invoke "resume-after-caught-throw") "continuation already consumed")
