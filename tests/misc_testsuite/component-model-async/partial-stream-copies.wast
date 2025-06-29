;;! component_model_async = true
;;! reference_types = true
;;! multi_memory = true
;;! gc_types = true

;; This test has two components $C and $D, where $D imports and calls $C.transform
;;  $C.transform takes and returns a stream<u8>
;;  Before $C.transform blocks the first time, it supplies a 12-byte read buffer
;;  When $D.run regains control after $C.transform blocks, it can perform multiple
;;   successful writes until it fully uses up the 12-byte buffer.
;;   ... and that's where I am so far ...
;;
;; (Copied from
;; https://github.com/WebAssembly/component-model/blob/add-tests/test/concurrency/partial-stream-copies.wast)
(component
  (component $C
    (core module $Memory (memory (export "mem") 1))
    (core instance $memory (instantiate $Memory))
    (core module $CM
      (import "" "mem" (memory 1))
      (import "" "task.return" (func $task.return (param i32)))
      (import "" "waitable.join" (func $waitable.join (param i32 i32)))
      (import "" "waitable-set.new" (func $waitable-set.new (result i32)))
      (import "" "stream.new" (func $stream.new (result i64)))
      (import "" "stream.read" (func $stream.read (param i32 i32 i32) (result i32)))
      (import "" "stream.write" (func $stream.write (param i32 i32 i32) (result i32)))
      (import "" "stream.drop-readable" (func $stream.drop-readable (param i32)))
      (import "" "stream.drop-writable" (func $stream.drop-writable (param i32)))

      ;; $ws is waited on by 'transform'
      (global $ws (mut i32) (i32.const 0))
      (func $start (global.set $ws (call $waitable-set.new)))
      (start $start)

      ;; $insr/$outsw are read/written by 'transform'
      (global $insr (mut i32) (i32.const 0))
      (global $inbufp (mut i32) (i32.const 0x10))
      (global $outsw (mut i32) (i32.const 0))
      (global $outbufp (mut i32) (i32.const 0x20))

      (func $transform (export "transform") (param i32) (result i32)
        (local $ret i32) (local $ret64 i64) (local $outsr i32)

        ;; check the incoming readable stream end
        (global.set $insr (local.get 0))
        (if (i32.ne (i32.const 2) (global.get $insr))
          (then unreachable))

        ;; create a new stream r/w pair $outsr/$outsw
        (local.set $ret64 (call $stream.new))
        (local.set $outsr (i32.wrap_i64 (local.get $ret64)))
        (if (i32.ne (i32.const 3) (local.get $outsr))
          (then unreachable))
        (global.set $outsw (i32.wrap_i64 (i64.shr_u (local.get $ret64) (i64.const 32))))
        (if (i32.ne (i32.const 4) (global.get $outsw))
          (then unreachable))

        ;; start async read on $insr which will block
        (local.set $ret (call $stream.read (global.get $insr) (global.get $inbufp) (i32.const 12)))
        (if (i32.ne (i32.const -1 (; BLOCKED ;)) (local.get $ret))
          (then unreachable))

        ;; return the readable end of the outgoing stream to the caller
        (call $task.return (local.get $outsr))

        ;; wait for the stream.read/write to complete
        (call $waitable.join (global.get $insr) (global.get $ws))
        (call $waitable.join (global.get $outsw) (global.get $ws))
        (i32.or (i32.const 2 (; WAIT ;)) (i32.shl (global.get $ws) (i32.const 4)))
      )
      (func $transform_cb (export "transform_cb") (param $event_code i32) (param $index i32) (param $payload i32) (result i32)
        (local $ret i32) (local $ret64 i64)

        ;; confirm the read succeeded fully
        (if (i32.ne (local.get $event_code) (i32.const 2 (; STREAM_READ ;)))
          (then unreachable))
        (if (i32.ne (local.get $index) (global.get $insr))
          (then unreachable))
        (if (i32.ne (local.get $payload) (i32.const 0xc0 (; COMPLETED=0 | (12 << 4) ;)))
          (then unreachable))
        (if (i32.ne (i32.const 0x89abcdef) (i32.load offset=0 (global.get $inbufp)))
          (then unreachable))
        (if (i32.ne (i32.const 0x01234567) (i32.load offset=4 (global.get $inbufp)))
          (then unreachable))
        (if (i32.ne (i32.const 0x89abcdef) (i32.load offset=8 (global.get $inbufp)))
          (then unreachable))

        ;; multiple read calls succeed until 12-byte buffer is consumed
        (local.set $ret (call $stream.read (global.get $insr) (global.get $inbufp) (i32.const 4)))
        (if (i32.ne (i32.const 0x40) (local.get $ret))
          (then unreachable))
        (if (i32.ne (i32.const 0x76543210) (i32.load (global.get $inbufp)))
          (then unreachable))
        (local.set $ret (call $stream.read (global.get $insr) (global.get $inbufp) (i32.const 2)))
        (if (i32.ne (i32.const 0x20) (local.get $ret))
          (then unreachable))
        (if (i32.ne (i32.const 0xba98) (i32.load16_u (global.get $inbufp)))
          (then unreachable))
        (local.set $ret (call $stream.read (global.get $insr) (global.get $inbufp) (i32.const 8)))
        (if (i32.ne (i32.const 0x60) (local.get $ret))
          (then unreachable))
        (if (i32.ne (i32.const 0x3210fedc) (i32.load (global.get $inbufp)))
          (then unreachable))
        (if (i32.ne (i32.const 0x7654) (i32.load16_u offset=4 (global.get $inbufp)))
          (then unreachable))

        (call $stream.drop-readable (global.get $insr))
        (call $stream.drop-writable (global.get $outsw))
        (return (i32.const 0 (; EXIT ;)))
      )
    )
    (type $ST (stream u8))
    (canon task.return (result $ST) (memory $memory "mem") (core func $task.return))
    (canon waitable.join (core func $waitable.join))
    (canon waitable-set.new (core func $waitable-set.new))
    (canon stream.new $ST (core func $stream.new))
    (canon stream.read $ST async (memory $memory "mem") (core func $stream.read))
    (canon stream.write $ST async (memory $memory "mem") (core func $stream.write))
    (canon stream.drop-readable $ST (core func $stream.drop-readable))
    (canon stream.drop-writable $ST (core func $stream.drop-writable))
    (core instance $cm (instantiate $CM (with "" (instance
      (export "mem" (memory $memory "mem"))
      (export "task.return" (func $task.return))
      (export "waitable.join" (func $waitable.join))
      (export "waitable-set.new" (func $waitable-set.new))
      (export "stream.new" (func $stream.new))
      (export "stream.read" (func $stream.read))
      (export "stream.write" (func $stream.write))
      (export "stream.drop-readable" (func $stream.drop-readable))
      (export "stream.drop-writable" (func $stream.drop-writable))
    ))))
    (func (export "transform") (param "in" (stream u8)) (result (stream u8)) (canon lift
      (core func $cm "transform")
      async (memory $memory "mem") (callback (func $cm "transform_cb"))
    ))
  )

  (component $D
    (import "transform" (func $transform (param "in" (stream u8)) (result (stream u8))))

    (core module $Memory (memory (export "mem") 1))
    (core instance $memory (instantiate $Memory))
    (core module $DM
      (import "" "mem" (memory 1))
      (import "" "waitable.join" (func $waitable.join (param i32 i32)))
      (import "" "waitable-set.new" (func $waitable-set.new (result i32)))
      (import "" "waitable-set.wait" (func $waitable-set.wait (param i32 i32) (result i32)))
      (import "" "stream.new" (func $stream.new (result i64)))
      (import "" "stream.read" (func $stream.read (param i32 i32 i32) (result i32)))
      (import "" "stream.write" (func $stream.write (param i32 i32 i32) (result i32)))
      (import "" "stream.drop-readable" (func $stream.drop-readable (param i32)))
      (import "" "stream.drop-writable" (func $stream.drop-writable (param i32)))
      (import "" "transform" (func $transform (param i32 i32) (result i32)))

      (func $run (export "run") (result i32)
        (local $ret i32) (local $ret64 i64) (local $retp i32)
        (local $insr i32) (local $insw i32) (local $outsr i32)
        (local $subtask i32)
        (local $ws i32)

        ;; create a new stream r/w pair $insr/$insw
        (local.set $ret64 (call $stream.new))
        (local.set $insr (i32.wrap_i64 (local.get $ret64)))
        (if (i32.ne (i32.const 1) (local.get $insr))
          (then unreachable))
        (local.set $insw (i32.wrap_i64 (i64.shr_u (local.get $ret64) (i64.const 32))))
        (if (i32.ne (i32.const 2) (local.get $insw))
          (then unreachable))

        ;; call 'transform' which will return a readable stream $outsr eagerly
        (local.set $retp (i32.const 8))
        (local.set $ret (call $transform (local.get $insr) (local.get $retp)))
        (if (i32.ne (i32.const 2 (; RETURNED=2 | (0<<4) ;)) (local.get $ret))
          (then unreachable))
        (local.set $outsr (i32.load (local.get $retp)))
        (if (i32.ne (i32.const 1) (local.get $outsr))
          (then unreachable))

        ;; multiple write calls succeed until 12-byte buffer is filled
        (i64.store (i32.const 16) (i64.const 0x0123456789abcdef))
        (local.set $ret (call $stream.write (local.get $insw) (i32.const 16) (i32.const 8)))
        (if (i32.ne (i32.const 0x80) (local.get $ret))
          (then unreachable))
        (local.set $ret (call $stream.write (local.get $insw) (i32.const 16) (i32.const 8)))
        (if (i32.ne (i32.const 0x40) (local.get $ret))
          (then unreachable))

        ;; start a blocking write with a 12-byte buffer
        (i64.store (i32.const 16) (i64.const 0xfedcba9876543210))
        (i32.store (i32.const 24) (i32.const 0x76543210))
        (local.set $ret (call $stream.write (local.get $insw) (i32.const 16) (i32.const 12)))
        (if (i32.ne (i32.const -1 (; BLOCKED ;)) (local.get $ret))
          (then unreachable))

        ;; wait for transform to read our write and drop all the streams
        (local.set $ws (call $waitable-set.new))
        (call $waitable.join (local.get $insw) (local.get $ws))
        (local.set $ret (call $waitable-set.wait (local.get $ws) (i32.const 0)))

        ;; confirm the write and the dropped stream
        (if (i32.ne (i32.const 3 (; STREAM_WRITE ;)) (local.get $ret))
          (then unreachable))
        (if (i32.ne (local.get $insw) (i32.load (i32.const 0)))
          (then unreachable))
        (if (i32.ne (i32.const 0xc1 (; DROPPED=1 | (12 << 4) ;) (; TODO: currently returns 0xc0 ;)) (i32.load (i32.const 4)))
          (then unreachable))

        (call $stream.drop-writable (local.get $insw))
        (call $stream.drop-readable (local.get $outsr))

        ;; return 42 to the top-level test harness
        (i32.const 42)
      )
    )
    (type $ST (stream u8))
    (canon waitable.join (core func $waitable.join))
    (canon waitable-set.new (core func $waitable-set.new))
    (canon waitable-set.wait (memory $memory "mem") (core func $waitable-set.wait))
    (canon stream.new $ST (core func $stream.new))
    (canon stream.read $ST async (memory $memory "mem") (core func $stream.read))
    (canon stream.write $ST async (memory $memory "mem") (core func $stream.write))
    (canon stream.drop-readable $ST (core func $stream.drop-readable))
    (canon stream.drop-writable $ST (core func $stream.drop-writable))
    (canon lower (func $transform) async (memory $memory "mem") (core func $transform'))
    (core instance $dm (instantiate $DM (with "" (instance
      (export "mem" (memory $memory "mem"))
      (export "waitable.join" (func $waitable.join))
      (export "waitable-set.new" (func $waitable-set.new))
      (export "waitable-set.wait" (func $waitable-set.wait))
      (export "stream.new" (func $stream.new))
      (export "stream.read" (func $stream.read))
      (export "stream.write" (func $stream.write))
      (export "stream.drop-readable" (func $stream.drop-readable))
      (export "stream.drop-writable" (func $stream.drop-writable))
      (export "transform" (func $transform'))
    ))))
    (func (export "run") (result u32) (canon lift (core func $dm "run")))
  )

  (instance $c (instantiate $C))
  (instance $d (instantiate $D (with "transform" (func $c "transform"))))
  (func (export "run") (alias export $d "run"))
)
(assert_return (invoke "run") (u32.const 42))
