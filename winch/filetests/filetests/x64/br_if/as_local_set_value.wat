;;! target = "x86_64"
(module
  (func (export "as-local-set-value") (param i32) (result i32)
    (local i32)
    (block (result i32)
      (local.set 0 (br_if 0 (i32.const 17) (local.get 0)))
      (i32.const -1)
    )
  )
)
;;    0:	 55                   	push	rbp
;;    1:	 4889e5               	mov	rbp, rsp
;;    4:	 4883ec10             	sub	rsp, 0x10
;;    8:	 897c240c             	mov	dword ptr [rsp + 0xc], edi
;;    c:	 4c893424             	mov	qword ptr [rsp], r14
;;   10:	 8b4c240c             	mov	ecx, dword ptr [rsp + 0xc]
;;   14:	 b811000000           	mov	eax, 0x11
;;   19:	 85c9                 	test	ecx, ecx
;;   1b:	 0f8509000000         	jne	0x2a
;;   21:	 8944240c             	mov	dword ptr [rsp + 0xc], eax
;;   25:	 b8ffffffff           	mov	eax, 0xffffffff
;;   2a:	 4883c410             	add	rsp, 0x10
;;   2e:	 5d                   	pop	rbp
;;   2f:	 c3                   	ret	
