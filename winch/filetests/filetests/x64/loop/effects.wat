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
;;   11:	 4c893424             	mov	qword ptr [rsp], r14
;;   15:	 b801000000           	mov	eax, 1
;;   1a:	 8944240c             	mov	dword ptr [rsp + 0xc], eax
;;   1e:	 8b44240c             	mov	eax, dword ptr [rsp + 0xc]
;;   22:	 6bc003               	imul	eax, eax, 3
;;   25:	 8944240c             	mov	dword ptr [rsp + 0xc], eax
;;   29:	 8b44240c             	mov	eax, dword ptr [rsp + 0xc]
;;   2d:	 83e805               	sub	eax, 5
;;   30:	 8944240c             	mov	dword ptr [rsp + 0xc], eax
;;   34:	 8b44240c             	mov	eax, dword ptr [rsp + 0xc]
;;   38:	 6bc007               	imul	eax, eax, 7
;;   3b:	 8944240c             	mov	dword ptr [rsp + 0xc], eax
;;   3f:	 8b44240c             	mov	eax, dword ptr [rsp + 0xc]
;;   43:	 83f8f2               	cmp	eax, -0xe
;;   46:	 b800000000           	mov	eax, 0
;;   4b:	 400f94c0             	sete	al
;;   4f:	 4883c410             	add	rsp, 0x10
;;   53:	 5d                   	pop	rbp
;;   54:	 c3                   	ret	
