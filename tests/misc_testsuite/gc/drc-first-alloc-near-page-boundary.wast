;;! gc = true
;;! flags = "-C collector=drc -O gc-zeal-alloc-counter=1000"

;; FIXME(#15433): remove the `gc-zeal-alloc-counter=1000` workaround above when
;; the issue is fixed.

;; The first allocation in a store grows the GC heap. The heap must grow enough
;; for the allocation's rounded-up size plus the free list's reserved first
;; bytes, including for sizes just below a multiple of the 64 KiB page size.

(module
  (type $a (array (mut i8)))
  (func (export "alloc") (param i32) (result i32)
    (array.len (array.new_default $a (local.get 0)))))

(assert_return (invoke "alloc" (i32.const 65500)) (i32.const 65500))
