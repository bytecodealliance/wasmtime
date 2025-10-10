;;! component_model_async = true
;;! component_model_async_builtins = true

;; stream.cancel-read
(component
  (core module $m
    (import "" "stream.cancel-read" (func $stream-cancel-read (param i32) (result i32)))
  )
  (type $stream-type (stream u8))
  (core func $stream-cancel-read (canon stream.cancel-read $stream-type async))
  (core instance $i (instantiate $m (with "" (instance (export "stream.cancel-read" (func $stream-cancel-read))))))
)

;; stream.cancel-write
(component
  (core module $m
    (import "" "stream.cancel-write" (func $stream-cancel-write (param i32) (result i32)))
  )
  (type $stream-type (stream u8))
  (core func $stream-cancel-write (canon stream.cancel-write $stream-type async))
  (core instance $i (instantiate $m (with "" (instance (export "stream.cancel-write" (func $stream-cancel-write))))))
)

;; future.cancel-read
(component
  (core module $m
    (import "" "future.cancel-read" (func $future-cancel-read (param i32) (result i32)))
  )
  (type $future-type (future u8))
  (core func $future-cancel-read (canon future.cancel-read $future-type async))
  (core instance $i (instantiate $m (with "" (instance (export "future.cancel-read" (func $future-cancel-read))))))
)

;; future.cancel-write
(component
  (core module $m
    (import "" "future.cancel-write" (func $future-cancel-write (param i32) (result i32)))
  )
  (type $future-type (future u8))
  (core func $future-cancel-write (canon future.cancel-write $future-type async))
  (core instance $i (instantiate $m (with "" (instance (export "future.cancel-write" (func $future-cancel-write))))))
)
