;;! shared_everything_threads = true
;;! reference_types = true
(component
  (core type $start (shared (func (param $context i32))))
  (core module $libc (table (export "start-table") shared 1 (ref null (shared func))))
  (core instance $libc (instantiate $libc))
  (core func $spawn_indirect (canon thread.spawn_indirect $start (table $libc "start-table")))
)
