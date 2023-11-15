;;! target = "x86_64"

(module
  (func $dummy)
  (func (export "as-if-condition") (param i32) (result i32)
    (if (result i32)
      (if (result i32) (local.get 0)
        (then (i32.const 1)) (else (i32.const 0))
      )
      (then (call $dummy) (i32.const 2))
      (else (call $dummy) (i32.const 3))
    )
  )
)
;;    0:	 55                   	push	rbp
;;    1:	 4889e5               	mov	rbp, rsp
;;    4:	 4883ec08             	sub	rsp, 8
;;    8:	 4c893424             	mov	qword ptr [rsp], r14
;;    c:	 4883c408             	add	rsp, 8
;;   10:	 5d                   	pop	rbp
;;   11:	 c3                   	ret	
;;
;;    0:	 55                   	push	rbp
;;    1:	 4889e5               	mov	rbp, rsp
;;    4:	 4883ec10             	sub	rsp, 0x10
;;    8:	 897c240c             	mov	dword ptr [rsp + 0xc], edi
;;    c:	 4c893424             	mov	qword ptr [rsp], r14
;;   10:	 8b44240c             	mov	eax, dword ptr [rsp + 0xc]
;;   14:	 85c0                 	test	eax, eax
;;   16:	 0f840a000000         	je	0x26
;;   1c:	 b801000000           	mov	eax, 1
;;   21:	 e905000000           	jmp	0x2b
;;   26:	 b800000000           	mov	eax, 0
;;   2b:	 85c0                 	test	eax, eax
;;   2d:	 0f840f000000         	je	0x42
;;   33:	 e800000000           	call	0x38
;;   38:	 b802000000           	mov	eax, 2
;;   3d:	 e90a000000           	jmp	0x4c
;;   42:	 e800000000           	call	0x47
;;   47:	 b803000000           	mov	eax, 3
;;   4c:	 4883c410             	add	rsp, 0x10
;;   50:	 5d                   	pop	rbp
;;   51:	 c3                   	ret	
