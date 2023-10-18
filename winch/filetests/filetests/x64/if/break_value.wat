;;! target = "x86_64"

(module
  (func (export "break-value") (param i32) (result i32)
    (if (result i32) (local.get 0)
      (then (br 0 (i32.const 18)) (i32.const 19))
      (else (br 0 (i32.const 21)) (i32.const 20))
    )
  )
)
;;    0:	 55                   	push	rbp
;;    1:	 4889e5               	mov	rbp, rsp
;;    4:	 4883ec10             	sub	rsp, 0x10
;;    8:	 897c240c             	mov	dword ptr [rsp + 0xc], edi
;;    c:	 4c893424             	mov	qword ptr [rsp], r14
;;   10:	 8b44240c             	mov	eax, dword ptr [rsp + 0xc]
;;   14:	 85c0                 	test	eax, eax
;;   16:	 0f840a000000         	je	0x26
;;   1c:	 b812000000           	mov	eax, 0x12
;;   21:	 e905000000           	jmp	0x2b
;;   26:	 b815000000           	mov	eax, 0x15
;;   2b:	 4883c410             	add	rsp, 0x10
;;   2f:	 5d                   	pop	rbp
;;   30:	 c3                   	ret	
