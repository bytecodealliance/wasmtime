;;! component_model_async = true
;;! component_model_more_async_builtins = true
;;! reference_types = true

;; This test exercises corner cases where extremely large values are sent
;; between guests and currently require copying out to the host in Wasmtime
;; which should result in a trap of some form rather than the host spending all
;; its time allocating and copying memory.

(component definition $A
  (type $t (list (list (list (list u8)))))
  (type $s (stream $t))
  (type $f (future $t))
  (type $functy (func async (result $s)))

  (component $A
    (core module $libc (memory (export "memory") 1))
    (core instance $libc (instantiate $libc))

    (core module $m
      (import "libc" "memory" (memory 1))
      (import "" "stream.new" (func $stream.new (result i64)))
      (import "" "stream.write" (func $stream.write (param i32 i32 i32) (result i32)))
      (import "" "future.new" (func $future.new (result i64)))
      (import "" "future.write" (func $future.write (param i32 i32) (result i32)))
      (import "" "task.return future" (func $task.return-future (param i32)))
      (import "" "task.return stream" (func $task.return-stream (param i32)))

      (func (export "big-stream") (result i32)
        (local $w i32)
        (local $r i32)
        (local $s i64)
        (local.set $s (call $stream.new))
        (local.set $r (i32.wrap_i64 (local.get $s)))
        (local.set $w (i32.wrap_i64 (i64.shr_u (local.get $s) (i64.const 32))))

        (call $task.return-stream (local.get $r))

        local.get $w
        (call $prepare-list-to-write (i32.const 5) (i32.const 2))
        call $stream.write
        unreachable
      )

      (func (export "big-future") (result i32)
        (local $w i32)
        (local $r i32)
        (local $s i64)
        (local $base i32)
        (local $len i32)
        (local.set $s (call $future.new))
        (local.set $r (i32.wrap_i64 (local.get $s)))
        (local.set $w (i32.wrap_i64 (i64.shr_u (local.get $s) (i64.const 32))))

        (call $task.return-future (local.get $r))

        (call $prepare-list-to-write (i32.const 4) (i32.const 2))
        local.set $len
        local.set $base

        (i32.store offset=0 (i32.const 100) (local.get $base))
        (i32.store offset=4 (i32.const 100) (local.get $len))


        local.get $w
        i32.const 100
        call $future.write
        unreachable
      )

      ;; Prepare $depth+1 layers of lists where the leaves point to all of
      ;; memory and each layer otherwise is a list of the previous layer.
      ;;
      ;; Each layer-of-lists is `$pages` large.
      (func $prepare-list-to-write (param $depth i32) (param $pages i32) (result i32 i32)
        (local $base i32)
        (local $len i32)

        (local $c_base i32)
        (local $c_len i32)
        (local $i i32)

        local.get $depth
        if
          ;; Case of $depth>0 meaning that this is a list-of-lists layer.
          ;; Allocate some memory to store this list itself then generate the
          ;; layer down by recursing.
          (local.set $base (call $grow (local.get $pages)))
          (local.set $len
            (i32.div_u
              (i32.mul (local.get $pages) (i32.const 65536))
              (i32.const 8)
            )
          )

          (call $prepare-list-to-write
            (i32.sub (local.get $depth) (i32.const 1))
            (local.get $pages))
          local.set $c_len
          local.set $c_base

          ;; Initialize this list-of-lists with all copies of the previous
          ;; layer's list.
          loop $l
            (i32.store offset=0
              (i32.add (local.get $base) (i32.mul (local.get $i) (i32.const 8)))
              (local.get $c_base))
            (i32.store offset=4
              (i32.add (local.get $base) (i32.mul (local.get $i) (i32.const 8)))
              (local.get $c_len))

            (local.set $i (i32.add (local.get $i) (i32.const 1)))
            (if (i32.lt_u (local.get $i) (local.get $len))
              (then (br $l)))
          end

        else
          ;; base case: the bottom list is just a byte list of all of memory.
          (local.set $base (i32.const 0))
          (local.set $len (i32.mul (memory.size) (i32.const 65536)))
        end

        local.get $base
        local.get $len
      )

      (func $grow (param i32) (result i32)
        (local $r i32)
        (local.set $r (memory.grow (local.get 0)))
        local.get $r
        i32.const -1
        i32.eq
        if unreachable end
        local.get $r
        i32.const 65536
        i32.mul
      )

      (func (export "cb") (param i32 i32 i32) (result i32) unreachable)
    )
    (core func $future.new (canon future.new $f))
    (core func $future.write (canon future.write $f (memory $libc "memory")))
    (core func $stream.new (canon stream.new $s))
    (core func $stream.write (canon stream.write $s (memory $libc "memory")))
    (core func $task.return-future (canon task.return (result $f)))
    (core func $task.return-stream (canon task.return (result $s)))
    (core instance $m (instantiate $m
      (with "libc" (instance $libc))
      (with "" (instance
        (export "future.new" (func $future.new))
        (export "future.write" (func $future.write))
        (export "stream.new" (func $stream.new))
        (export "stream.write" (func $stream.write))
        (export "task.return future" (func $task.return-future))
        (export "task.return stream" (func $task.return-stream))
      ))
    ))

    (func (export "big-stream") (result $s)
      (canon lift (core func $m "big-stream") async
        (callback (func $m "cb"))))
    (func (export "big-future") (result $f)
      (canon lift (core func $m "big-future") async
        (callback (func $m "cb"))))
  )

  (component $B
    (import "a" (instance $a
      (export "big-future" (func (result $f)))
      (export "big-stream" (func (result $s)))
    ))

    (core module $libc
      (memory (export "memory") 1)
      (func (export "realloc") (param i32 i32 i32 i32) (result i32) unreachable)
    )
    (core instance $libc (instantiate $libc))

    (core module $m
      (import "libc" "memory" (memory 1))
      (import "" "big-stream" (func $big-stream (result i32)))
      (import "" "big-future" (func $big-future (result i32)))
      (import "" "stream.read" (func $stream.read (param i32 i32 i32) (result i32)))
      (import "" "future.read" (func $future.read (param i32 i32) (result i32)))

      (func (export "stream")
        (call $stream.read
          (call $big-stream)
          i32.const 0
          i32.const 100
        )
        unreachable
      )
      (func (export "future")
        (call $future.read
          (call $big-future)
          i32.const 0
        )
        unreachable
      )
    )
    (core func $big-stream (canon lower (func $a "big-stream")))
    (core func $big-future (canon lower (func $a "big-future")))
    (core func $stream.read
      (canon stream.read $s
        (memory $libc "memory")
        (realloc (func $libc "realloc"))
      )
    )
    (core func $future.read
      (canon future.read $f
        (memory $libc "memory")
        (realloc (func $libc "realloc"))
      )
    )
    (core instance $m (instantiate $m
      (with "libc" (instance $libc))
      (with "" (instance
        (export "big-stream" (func $big-stream))
        (export "big-future" (func $big-future))
        (export "future.read" (func $future.read))
        (export "stream.read" (func $stream.read))
      ))
    ))

    (func (export "stream") async (canon lift (core func $m "stream")))
    (func (export "future") async (canon lift (core func $m "future")))

  )

  (instance $a (instantiate $A))
  (instance $b (instantiate $B (with "a" (instance $a))))
  (export "stream" (func $b "stream"))
  (export "future" (func $b "future"))
)

(component instance $A $A)
(assert_trap (invoke "stream") "fuel allocated for hostcalls has been exhausted")
(component instance $A $A)
(assert_trap (invoke "future") "fuel allocated for hostcalls has been exhausted")
