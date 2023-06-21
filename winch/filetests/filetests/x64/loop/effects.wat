;;! target = "x86_64"
(module
  (func $fx (export "effects") (result i32)
    (local i32)
    (block
      (loop
        (local.set 0 (i32.const 1))
        (local.set 0 (i32.mul (local.get 0) (i32.const 3)))
        (local.set 0 (i32.sub (local.get 0) (i32.const 5)))
        (local.set 0 (i32.mul (local.get 0) (i32.const 7)))
        (br 1)
        (local.set 0 (i32.mul (local.get 0) (i32.const 100)))
      )
    )
    (i32.eq (local.get 0) (i32.const -14))
  )
)
;;    0:	 55                   	push	rbp
;;    1:	 4889e5               	mov	rbp, rsp
;;    4:	 4883ec10             	sub	rsp, 0x10
;;    8:	 48c744240800000000   	
;; 				mov	qword ptr [rsp + 8], 0
;;   11:	 4c89742404           	mov	qword ptr [rsp + 4], r14
;;   16:	 b801000000           	mov	eax, 1
;;   1b:	 8944240c             	mov	dword ptr [rsp + 0xc], eax
;;   1f:	 8b44240c             	mov	eax, dword ptr [rsp + 0xc]
;;   23:	 6bc003               	imul	eax, eax, 3
;;   26:	 8944240c             	mov	dword ptr [rsp + 0xc], eax
;;   2a:	 8b44240c             	mov	eax, dword ptr [rsp + 0xc]
;;   2e:	 83e805               	sub	eax, 5
;;   31:	 8944240c             	mov	dword ptr [rsp + 0xc], eax
;;   35:	 8b44240c             	mov	eax, dword ptr [rsp + 0xc]
;;   39:	 6bc007               	imul	eax, eax, 7
;;   3c:	 8944240c             	mov	dword ptr [rsp + 0xc], eax
;;   40:	 8b44240c             	mov	eax, dword ptr [rsp + 0xc]
;;   44:	 83f8f2               	cmp	eax, -0xe
;;   47:	 b800000000           	mov	eax, 0
;;   4c:	 400f94c0             	sete	al
;;   50:	 4883c410             	add	rsp, 0x10
;;   54:	 5d                   	pop	rbp
;;   55:	 c3                   	ret	
