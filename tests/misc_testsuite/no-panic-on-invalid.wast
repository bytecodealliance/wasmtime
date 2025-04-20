
(assert_malformed
  (module binary
    "\00asm\01\00\00\00" ;; version header

    "\01\06"          ;; type section, 6 bytes
    "\01"             ;; 1 type
    "\60\01\7f\01\7f" ;; function type, 1 i32 parameter, 1 i32 result


    "\03\02"          ;; function section, 2 bytes
    "\01\00"          ;; 1 function, type 0

    "\0a\14"          ;; code section, 20 bytes
    "\01"             ;; 1 function
    "\12"             ;; 18-byte function
    "\00"             ;; no locals
    "\41\00"          ;; i32.const 0
    "\41\00"          ;; i32.const 0
    "\0d\00"          ;; br_if 0
    "\41\00"          ;; i32.const 0
    "\0f"             ;; return
    "\0b"             ;; end

    ;; operator-wise this function is now done, but the invalid part of this
    ;; continues going and adds more instructions
    "\02\40"          ;; block
    "\41\00"          ;; i32.const 0
    "\0f"             ;; return
    "\0b"             ;; end

    ;; pretend this is the actual function end
    "\0b"             ;; end
  )
  "hello")
