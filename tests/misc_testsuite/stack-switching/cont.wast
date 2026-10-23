;;! bulk_memory = true
;;! gc = true
;;! function_references = true
;;! reference_types = true
;;! stack_switching = true
;; Unhandled tags & guards

(module
  (tag $exn)
  (tag $e1)
  (tag $e2)

  (type $f1 (func))
  (type $k1 (cont $f1))

  (func $f1 (export "unhandled-1")
    (suspend $e1)
  )

  (func (export "unhandled-2")
    (resume $k1 (cont.new $k1 (ref.func $f1)))
  )

  (func (export "unhandled-3")
    (block $h (result (ref $k1))
      (resume $k1 (on $e2 $h) (cont.new $k1 (ref.func $f1)))
      (unreachable)
    )
    (drop)
  )

  (func (export "handled")
    (block $h (result (ref $k1))
      (resume $k1 (on $e1 $h) (cont.new $k1 (ref.func $f1)))
      (unreachable)
    )
    (drop)
  )

  (elem declare func $f2)
  (func $f2
    (throw $exn)
  )

  (func (export "uncaught-1")
    (block $h (result (ref $k1))
      (resume $k1 (on $e1 $h) (cont.new $k1 (ref.func $f2)))
      (unreachable)
    )
    (drop)
  )

  (func (export "uncaught-2")
    (block $h (result (ref $k1))
      (resume $k1 (on $e1 $h) (cont.new $k1 (ref.func $f1)))
      (unreachable)
    )
    (resume_throw $k1 $exn)
  )

  (elem declare func $f3)
  (func $f3
    (call $f4)
  )
  (func $f4
    (suspend $e1)
  )

  (func (export "uncaught-3")
    (block $h (result (ref $k1))
      (resume $k1 (on $e1 $h) (cont.new $k1 (ref.func $f3)))
      (unreachable)
    )
    (resume_throw $k1 $exn)
  )

  (elem declare func $r0 $r1)
  (func $r0)
  (func $r1 (suspend $e1) (suspend $e1))

  (func $nl1 (param $k (ref $k1))
    (resume $k1 (local.get $k))
    (resume $k1 (local.get $k))
  )
  (func $nl2 (param $k (ref $k1))
    (block $h (result (ref $k1))
      (resume $k1 (on $e1 $h) (local.get $k))
      (unreachable)
    )
    (resume $k1 (local.get $k))
    (unreachable)
  )
  (func $nl3 (param $k (ref $k1))
    (local $k' (ref null $k1))
    (block $h1 (result (ref $k1))
      (resume $k1 (on $e1 $h1) (local.get $k))
      (unreachable)
    )
    (local.set $k')
    (block $h2 (result (ref $k1))
      (resume $k1 (on $e1 $h2) (local.get $k'))
      (unreachable)
    )
    (resume $k1 (local.get $k'))
    (unreachable)
  )
  (func $nl4 (param $k (ref $k1))
    (drop (cont.bind $k1 $k1 (local.get $k)))
    (resume $k1 (local.get $k))
  )

  (func (export "non-linear-1")
    (call $nl1 (cont.new $k1 (ref.func $r0)))
  )
  (func (export "non-linear-2")
    (call $nl2 (cont.new $k1 (ref.func $r1)))
  )
  (func (export "non-linear-3")
    (call $nl3 (cont.new $k1 (ref.func $r1)))
  )
  (func (export "non-linear-4")
    (call $nl4 (cont.new $k1 (ref.func $r1)))
  )

  (func (export "null-resume")
    (resume $k1
      (ref.null $k1)
    )
  )

  (func (export "null-new")
    (cont.new $k1
      (ref.null $f1)
    )
    (drop)
  )

  (func (export "null-bind")
    (drop
      (cont.bind $k1 $k1
        (ref.null $k1)
      )
    )
  )
)

(assert_suspension (invoke "unhandled-1") "unhandled")
(assert_suspension (invoke "unhandled-2") "unhandled")
(assert_suspension (invoke "unhandled-3") "unhandled")
(assert_return (invoke "handled"))

(assert_exception (invoke "uncaught-1"))
(assert_exception (invoke "uncaught-2"))
(assert_exception (invoke "uncaught-3"))

(assert_trap (invoke "non-linear-1") "continuation already consumed")
(assert_trap (invoke "non-linear-2") "continuation already consumed")
(assert_trap (invoke "non-linear-3") "continuation already consumed")
(assert_trap (invoke "non-linear-4") "continuation already consumed")

(assert_trap (invoke "null-resume") "null continuation reference")
(assert_trap (invoke "null-new") "null function reference")
(assert_trap (invoke "null-bind") "null continuation reference")

(assert_invalid
  (module
    (type $ft (func))
    (func
      (cont.new $ft (ref.null $ft))
      (drop)))
  "non-continuation type 0")

(assert_invalid
  (module
    (type $ft (func))
    (type $ct (cont $ft))
    (func
      (resume $ft (ref.null $ct))
      (unreachable)))
  "non-continuation type 0")

(assert_invalid
  (module
    (type $ft (func))
    (type $ct (cont $ft))
    (tag $exn)
    (func
      (resume_throw $ft $exn (ref.null $ct))
      (unreachable)))
  "non-continuation type 0")

(assert_invalid
  (module
    (type $ft (func))
    (type $ct (cont $ft))
    (func
      (cont.bind $ft $ct (ref.null $ct))
      (unreachable)))
  "non-continuation type 0")

(assert_invalid
  (module
    (type $ft (func))
    (type $ct (cont $ft))
    (func
      (cont.bind $ct $ft (ref.null $ct))
      (unreachable)))
  "non-continuation type 0")

(assert_invalid
  (module
    (type $ft (func))
    (type $ct (cont $ft))
    (tag $foo)
    (func
      (block $on_foo (result (ref $ft))
        (resume $ct (on $foo $on_foo) (ref.null $ct))
        (unreachable)
      )
      (drop)))
  "non-continuation type")

(assert_invalid
  (module
    (type $ft (func))
    (type $ct (cont $ft))
    (tag $foo)
    (func
      (block $on_foo (result (ref $ct) (ref $ft))
        (resume $ct (on $foo $on_foo) (ref.null $ct))
        (unreachable)
      )
      (drop)
      (drop)))
  "type mismatch between tag type and label type")

(assert_invalid
  (module
    (type $ct (cont $ct)))
  "non-function type 0")

(assert_invalid
  (module
    (rec
      (type $s0 (struct (field (ref 0) (ref 1) (ref $s0) (ref $s1))))
      (type $s1 (struct (field (ref 0) (ref 1) (ref $s0) (ref $s1))))
    )
    (type $ct (cont $s0)))
  "non-function type 0")

(module
  (rec
    (type $f1 (func (param (ref $f2))))
    (type $f2 (func (param (ref $f1))))
  )
  (type $c1 (cont $f1))
  (type $c2 (cont $f2))
)

(assert_invalid
  (module
    (rec
      (type $fA (func))
      (type $fB (func))
      (type $cont (cont $fA))
    )
    (elem declare func $b)
    (func $a
      (drop
        (cont.new $cont ;; expects a ref of $fA, not $fB
          (ref.func $b)
        )
      )
    )
    (func $b (type $fB)
    )
  )
  "type mismatch")


;; Test resume_throw used on the very first execution of a continuation (so the
;; code in the continuation function is never reached).
(module
  (tag $exn)

  (type $f (func))
  (type $k (cont $f))

  (func $never
    (unreachable)
  )

  (func (export "resume_throw-never")
    (block $handle
      (try_table (catch $exn $handle)
        (resume_throw $k $exn
          (cont.new $k (ref.func $never))
        )
      )
    )
  )

  (elem declare func $never)
)

(assert_return (invoke "resume_throw-never"))

;; Simple state example

(module $state
  (tag $get (result i32))
  (tag $set (param i32) (result i32))

  (type $f (func (param i32) (result i32)))
  (type $k (cont $f))

  (func $runner (param $s i32) (param $k (ref $k)) (result i32)
    (loop $loop
      (block $on_get (result (ref $k))
        (block $on_set (result i32 (ref $k))
          (resume $k (on $get $on_get) (on $set $on_set)
            (local.get $s) (local.get $k)
          )
          (return)
        )
        ;; on set
        (local.set $k)
        (local.set $s)
        (br $loop)
      )
      ;; on get
      (local.set $k)
      (br $loop)
    )
    (unreachable)
  )

  (func $f (param i32) (result i32)
    (drop (suspend $set (i32.const 7)))
    (i32.add
      (suspend $get)
      (i32.mul
        (i32.const 2)
        (i32.add
          (suspend $set (i32.const 3))
          (suspend $get)
        )
      )
    )
  )

  (elem declare func $f)
  (func (export "run") (result i32)
    (call $runner (i32.const 0) (cont.new $k (ref.func $f)))
  )
)

(assert_return (invoke "run") (i32.const 19))


;; Simple generator example

(module $generator
  (type $gen (func (param i64)))
  (type $geny (func (param i32)))
  (type $cont0 (cont $gen))
  (type $cont (cont $geny))

  (tag $yield (param i64) (result i32))

  ;; Hook for logging purposes
  (global $hook (export "hook") (mut (ref $gen)) (ref.func $dummy))
  (func $dummy (param i64))

  (func $gen (export "start") (param $i i64)
    (loop $l
      (br_if 1 (suspend $yield (local.get $i)))
      (call_ref $gen (local.get $i) (global.get $hook))
      (local.set $i (i64.add (local.get $i) (i64.const 1)))
      (br $l)
    )
  )

  (elem declare func $gen)

  (func (export "sum") (param $i i64) (param $j i64) (result i64)
    (local $sum i64)
    (local $n i64)
    (local $k (ref null $cont))
    (local.get $i)
    (cont.new $cont0 (ref.func $gen))
    (block $on_first_yield (param i64 (ref $cont0)) (result i64 (ref $cont))
      (resume $cont0 (on $yield $on_first_yield))
      (unreachable)
    )
    (loop $on_yield (param i64) (param (ref $cont))
      (local.set $k)
      (local.set $n)
      (local.set $sum (i64.add (local.get $sum) (local.get $n)))
      (i64.eq (local.get $n) (local.get $j))
      (local.get $k)
      (resume $cont (on $yield $on_yield))
    )
    (return (local.get $sum))
  )
)

(register "generator")

(assert_return (invoke "sum" (i64.const 0) (i64.const 0)) (i64.const 0))
(assert_return (invoke "sum" (i64.const 2) (i64.const 2)) (i64.const 2))
(assert_return (invoke "sum" (i64.const 0) (i64.const 3)) (i64.const 6))
(assert_return (invoke "sum" (i64.const 1) (i64.const 10)) (i64.const 55))
(assert_return (invoke "sum" (i64.const 100) (i64.const 2000)) (i64.const 1_996_050))


;; Simple scheduler example

(module $scheduler
  (type $proc (func))
  (type $cont (cont $proc))

  (tag $yield (export "yield"))
  (tag $spawn (export "spawn") (param (ref $cont)))

  ;; Table as simple queue (keeping it simple, no ring buffer)
  (table $queue 0 (ref null $cont))
  (global $qdelta i32 (i32.const 10))
  (global $qback (mut i32) (i32.const 0))
  (global $qfront (mut i32) (i32.const 0))

  (func $queue-empty (result i32)
    (i32.eq (global.get $qfront) (global.get $qback))
  )

  (func $dequeue (result (ref null $cont))
    (local $i i32)
    (if (call $queue-empty)
      (then (return (ref.null $cont)))
    )
    (local.set $i (global.get $qfront))
    (global.set $qfront (i32.add (local.get $i) (i32.const 1)))
    (table.get $queue (local.get $i))
  )

  (func $enqueue (param $k (ref $cont))
    ;; Check if queue is full
    (if (i32.eq (global.get $qback) (table.size $queue))
      (then
        ;; Check if there is enough space in the front to compact
        (if (i32.lt_u (global.get $qfront) (global.get $qdelta))
          (then
            ;; Space is below threshold, grow table instead
            (drop (table.grow $queue (ref.null $cont) (global.get $qdelta)))
          )
          (else
            ;; Enough space, move entries up to head of table
            (global.set $qback (i32.sub (global.get $qback) (global.get $qfront)))
            (table.copy $queue $queue
              (i32.const 0)         ;; dest = new front = 0
              (global.get $qfront)  ;; src = old front
              (global.get $qback)   ;; len = new back = old back - old front
            )
            (table.fill $queue      ;; null out old entries to avoid leaks
              (global.get $qback)   ;; start = new back
              (ref.null $cont)      ;; init value
              (global.get $qfront)  ;; len = old front = old front - new front
            )
            (global.set $qfront (i32.const 0))
          )
        )
      )
    )
    (table.set $queue (global.get $qback) (local.get $k))
    (global.set $qback (i32.add (global.get $qback) (i32.const 1)))
  )

  (func $scheduler (export "scheduler") (param $main (ref $cont))
    (call $enqueue (local.get $main))
    (loop $l
      (if (call $queue-empty) (then (return)))
      (block $on_yield (result (ref $cont))
        (block $on_spawn (result (ref $cont) (ref $cont))
          (resume $cont (on $yield $on_yield) (on $spawn $on_spawn)
            (call $dequeue)
          )
          (br $l)  ;; thread terminated
        )
        ;; on $spawn, proc and cont on stack
        (call $enqueue)             ;; continuation of old thread
        (call $enqueue)             ;; new thread
        (br $l)
      )
      ;; on $yield, cont on stack
      (call $enqueue)
      (br $l)
    )
  )
)

(register "scheduler")

(module
  (type $proc (func))
  (type $pproc (func (param i32))) ;; parameterised proc
  (type $cont (cont $proc))
  (type $pcont (cont $pproc)) ;; parameterised continuation proc
  (tag $yield (import "scheduler" "yield"))
  (tag $spawn (import "scheduler" "spawn") (param (ref $cont)))
  (func $scheduler (import "scheduler" "scheduler") (param $main (ref $cont)))

  (func $log (import "spectest" "print_i32") (param i32))

  (global $width (mut i32) (i32.const 0))
  (global $depth (mut i32) (i32.const 0))

  (elem declare func $main $thread1 $thread2 $thread3)

  (func $main
    (call $log (i32.const 0))
    (suspend $spawn (cont.new $cont (ref.func $thread1)))
    (call $log (i32.const 1))
    (suspend $spawn (cont.bind $pcont $cont (global.get $depth) (cont.new $pcont (ref.func $thread2))))
    (call $log (i32.const 2))
    (suspend $spawn (cont.new $cont (ref.func $thread3)))
    (call $log (i32.const 3))
  )

  (func $thread1
    (call $log (i32.const 10))
    (suspend $yield)
    (call $log (i32.const 11))
    (suspend $yield)
    (call $log (i32.const 12))
    (suspend $yield)
    (call $log (i32.const 13))
  )

  (func $thread2 (param $d i32)
    (local $w i32)
    (local.set $w (global.get $width))
    (call $log (i32.const 20))
    (br_if 0 (i32.eqz (local.get $d)))
    (call $log (i32.const 21))
    (loop $l
      (if (local.get $w)
        (then
          (call $log (i32.const 22))
          (suspend $yield)
          (call $log (i32.const 23))
          (suspend $spawn
            (cont.bind $pcont $cont
              (i32.sub (local.get $d) (i32.const 1))
              (cont.new $pcont (ref.func $thread2))
            )
          )
          (call $log (i32.const 24))
          (local.set $w (i32.sub (local.get $w) (i32.const 1)))
          (br $l)
        )
      )
    )
    (call $log (i32.const 25))
  )

  (func $thread3
    (call $log (i32.const 30))
    (suspend $yield)
    (call $log (i32.const 31))
    (suspend $yield)
    (call $log (i32.const 32))
  )

  (func (export "run") (param $width i32) (param $depth i32)
    (global.set $depth (local.get $depth))
    (global.set $width (local.get $width))
    (call $log (i32.const -1))
    (call $scheduler (cont.new $cont (ref.func $main)))
    (call $log (i32.const -2))
  )
)

(assert_return (invoke "run" (i32.const 0) (i32.const 0)))
(assert_return (invoke "run" (i32.const 0) (i32.const 1)))
(assert_return (invoke "run" (i32.const 1) (i32.const 0)))
(assert_return (invoke "run" (i32.const 1) (i32.const 1)))
(assert_return (invoke "run" (i32.const 3) (i32.const 4)))


;; Nested example: generator in a thread

(module $concurrent_generator
  (func $log (import "spectest" "print_i64") (param i64))

  (tag $syield (import "scheduler" "yield"))
  (tag $spawn (import "scheduler" "spawn") (param (ref $cont)))
  (func $scheduler (import "scheduler" "scheduler") (param $main (ref $cont)))

  (type $ghook (func (param i64)))
  (func $gsum (import "generator" "sum") (param i64 i64) (result i64))
  (global $ghook (import "generator" "hook") (mut (ref $ghook)))

  (global $result (mut i64) (i64.const 0))
  (global $done (mut i32) (i32.const 0))

  (elem declare func $main $bg-thread $syield)

  (func $syield (param $i i64)
    (call $log (local.get $i))
    (suspend $syield)
  )

  (func $bg-thread
    (call $log (i64.const -10))
    (loop $l
      (call $log (i64.const -11))
      (suspend $syield)
      (br_if $l (i32.eqz (global.get $done)))
    )
    (call $log (i64.const -12))
  )

  (func $main (param $i i64) (param $j i64)
    (suspend $spawn (cont.new $cont (ref.func $bg-thread)))
    (global.set $ghook (ref.func $syield))
    (global.set $result (call $gsum (local.get $i) (local.get $j)))
    (global.set $done (i32.const 1))
  )

  (type $proc (func))
  (type $pproc (func (param i64 i64)))
  (type $cont (cont $proc))
  (type $pcont (cont $pproc))
  (func (export "sum") (param $i i64) (param $j i64) (result i64)
    (call $log (i64.const -1))
    (call $scheduler
      (cont.bind $pcont $cont (local.get $i) (local.get $j) (cont.new $pcont (ref.func $main)))
    )
    (call $log (i64.const -2))
    (global.get $result)
  )
)

(assert_return (invoke "sum" (i64.const 10) (i64.const 20)) (i64.const 165))


;; cont.bind

(module
  (type $f2 (func (param i32 i32) (result i32 i32 i32 i32 i32 i32)))
  (type $f4 (func (param i32 i32 i32 i32) (result i32 i32 i32 i32 i32 i32)))
  (type $f6 (func (param i32 i32 i32 i32 i32 i32) (result i32 i32 i32 i32 i32 i32)))

  (type $k2 (cont $f2))
  (type $k4 (cont $f4))
  (type $k6 (cont $f6))

  (elem declare func $f)
  (func $f (param i32 i32 i32 i32 i32 i32) (result i32 i32 i32 i32 i32 i32)
    (local.get 0) (local.get 1) (local.get 2)
    (local.get 3) (local.get 4) (local.get 5)
  )

  (func (export "run") (result i32 i32 i32 i32 i32 i32)
    (local $k6 (ref null $k6))
    (local $k4 (ref null $k4))
    (local $k2 (ref null $k2))
    (local.set $k6 (cont.new $k6 (ref.func $f)))
    (local.set $k4 (cont.bind $k6 $k4 (i32.const 1) (i32.const 2) (local.get $k6)))
    (local.set $k2 (cont.bind $k4 $k2 (i32.const 3) (i32.const 4) (local.get $k4)))
    (resume $k2 (i32.const 5) (i32.const 6) (local.get $k2))
  )
)

(assert_return (invoke "run")
  (i32.const 1) (i32.const 2) (i32.const 3)
  (i32.const 4) (i32.const 5) (i32.const 6)
)


(module
  (tag $e (result i32 i32 i32 i32 i32 i32))

  (type $f0 (func (result i32 i32 i32 i32 i32 i32 i32)))
  (type $f2 (func (param i32 i32) (result i32 i32 i32 i32 i32 i32 i32)))
  (type $f4 (func (param i32 i32 i32 i32) (result i32 i32 i32 i32 i32 i32 i32)))
  (type $f6 (func (param i32 i32 i32 i32 i32 i32) (result i32 i32 i32 i32 i32 i32 i32)))

  (type $k0 (cont $f0))
  (type $k2 (cont $f2))
  (type $k4 (cont $f4))
  (type $k6 (cont $f6))

  (elem declare func $f)
  (func $f (result i32 i32 i32 i32 i32 i32 i32)
    (i32.const 0) (suspend $e)
  )

  (func (export "run") (result i32 i32 i32 i32 i32 i32 i32)
    (local $k6 (ref null $k6))
    (local $k4 (ref null $k4))
    (local $k2 (ref null $k2))
    (block $l (result (ref $k6))
      (resume $k0 (on $e $l) (cont.new $k0 (ref.func $f)))
      (unreachable)
    )
    (local.set $k6)
    (local.set $k4 (cont.bind $k6 $k4 (i32.const 1) (i32.const 2) (local.get $k6)))
    (local.set $k2 (cont.bind $k4 $k2 (i32.const 3) (i32.const 4) (local.get $k4)))
    (resume $k2 (i32.const 5) (i32.const 6) (local.get $k2))
  )
)

(assert_return (invoke "run")
  (i32.const 0) (i32.const 1) (i32.const 2) (i32.const 3)
  (i32.const 4) (i32.const 5) (i32.const 6)
)

;; Subtyping
(module
  (type $ft1 (func (param i32)))
  (type $ct1 (sub (cont $ft1)))

  (type $ft0 (func))
  (type $ct0 (sub (cont $ft0)))

  (func $test (param $x (ref $ct1))
    (i32.const 123)
    (local.get $x)
    (cont.bind $ct1 $ct0)
    (drop)
  )
)

(module
    (type $f1 (sub (func (result anyref))))
    (type $f2 (sub $f1 (func (result eqref))))
    (type $c1 (sub (cont $f1)))
    (type $c2 (sub $c1 (cont $f2)))
)

;; Globals
(module
  (type $ft (func))
  (type $ct (cont $ft))

  (global $k (mut (ref null $ct)) (ref.null $ct))
  (global $g (ref null $ct) (ref.null $ct))

  (func $f)
  (elem declare func $f)

  (func (export "set-global")
    (global.set $k (cont.new $ct (ref.func $f))))
)
(assert_return (invoke "set-global"))

;; Switch
(module
  (rec
    (type $ft (func (param (ref null $ct))))
    (type $ct (cont $ft)))

  (func $print-i32 (import "spectest" "print_i32") (param i32))

  (global $fi (mut i32) (i32.const 0))
  (global $gi (mut i32) (i32.const 1))

  (tag $swap)

  (func $init (export "init") (result i32)
    (resume $ct (on $swap switch)
      (cont.new $ct (ref.func $g))
      (cont.new $ct (ref.func $f)))
    (return (i32.const 42)))
  (func $f (type $ft)
    (local $nextk (ref null $ct))
    (local.set $nextk (local.get 0))
    (call $print-i32 (global.get $fi))
    (switch $ct $swap (local.get $nextk))
    (local.set $nextk)
    (call $print-i32 (global.get $fi))
    (switch $ct $swap (local.get $nextk))
    (drop))
  (func $g (type $ft)
    (local $nextk (ref null $ct))
    (local.set $nextk (local.get 0))
    (call $print-i32 (global.get $gi))
    (switch $ct $swap (local.get $nextk))
    (local.set $nextk)
    (call $print-i32 (global.get $gi)))
  (elem declare func $f $g)
)
(assert_return (invoke "init") (i32.const 42))

(module
  (rec
    (type $ft (func (param i32) (param (ref null $ct)) (result i32)))
    (type $ct (cont $ft)))

  (func $print-i32 (import "spectest" "print_i32") (param i32))

  (tag $swap (result i32))

  (func $init (export "init") (result i32)
    (resume $ct (on $swap switch)
      (i32.const 1)
      (cont.new $ct (ref.func $g))
      (cont.new $ct (ref.func $f))))
  (func $f (type $ft)
    (local $i i32)
    (local $nextk (ref null $ct))
    (local.set $i (local.get 0))
    (local.set $nextk (local.get 1))
    (call $print-i32 (local.get $i))
    (switch $ct $swap (i32.add (i32.const 1) (local.get $i)) (local.get $nextk))
    (local.set $nextk)
    (local.set $i)
    (call $print-i32 (local.get $i))
    (switch $ct $swap (i32.add (i32.const 1) (local.get $i)) (local.get $nextk))
    (unreachable))
  (func $g (type $ft)
    (local $i i32)
    (local $nextk (ref null $ct))
    (local.set $i (local.get 0))
    (local.set $nextk (local.get 1))
    (call $print-i32 (local.get $i))
    (switch $ct $swap (i32.add (i32.const 1) (local.get $i)) (local.get $nextk))
    (local.set $nextk)
    (local.set $i)
    (call $print-i32 (local.get $i))
    (return (local.get $i)))
  (elem declare func $f $g)
)
(assert_return (invoke "init") (i32.const 4))


(assert_invalid
  (module
    (rec
      (type $ft (func (param (ref null $ct))))
      (type $ct (cont $ft)))
    (type $ft2 (func))
    (type $ct2 (cont $ft2))

    (tag $swap)
    (func $f (type $ft)
      (switch $ct $swap (cont.new $ct2 (ref.null $ft2)))
      (drop)))
   "type mismatch")

(assert_invalid
  (module
    (rec
      (type $ft (func (param i32) (param (ref null $ct))))
      (type $ct (cont $ft)))

    (tag $swap)
    (func $f (type $ft)
      (switch $ct $swap (i64.const 0) (local.get 1))
      (drop)
      (drop)))
   "type mismatch")

(module
  (type $ft1 (func))
  (type $ct1 (cont $ft1))
  (rec
    (type $ft2 (func (param (ref null $ct2))))
    (type $ct2 (cont $ft2)))

  (tag $t)

  (func $suspend (type $ft2)
    (suspend $t))

  (func $switch (type $ft2)
    (switch $ct2 $t (local.get 0))
    (drop))

  (func (export "unhandled-suspend-t")
    (resume $ct2 (on $t switch)
      (cont.new $ct2 (ref.func $suspend))
      (cont.new $ct2 (ref.func $suspend))))
  (func (export "unhandled-switch-t")
    (block $l (result (ref $ct1))
      (resume $ct2 (on $t $l)
        (cont.new $ct2 (ref.func $switch))
        (cont.new $ct2 (ref.func $switch)))
      (unreachable)
    )
    (unreachable))

  (elem declare func $suspend $switch)
)
(assert_suspension (invoke "unhandled-suspend-t") "unhandled tag")
(assert_suspension (invoke "unhandled-switch-t") "unhandled tag")

(module
  (rec
    (type $ft (func (param (ref null $ct))))
    (type $ct (cont $ft)))

  (tag $t)

  (func
    (cont.new $ct (ref.null $ft))
    (unreachable))
  (func
    (cont.bind $ct $ct (ref.null $ct))
    (unreachable))
  (func
    (resume $ct (ref.null $ct) (ref.null $ct))
    (unreachable))
  (func
    (resume_throw $ct $t (ref.null $ct))
    (unreachable))
  (func
    (switch $ct $t (ref.null $ct))
    (unreachable))
)

(module $co2
  (type $task (func (result i32))) ;; type alias task = [] -> []
  (type $ct   (cont $task)) ;; type alias   ct = $task
  (tag $pause (export "pause"))   ;; pause : [] -> []
  (tag $cancel (export "cancel"))   ;; cancel : [] -> []
  ;; run : [(ref $task) (ref $task)] -> []
  ;; implements a 'seesaw' (c.f. Ganz et al. (ICFP@99))
  (func $run (export "seesaw") (param $up (ref $ct)) (param $down (ref $ct)) (result i32)
    (local $result i32)
    ;; run $up
    (loop $run_next (result i32)
      (block $on_pause (result (ref $ct))
        (resume $ct (on $pause $on_pause)
                    (local.get $up))
        ;; $up finished, store its result
        (local.set $result)
        ;; next cancel $down
        (block $on_cancel
          (try_table (catch $cancel $on_cancel)
            ;; inject the cancel exception into $down
            (resume_throw $ct $cancel (local.get $down))
            (drop) ;; drop the return value if it handled $cancel
                   ;; itself and returned normally...
          )
        ) ;; ... otherwise catch $cancel and return $up's result.
        (return (local.get $result))
      ) ;; on_pause clause, stack type: [(cont $ct)]
      (local.set $up)
      ;; swap $up and $down
      (local.get $down)
      (local.set $down (local.get $up))
      (local.set $up)
      (br $run_next)
    )
  )
)
(register "co2")

(module $client
  (type $task-0 (func (param i32) (result i32)))
  (type $ct-0 (cont $task-0))
  (type $task (func (result i32)))
  (type $ct (cont $task))

  (func $seesaw (import "co2" "seesaw") (param (ref $ct)) (param (ref $ct)) (result i32))
  (func $print-i32 (import "spectest" "print_i32") (param i32))
  (tag $pause (import "co2" "pause"))

  (func $even (param $niter i32) (result i32)
     (local $next i32) ;; zero initialised.
     (local $i i32)
     (loop $print-next
       (call $print-i32 (local.get $next))
       (suspend $pause)
       (local.set $next (i32.add (local.get $next) (i32.const 2)))
       (local.set $i (i32.add (local.get $i) (i32.const 1)))
       (br_if $print-next (i32.lt_u (local.get $i) (local.get $niter)))
     )
     (local.get $next)
  )
  (func $odd (param $niter i32) (result i32)
     (local $next i32) ;; zero initialised.
     (local $i i32)
     (local.set $next (i32.const 1))
     (loop $print-next
       (call $print-i32 (local.get $next))
       (suspend $pause)
       (local.set $next (i32.add (local.get $next) (i32.const 2)))
       (local.set $i (i32.add (local.get $i) (i32.const 1)))
       (br_if $print-next (i32.lt_u (local.get $i) (local.get $niter)))
     )
     (local.get $next)
  )

  (func (export "main") (result i32)
    (call $seesaw
       (cont.bind $ct-0 $ct
         (i32.const 5) (cont.new $ct-0 (ref.func $even)))
       (cont.bind $ct-0 $ct
         (i32.const 5) (cont.new $ct-0 (ref.func $odd)))))

  (elem declare func $even $odd)
)
(assert_return (invoke "main") (i32.const 10))

(module
  (type $f1 (func (result i32)))
  (type $c1 (cont $f1))
  (type $f2 (func (param (ref null $c1)) (result i32)))
  (type $c2 (cont $f2))
  (type $f3 (func (param (ref null $c2)) (result i32)))
  (type $c3 (cont $f3))
  (tag $e (result i32))

  (func $fn_1 (param (ref null $c2)) (result i32)
    (local.get 0)
    (switch $c2 $e)
    (i32.const 24)
  )
  (elem declare func $fn_1)

  (func $fn_2 (result i32)
    (cont.new $c3 (ref.func $fn_1))
    (switch $c3 $e)
    (drop)
    (i32.const -1)
  )
  (elem declare func $fn_2)

  (func (export "main") (result i32)
    (cont.new $c1 (ref.func $fn_2))
    (resume $c1 (on $e switch))
  )
)

(assert_return (invoke "main") (i32.const -1))

;; Syntax: check unfolded forms
(module
  (type $ft (func))
  (type $ct (cont $ft))
  (rec
    (type $ft2 (func (param (ref null $ct2))))
    (type $ct2 (cont $ft2)))

  (tag $yield (param i32))
  (tag $swap)

  ;; Check cont.new
  (func (result (ref $ct))
    ref.null $ft
    block (param (ref null $ft)) (result (ref $ct))
      cont.new $ct
    end
  )
  ;; Check cont.bind
  (func (param (ref $ct)) (result (ref $ct))
    local.get 0
    block (param (ref $ct)) (result (ref $ct))
      cont.bind $ct $ct
    end
  )
  ;; Check suspend
  (func
    block
      suspend $swap
    end
  )
  ;; Check resume
  (func (param $k (ref $ct)) (result i32)
    (local.get $k)
    block $on_yield (param (ref $ct)) (result i32 (ref $ct))
      resume $ct (on $yield $on_yield)
      i32.const 42
      return
    end
    local.set $k
  )
  ;; Check resume_throw
  (func (param $k (ref $ct)) (result i32)
    block $on_yield (result i32 (ref $ct))
      i32.const 42
      local.get $k
      resume_throw $ct $yield
      i32.const 42
      return
    end
    local.set $k
  )
  ;; Check switch
  (func (param $k (ref $ct2))
    local.get $k
    block (param (ref $ct2)) (result (ref null $ct2))
      switch $ct2 $swap
    end
    drop
  )
)

;; Syntax: check instructions in tail position in unfolded form
(module
  (type $ft (func))
  (type $ct (cont $ft))
  (rec
    (type $ft2 (func (param (ref null $ct2))))
    (type $ct2 (cont $ft2)))

  (tag $yield (param i32))
  (tag $swap)

  ;; Check cont.new
  (func (result (ref $ct))
    ref.null $ft
    cont.new $ct
  )
  ;; Check cont.bind
  (func (param (ref $ct)) (result (ref $ct))
    local.get 0
    cont.bind $ct $ct
  )

  ;; Check resume
  (func (;2;) (param $k (ref $ct))
    local.get $k
    resume $ct
  )
  ;; Check resume_throw
  (func (param $k (ref $ct))
    i32.const 42
    local.get $k
    resume_throw $ct $yield
  )
  ;; Check switch
  (func (param $k (ref $ct2)) (result (ref null $ct2))
    local.get $k
    switch $ct2 $swap
  )
  ;; Check suspend
  (func
    suspend $swap
  )
)

(module
  (type $ft0 (func))
  (type $ct0 (cont $ft0))

  (type $ft1 (func (param (ref $ct0))))
  (type $ct1 (cont $ft1))

  (tag $t)

  (func $f
    (cont.new $ct1 (ref.func $g))
    (switch $ct1 $t)
  )
  (elem declare func $f)

  (func $g (param (ref $ct0)))
  (elem declare func $g)

  (func $entry
    (cont.new $ct0 (ref.func $f))
    (resume $ct0 (on $t switch))
  )
)

(assert_invalid
  (module
    (rec
      (type $ft (func (param (ref $ct))))
      (type $ct (cont $ft)))
    (tag $t (param i32))

    (func (param $k (ref $ct))
      (switch $ct $t)))
  "type mismatch: non-empty tag parameter type")

;; Synthesized from https://github.com/WebAssembly/stack-switching/issues/117
(assert_invalid
  (module
    (type $ft (func))
    (type $ct (cont $ft))
    (tag $t)

    (func
      (block $on_t (result (ref cont))
        (resume $ct (on $t $on_t) (cont.new $ct (ref.null $ft)))
        (unreachable)
      )
      (drop)
    ))
  "type mismatch: (ref cont)")

(assert_invalid
  (module
    (type $ft (func))
    (type $ct (cont $ft))
    (tag $t)

    (func
      (block $on_t (result (ref nocont))
        (resume $ct (on $t $on_t) (cont.new $ct (ref.null $ft)))
        (unreachable)
      )
      (drop)
    ))
  "type mismatch: (ref nocont)")

;; https://github.com/WebAssembly/stack-switching/issues/117#issuecomment-2908974084
(module
  (type $f (func))
  (type $c (sub (cont $f)))
  (tag $e)
  (func (param $c (ref $c))
    (local.get $c)
    (resume $c)
  )
)
