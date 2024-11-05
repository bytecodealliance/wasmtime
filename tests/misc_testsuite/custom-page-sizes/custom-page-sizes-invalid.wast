;;! custom_page_sizes = true
;;! multi_memory = true

;; Page size that is not a power of two.
(assert_malformed
  (module quote "(memory 0 (pagesize 3))")
  "invalid custom page size"
)

;; Power-of-two page sizes that are not 1 or 64KiB.
(assert_invalid
  (module (memory 0 (pagesize 2)))
  "invalid custom page size"
)
(assert_invalid
  (module (memory 0 (pagesize 4)))
  "invalid custom page size"
)
(assert_invalid
  (module (memory 0 (pagesize 8)))
  "invalid custom page size"
)
(assert_invalid
  (module (memory 0 (pagesize 16)))
  "invalid custom page size"
)
(assert_invalid
  (module (memory 0 (pagesize 32)))
  "invalid custom page size"
)
(assert_invalid
  (module (memory 0 (pagesize 64)))
  "invalid custom page size"
)
(assert_invalid
  (module (memory 0 (pagesize 128)))
  "invalid custom page size"
)
(assert_invalid
  (module (memory 0 (pagesize 256)))
  "invalid custom page size"
)
(assert_invalid
  (module (memory 0 (pagesize 512)))
  "invalid custom page size"
)
(assert_invalid
  (module (memory 0 (pagesize 1024)))
  "invalid custom page size"
)
(assert_invalid
  (module (memory 0 (pagesize 2048)))
  "invalid custom page size"
)
(assert_invalid
  (module (memory 0 (pagesize 4096)))
  "invalid custom page size"
)
(assert_invalid
  (module (memory 0 (pagesize 8192)))
  "invalid custom page size"
)
(assert_invalid
  (module (memory 0 (pagesize 16384)))
  "invalid custom page size"
)
(assert_invalid
  (module (memory 0 (pagesize 32768)))
  "invalid custom page size"
)

;; Power-of-two page size that is larger than 64KiB.
(assert_invalid
  (module (memory 0 (pagesize 0x20000)))
  "invalid custom page size"
)

;; Power of two page size that cannot fit in a u64 to exercise checks against
;; shift overflow.
(assert_malformed
  (module binary
    "\00asm" "\01\00\00\00"
    "\05\04\01"                ;; Memory section

    ;; memory 0
    "\08"                      ;; flags w/ custom page size
    "\00"                      ;; minimum = 0
    "\41"                      ;; pagesize = 2**65
  )
  "invalid custom page size"
)

;; Importing a memory with the wrong page size.

(module $m
  (memory (export "small-pages-memory") 0 (pagesize 1))
  (memory (export "large-pages-memory") 0 (pagesize 65536))
)
(register "m" $m)

(assert_unlinkable
  (module
    (memory (import "m" "small-pages-memory") 0 (pagesize 65536))
  )
  "memory types incompatible"
)

(assert_unlinkable
  (module
    (memory (import "m" "large-pages-memory") 0 (pagesize 1))
  )
  "memory types incompatible"
)
