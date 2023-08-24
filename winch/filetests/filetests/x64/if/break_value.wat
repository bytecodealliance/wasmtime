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
;;    c:	 4c89742404           	mov	qword ptr [rsp + 4], r14
;;   11:	 8b44240c             	mov	eax, dword ptr [rsp + 0xc]
;;   15:	 85c0                 	test	eax, eax
;;   17:	 0f840a000000         	je	0x27
;;   1d:	 b812000000           	mov	eax, 0x12
;;   22:	 e905000000           	jmp	0x2c
;;   27:	 b815000000           	mov	eax, 0x15
;;   2c:	 4883c410             	add	rsp, 0x10
;;   30:	 5d                   	pop	rbp
;;   31:	 c3                   	ret	
