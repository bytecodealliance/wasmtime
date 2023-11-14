;;! target = "x86_64"
(module
  (func (export "break-inner") (result i32)
    (local i32)
    (local.set 0 (i32.const 0))
    (local.set 0 (i32.add (local.get 0) (block (result i32) (loop (result i32) (block (result i32) (br 2 (i32.const 0x1)))))))
    (local.set 0 (i32.add (local.get 0) (block (result i32) (loop (result i32) (loop (result i32) (br 2 (i32.const 0x2)))))))
    (local.set 0 (i32.add (local.get 0) (block (result i32) (loop (result i32) (block (result i32) (loop (result i32) (br 1 (i32.const 0x4))))))))
    (local.set 0 (i32.add (local.get 0) (block (result i32) (loop (result i32) (i32.ctz (br 1 (i32.const 0x8)))))))
    (local.set 0 (i32.add (local.get 0) (block (result i32) (loop (result i32) (i32.ctz (loop (result i32) (br 2 (i32.const 0x10))))))))
    (local.get 0)
  )
)
;;    0:	 55                   	push	rbp
;;    1:	 4889e5               	mov	rbp, rsp
;;    4:	 4883ec10             	sub	rsp, 0x10
;;    8:	 48c744240800000000   	
;; 				mov	qword ptr [rsp + 8], 0
;;   11:	 4c893424             	mov	qword ptr [rsp], r14
;;   15:	 b800000000           	mov	eax, 0
;;   1a:	 8944240c             	mov	dword ptr [rsp + 0xc], eax
;;   1e:	 448b5c240c           	mov	r11d, dword ptr [rsp + 0xc]
;;   23:	 4883ec04             	sub	rsp, 4
;;   27:	 44891c24             	mov	dword ptr [rsp], r11d
;;   2b:	 b801000000           	mov	eax, 1
;;   30:	 8b0c24               	mov	ecx, dword ptr [rsp]
;;   33:	 4883c404             	add	rsp, 4
;;   37:	 01c1                 	add	ecx, eax
;;   39:	 894c240c             	mov	dword ptr [rsp + 0xc], ecx
;;   3d:	 448b5c240c           	mov	r11d, dword ptr [rsp + 0xc]
;;   42:	 4883ec04             	sub	rsp, 4
;;   46:	 44891c24             	mov	dword ptr [rsp], r11d
;;   4a:	 b802000000           	mov	eax, 2
;;   4f:	 8b0c24               	mov	ecx, dword ptr [rsp]
;;   52:	 4883c404             	add	rsp, 4
;;   56:	 01c1                 	add	ecx, eax
;;   58:	 894c240c             	mov	dword ptr [rsp + 0xc], ecx
;;   5c:	 448b5c240c           	mov	r11d, dword ptr [rsp + 0xc]
;;   61:	 4883ec04             	sub	rsp, 4
;;   65:	 44891c24             	mov	dword ptr [rsp], r11d
;;   69:	 b804000000           	mov	eax, 4
;;   6e:	 8b0c24               	mov	ecx, dword ptr [rsp]
;;   71:	 4883c404             	add	rsp, 4
;;   75:	 01c1                 	add	ecx, eax
;;   77:	 894c240c             	mov	dword ptr [rsp + 0xc], ecx
;;   7b:	 448b5c240c           	mov	r11d, dword ptr [rsp + 0xc]
;;   80:	 4883ec04             	sub	rsp, 4
;;   84:	 44891c24             	mov	dword ptr [rsp], r11d
;;   88:	 b808000000           	mov	eax, 8
;;   8d:	 8b0c24               	mov	ecx, dword ptr [rsp]
;;   90:	 4883c404             	add	rsp, 4
;;   94:	 01c1                 	add	ecx, eax
;;   96:	 894c240c             	mov	dword ptr [rsp + 0xc], ecx
;;   9a:	 448b5c240c           	mov	r11d, dword ptr [rsp + 0xc]
;;   9f:	 4883ec04             	sub	rsp, 4
;;   a3:	 44891c24             	mov	dword ptr [rsp], r11d
;;   a7:	 b810000000           	mov	eax, 0x10
;;   ac:	 8b0c24               	mov	ecx, dword ptr [rsp]
;;   af:	 4883c404             	add	rsp, 4
;;   b3:	 01c1                 	add	ecx, eax
;;   b5:	 894c240c             	mov	dword ptr [rsp + 0xc], ecx
;;   b9:	 8b44240c             	mov	eax, dword ptr [rsp + 0xc]
;;   bd:	 4883c410             	add	rsp, 0x10
;;   c1:	 5d                   	pop	rbp
;;   c2:	 c3                   	ret	
