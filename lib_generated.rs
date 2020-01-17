#![feature(prelude_import)]
#[prelude_import]
use std::prelude::v1::*;
#[macro_use]
extern crate std;
pub type Filesize = u64;
pub type Timestamp = u64;
#[repr(u32)]
pub enum Clockid {
    Realtime,
    Monotonic,
    ProcessCputimeId,
    ThreadCputimeId,
}
#[automatically_derived]
#[allow(unused_qualifications)]
impl ::core::marker::Copy for Clockid {}
#[automatically_derived]
#[allow(unused_qualifications)]
impl ::core::clone::Clone for Clockid {
    #[inline]
    fn clone(&self) -> Clockid {
        {
            *self
        }
    }
}
#[automatically_derived]
#[allow(unused_qualifications)]
impl ::core::fmt::Debug for Clockid {
    fn fmt(&self, f: &mut ::core::fmt::Formatter) -> ::core::fmt::Result {
        match (&*self,) {
            (&Clockid::Realtime,) => {
                let mut debug_trait_builder = f.debug_tuple("Realtime");
                debug_trait_builder.finish()
            }
            (&Clockid::Monotonic,) => {
                let mut debug_trait_builder = f.debug_tuple("Monotonic");
                debug_trait_builder.finish()
            }
            (&Clockid::ProcessCputimeId,) => {
                let mut debug_trait_builder = f.debug_tuple("ProcessCputimeId");
                debug_trait_builder.finish()
            }
            (&Clockid::ThreadCputimeId,) => {
                let mut debug_trait_builder = f.debug_tuple("ThreadCputimeId");
                debug_trait_builder.finish()
            }
        }
    }
}
#[automatically_derived]
#[allow(unused_qualifications)]
impl ::core::hash::Hash for Clockid {
    fn hash<__H: ::core::hash::Hasher>(&self, state: &mut __H) -> () {
        match (&*self,) {
            _ => ::core::hash::Hash::hash(
                &unsafe { ::core::intrinsics::discriminant_value(self) },
                state,
            ),
        }
    }
}
impl ::core::marker::StructuralEq for Clockid {}
#[automatically_derived]
#[allow(unused_qualifications)]
impl ::core::cmp::Eq for Clockid {
    #[inline]
    #[doc(hidden)]
    fn assert_receiver_is_total_eq(&self) -> () {
        {}
    }
}
impl ::core::marker::StructuralPartialEq for Clockid {}
#[automatically_derived]
#[allow(unused_qualifications)]
impl ::core::cmp::PartialEq for Clockid {
    #[inline]
    fn eq(&self, other: &Clockid) -> bool {
        {
            let __self_vi = unsafe { ::core::intrinsics::discriminant_value(&*self) } as u32;
            let __arg_1_vi = unsafe { ::core::intrinsics::discriminant_value(&*other) } as u32;
            if true && __self_vi == __arg_1_vi {
                match (&*self, &*other) {
                    _ => true,
                }
            } else {
                false
            }
        }
    }
}
#[repr(u16)]
pub enum Errno {
    Esuccess,
    E2big,
    Eacces,
    Eaddrinuse,
    Eaddrnotavail,
    Eafnosupport,
    Eagain,
    Ealready,
    Ebadf,
    Ebadmsg,
    Ebusy,
    Ecanceled,
    Echild,
    Econnaborted,
    Econnrefused,
    Econnreset,
    Edeadlk,
    Edestaddrreq,
    Edom,
    Edquot,
    Eexist,
    Efault,
    Efbig,
    Ehostunreach,
    Eidrm,
    Eilseq,
    Einprogress,
    Eintr,
    Einval,
    Eio,
    Eisconn,
    Eisdir,
    Eloop,
    Emfile,
    Emlink,
    Emsgsize,
    Emultihop,
    Enametoolong,
    Enetdown,
    Enetreset,
    Enetunreach,
    Enfile,
    Enobufs,
    Enodev,
    Enoent,
    Enoexec,
    Enolck,
    Enolink,
    Enomem,
    Enomsg,
    Enoprotoopt,
    Enospc,
    Enosys,
    Enotconn,
    Enotdir,
    Enotempty,
    Enotrecoverable,
    Enotsock,
    Enotsup,
    Enotty,
    Enxio,
    Eoverflow,
    Eownerdead,
    Eperm,
    Epipe,
    Eproto,
    Eprotonosupport,
    Eprototype,
    Erange,
    Erofs,
    Espipe,
    Esrch,
    Estale,
    Etimedout,
    Etxtbsy,
    Exdev,
    Enotcapable,
}
#[automatically_derived]
#[allow(unused_qualifications)]
impl ::core::marker::Copy for Errno {}
#[automatically_derived]
#[allow(unused_qualifications)]
impl ::core::clone::Clone for Errno {
    #[inline]
    fn clone(&self) -> Errno {
        {
            *self
        }
    }
}
#[automatically_derived]
#[allow(unused_qualifications)]
impl ::core::fmt::Debug for Errno {
    fn fmt(&self, f: &mut ::core::fmt::Formatter) -> ::core::fmt::Result {
        match (&*self,) {
            (&Errno::Esuccess,) => {
                let mut debug_trait_builder = f.debug_tuple("Esuccess");
                debug_trait_builder.finish()
            }
            (&Errno::E2big,) => {
                let mut debug_trait_builder = f.debug_tuple("E2big");
                debug_trait_builder.finish()
            }
            (&Errno::Eacces,) => {
                let mut debug_trait_builder = f.debug_tuple("Eacces");
                debug_trait_builder.finish()
            }
            (&Errno::Eaddrinuse,) => {
                let mut debug_trait_builder = f.debug_tuple("Eaddrinuse");
                debug_trait_builder.finish()
            }
            (&Errno::Eaddrnotavail,) => {
                let mut debug_trait_builder = f.debug_tuple("Eaddrnotavail");
                debug_trait_builder.finish()
            }
            (&Errno::Eafnosupport,) => {
                let mut debug_trait_builder = f.debug_tuple("Eafnosupport");
                debug_trait_builder.finish()
            }
            (&Errno::Eagain,) => {
                let mut debug_trait_builder = f.debug_tuple("Eagain");
                debug_trait_builder.finish()
            }
            (&Errno::Ealready,) => {
                let mut debug_trait_builder = f.debug_tuple("Ealready");
                debug_trait_builder.finish()
            }
            (&Errno::Ebadf,) => {
                let mut debug_trait_builder = f.debug_tuple("Ebadf");
                debug_trait_builder.finish()
            }
            (&Errno::Ebadmsg,) => {
                let mut debug_trait_builder = f.debug_tuple("Ebadmsg");
                debug_trait_builder.finish()
            }
            (&Errno::Ebusy,) => {
                let mut debug_trait_builder = f.debug_tuple("Ebusy");
                debug_trait_builder.finish()
            }
            (&Errno::Ecanceled,) => {
                let mut debug_trait_builder = f.debug_tuple("Ecanceled");
                debug_trait_builder.finish()
            }
            (&Errno::Echild,) => {
                let mut debug_trait_builder = f.debug_tuple("Echild");
                debug_trait_builder.finish()
            }
            (&Errno::Econnaborted,) => {
                let mut debug_trait_builder = f.debug_tuple("Econnaborted");
                debug_trait_builder.finish()
            }
            (&Errno::Econnrefused,) => {
                let mut debug_trait_builder = f.debug_tuple("Econnrefused");
                debug_trait_builder.finish()
            }
            (&Errno::Econnreset,) => {
                let mut debug_trait_builder = f.debug_tuple("Econnreset");
                debug_trait_builder.finish()
            }
            (&Errno::Edeadlk,) => {
                let mut debug_trait_builder = f.debug_tuple("Edeadlk");
                debug_trait_builder.finish()
            }
            (&Errno::Edestaddrreq,) => {
                let mut debug_trait_builder = f.debug_tuple("Edestaddrreq");
                debug_trait_builder.finish()
            }
            (&Errno::Edom,) => {
                let mut debug_trait_builder = f.debug_tuple("Edom");
                debug_trait_builder.finish()
            }
            (&Errno::Edquot,) => {
                let mut debug_trait_builder = f.debug_tuple("Edquot");
                debug_trait_builder.finish()
            }
            (&Errno::Eexist,) => {
                let mut debug_trait_builder = f.debug_tuple("Eexist");
                debug_trait_builder.finish()
            }
            (&Errno::Efault,) => {
                let mut debug_trait_builder = f.debug_tuple("Efault");
                debug_trait_builder.finish()
            }
            (&Errno::Efbig,) => {
                let mut debug_trait_builder = f.debug_tuple("Efbig");
                debug_trait_builder.finish()
            }
            (&Errno::Ehostunreach,) => {
                let mut debug_trait_builder = f.debug_tuple("Ehostunreach");
                debug_trait_builder.finish()
            }
            (&Errno::Eidrm,) => {
                let mut debug_trait_builder = f.debug_tuple("Eidrm");
                debug_trait_builder.finish()
            }
            (&Errno::Eilseq,) => {
                let mut debug_trait_builder = f.debug_tuple("Eilseq");
                debug_trait_builder.finish()
            }
            (&Errno::Einprogress,) => {
                let mut debug_trait_builder = f.debug_tuple("Einprogress");
                debug_trait_builder.finish()
            }
            (&Errno::Eintr,) => {
                let mut debug_trait_builder = f.debug_tuple("Eintr");
                debug_trait_builder.finish()
            }
            (&Errno::Einval,) => {
                let mut debug_trait_builder = f.debug_tuple("Einval");
                debug_trait_builder.finish()
            }
            (&Errno::Eio,) => {
                let mut debug_trait_builder = f.debug_tuple("Eio");
                debug_trait_builder.finish()
            }
            (&Errno::Eisconn,) => {
                let mut debug_trait_builder = f.debug_tuple("Eisconn");
                debug_trait_builder.finish()
            }
            (&Errno::Eisdir,) => {
                let mut debug_trait_builder = f.debug_tuple("Eisdir");
                debug_trait_builder.finish()
            }
            (&Errno::Eloop,) => {
                let mut debug_trait_builder = f.debug_tuple("Eloop");
                debug_trait_builder.finish()
            }
            (&Errno::Emfile,) => {
                let mut debug_trait_builder = f.debug_tuple("Emfile");
                debug_trait_builder.finish()
            }
            (&Errno::Emlink,) => {
                let mut debug_trait_builder = f.debug_tuple("Emlink");
                debug_trait_builder.finish()
            }
            (&Errno::Emsgsize,) => {
                let mut debug_trait_builder = f.debug_tuple("Emsgsize");
                debug_trait_builder.finish()
            }
            (&Errno::Emultihop,) => {
                let mut debug_trait_builder = f.debug_tuple("Emultihop");
                debug_trait_builder.finish()
            }
            (&Errno::Enametoolong,) => {
                let mut debug_trait_builder = f.debug_tuple("Enametoolong");
                debug_trait_builder.finish()
            }
            (&Errno::Enetdown,) => {
                let mut debug_trait_builder = f.debug_tuple("Enetdown");
                debug_trait_builder.finish()
            }
            (&Errno::Enetreset,) => {
                let mut debug_trait_builder = f.debug_tuple("Enetreset");
                debug_trait_builder.finish()
            }
            (&Errno::Enetunreach,) => {
                let mut debug_trait_builder = f.debug_tuple("Enetunreach");
                debug_trait_builder.finish()
            }
            (&Errno::Enfile,) => {
                let mut debug_trait_builder = f.debug_tuple("Enfile");
                debug_trait_builder.finish()
            }
            (&Errno::Enobufs,) => {
                let mut debug_trait_builder = f.debug_tuple("Enobufs");
                debug_trait_builder.finish()
            }
            (&Errno::Enodev,) => {
                let mut debug_trait_builder = f.debug_tuple("Enodev");
                debug_trait_builder.finish()
            }
            (&Errno::Enoent,) => {
                let mut debug_trait_builder = f.debug_tuple("Enoent");
                debug_trait_builder.finish()
            }
            (&Errno::Enoexec,) => {
                let mut debug_trait_builder = f.debug_tuple("Enoexec");
                debug_trait_builder.finish()
            }
            (&Errno::Enolck,) => {
                let mut debug_trait_builder = f.debug_tuple("Enolck");
                debug_trait_builder.finish()
            }
            (&Errno::Enolink,) => {
                let mut debug_trait_builder = f.debug_tuple("Enolink");
                debug_trait_builder.finish()
            }
            (&Errno::Enomem,) => {
                let mut debug_trait_builder = f.debug_tuple("Enomem");
                debug_trait_builder.finish()
            }
            (&Errno::Enomsg,) => {
                let mut debug_trait_builder = f.debug_tuple("Enomsg");
                debug_trait_builder.finish()
            }
            (&Errno::Enoprotoopt,) => {
                let mut debug_trait_builder = f.debug_tuple("Enoprotoopt");
                debug_trait_builder.finish()
            }
            (&Errno::Enospc,) => {
                let mut debug_trait_builder = f.debug_tuple("Enospc");
                debug_trait_builder.finish()
            }
            (&Errno::Enosys,) => {
                let mut debug_trait_builder = f.debug_tuple("Enosys");
                debug_trait_builder.finish()
            }
            (&Errno::Enotconn,) => {
                let mut debug_trait_builder = f.debug_tuple("Enotconn");
                debug_trait_builder.finish()
            }
            (&Errno::Enotdir,) => {
                let mut debug_trait_builder = f.debug_tuple("Enotdir");
                debug_trait_builder.finish()
            }
            (&Errno::Enotempty,) => {
                let mut debug_trait_builder = f.debug_tuple("Enotempty");
                debug_trait_builder.finish()
            }
            (&Errno::Enotrecoverable,) => {
                let mut debug_trait_builder = f.debug_tuple("Enotrecoverable");
                debug_trait_builder.finish()
            }
            (&Errno::Enotsock,) => {
                let mut debug_trait_builder = f.debug_tuple("Enotsock");
                debug_trait_builder.finish()
            }
            (&Errno::Enotsup,) => {
                let mut debug_trait_builder = f.debug_tuple("Enotsup");
                debug_trait_builder.finish()
            }
            (&Errno::Enotty,) => {
                let mut debug_trait_builder = f.debug_tuple("Enotty");
                debug_trait_builder.finish()
            }
            (&Errno::Enxio,) => {
                let mut debug_trait_builder = f.debug_tuple("Enxio");
                debug_trait_builder.finish()
            }
            (&Errno::Eoverflow,) => {
                let mut debug_trait_builder = f.debug_tuple("Eoverflow");
                debug_trait_builder.finish()
            }
            (&Errno::Eownerdead,) => {
                let mut debug_trait_builder = f.debug_tuple("Eownerdead");
                debug_trait_builder.finish()
            }
            (&Errno::Eperm,) => {
                let mut debug_trait_builder = f.debug_tuple("Eperm");
                debug_trait_builder.finish()
            }
            (&Errno::Epipe,) => {
                let mut debug_trait_builder = f.debug_tuple("Epipe");
                debug_trait_builder.finish()
            }
            (&Errno::Eproto,) => {
                let mut debug_trait_builder = f.debug_tuple("Eproto");
                debug_trait_builder.finish()
            }
            (&Errno::Eprotonosupport,) => {
                let mut debug_trait_builder = f.debug_tuple("Eprotonosupport");
                debug_trait_builder.finish()
            }
            (&Errno::Eprototype,) => {
                let mut debug_trait_builder = f.debug_tuple("Eprototype");
                debug_trait_builder.finish()
            }
            (&Errno::Erange,) => {
                let mut debug_trait_builder = f.debug_tuple("Erange");
                debug_trait_builder.finish()
            }
            (&Errno::Erofs,) => {
                let mut debug_trait_builder = f.debug_tuple("Erofs");
                debug_trait_builder.finish()
            }
            (&Errno::Espipe,) => {
                let mut debug_trait_builder = f.debug_tuple("Espipe");
                debug_trait_builder.finish()
            }
            (&Errno::Esrch,) => {
                let mut debug_trait_builder = f.debug_tuple("Esrch");
                debug_trait_builder.finish()
            }
            (&Errno::Estale,) => {
                let mut debug_trait_builder = f.debug_tuple("Estale");
                debug_trait_builder.finish()
            }
            (&Errno::Etimedout,) => {
                let mut debug_trait_builder = f.debug_tuple("Etimedout");
                debug_trait_builder.finish()
            }
            (&Errno::Etxtbsy,) => {
                let mut debug_trait_builder = f.debug_tuple("Etxtbsy");
                debug_trait_builder.finish()
            }
            (&Errno::Exdev,) => {
                let mut debug_trait_builder = f.debug_tuple("Exdev");
                debug_trait_builder.finish()
            }
            (&Errno::Enotcapable,) => {
                let mut debug_trait_builder = f.debug_tuple("Enotcapable");
                debug_trait_builder.finish()
            }
        }
    }
}
#[automatically_derived]
#[allow(unused_qualifications)]
impl ::core::hash::Hash for Errno {
    fn hash<__H: ::core::hash::Hasher>(&self, state: &mut __H) -> () {
        match (&*self,) {
            _ => ::core::hash::Hash::hash(
                &unsafe { ::core::intrinsics::discriminant_value(self) },
                state,
            ),
        }
    }
}
impl ::core::marker::StructuralEq for Errno {}
#[automatically_derived]
#[allow(unused_qualifications)]
impl ::core::cmp::Eq for Errno {
    #[inline]
    #[doc(hidden)]
    fn assert_receiver_is_total_eq(&self) -> () {
        {}
    }
}
impl ::core::marker::StructuralPartialEq for Errno {}
#[automatically_derived]
#[allow(unused_qualifications)]
impl ::core::cmp::PartialEq for Errno {
    #[inline]
    fn eq(&self, other: &Errno) -> bool {
        {
            let __self_vi = unsafe { ::core::intrinsics::discriminant_value(&*self) } as u16;
            let __arg_1_vi = unsafe { ::core::intrinsics::discriminant_value(&*other) } as u16;
            if true && __self_vi == __arg_1_vi {
                match (&*self, &*other) {
                    _ => true,
                }
            } else {
                false
            }
        }
    }
}
#[repr(transparent)]
pub struct Rights(u64);
#[automatically_derived]
#[allow(unused_qualifications)]
impl ::core::marker::Copy for Rights {}
#[automatically_derived]
#[allow(unused_qualifications)]
impl ::core::clone::Clone for Rights {
    #[inline]
    fn clone(&self) -> Rights {
        {
            let _: ::core::clone::AssertParamIsClone<u64>;
            *self
        }
    }
}
#[automatically_derived]
#[allow(unused_qualifications)]
impl ::core::fmt::Debug for Rights {
    fn fmt(&self, f: &mut ::core::fmt::Formatter) -> ::core::fmt::Result {
        match *self {
            Rights(ref __self_0_0) => {
                let mut debug_trait_builder = f.debug_tuple("Rights");
                let _ = debug_trait_builder.field(&&(*__self_0_0));
                debug_trait_builder.finish()
            }
        }
    }
}
#[automatically_derived]
#[allow(unused_qualifications)]
impl ::core::hash::Hash for Rights {
    fn hash<__H: ::core::hash::Hasher>(&self, state: &mut __H) -> () {
        match *self {
            Rights(ref __self_0_0) => ::core::hash::Hash::hash(&(*__self_0_0), state),
        }
    }
}
impl ::core::marker::StructuralEq for Rights {}
#[automatically_derived]
#[allow(unused_qualifications)]
impl ::core::cmp::Eq for Rights {
    #[inline]
    #[doc(hidden)]
    fn assert_receiver_is_total_eq(&self) -> () {
        {
            let _: ::core::cmp::AssertParamIsEq<u64>;
        }
    }
}
impl ::core::marker::StructuralPartialEq for Rights {}
#[automatically_derived]
#[allow(unused_qualifications)]
impl ::core::cmp::PartialEq for Rights {
    #[inline]
    fn eq(&self, other: &Rights) -> bool {
        match *other {
            Rights(ref __self_1_0) => match *self {
                Rights(ref __self_0_0) => (*__self_0_0) == (*__self_1_0),
            },
        }
    }
    #[inline]
    fn ne(&self, other: &Rights) -> bool {
        match *other {
            Rights(ref __self_1_0) => match *self {
                Rights(ref __self_0_0) => (*__self_0_0) != (*__self_1_0),
            },
        }
    }
}
impl Rights {
    pub const FD_DATASYNC: Rights = Rights(1);
    pub const FD_READ: Rights = Rights(2);
    pub const FD_SEEK: Rights = Rights(4);
    pub const FD_FDSTAT_SET_FLAGS: Rights = Rights(8);
    pub const FD_SYNC: Rights = Rights(16);
    pub const FD_TELL: Rights = Rights(32);
    pub const FD_WRITE: Rights = Rights(64);
    pub const FD_ADVISE: Rights = Rights(128);
    pub const FD_ALLOCATE: Rights = Rights(256);
    pub const PATH_CREATE_DIRECTORY: Rights = Rights(512);
    pub const PATH_CREATE_FILE: Rights = Rights(1024);
    pub const PATH_LINK_SOURCE: Rights = Rights(2048);
    pub const PATH_LINK_TARGET: Rights = Rights(4096);
    pub const PATH_OPEN: Rights = Rights(8192);
    pub const FD_READDIR: Rights = Rights(16384);
    pub const PATH_READLINK: Rights = Rights(32768);
    pub const PATH_RENAME_SOURCE: Rights = Rights(65536);
    pub const PATH_RENAME_TARGET: Rights = Rights(131072);
    pub const PATH_FILESTAT_GET: Rights = Rights(262144);
    pub const PATH_FILESTAT_SET_SIZE: Rights = Rights(524288);
    pub const PATH_FILESTAT_SET_TIMES: Rights = Rights(1048576);
    pub const FD_FILESTAT_GET: Rights = Rights(2097152);
    pub const FD_FILESTAT_SET_SIZE: Rights = Rights(4194304);
    pub const FD_FILESTAT_SET_TIMES: Rights = Rights(8388608);
    pub const PATH_SYMLINK: Rights = Rights(16777216);
    pub const PATH_REMOVE_DIRECTORY: Rights = Rights(33554432);
    pub const PATH_UNLINK_FILE: Rights = Rights(67108864);
    pub const POLL_FD_READWRITE: Rights = Rights(134217728);
    pub const SOCK_SHUTDOWN: Rights = Rights(268435456);
}
pub type Fd = u32;
pub type Filedelta = i64;
#[repr(u8)]
pub enum Whence {
    Set,
    Cur,
    End,
}
#[automatically_derived]
#[allow(unused_qualifications)]
impl ::core::marker::Copy for Whence {}
#[automatically_derived]
#[allow(unused_qualifications)]
impl ::core::clone::Clone for Whence {
    #[inline]
    fn clone(&self) -> Whence {
        {
            *self
        }
    }
}
#[automatically_derived]
#[allow(unused_qualifications)]
impl ::core::fmt::Debug for Whence {
    fn fmt(&self, f: &mut ::core::fmt::Formatter) -> ::core::fmt::Result {
        match (&*self,) {
            (&Whence::Set,) => {
                let mut debug_trait_builder = f.debug_tuple("Set");
                debug_trait_builder.finish()
            }
            (&Whence::Cur,) => {
                let mut debug_trait_builder = f.debug_tuple("Cur");
                debug_trait_builder.finish()
            }
            (&Whence::End,) => {
                let mut debug_trait_builder = f.debug_tuple("End");
                debug_trait_builder.finish()
            }
        }
    }
}
#[automatically_derived]
#[allow(unused_qualifications)]
impl ::core::hash::Hash for Whence {
    fn hash<__H: ::core::hash::Hasher>(&self, state: &mut __H) -> () {
        match (&*self,) {
            _ => ::core::hash::Hash::hash(
                &unsafe { ::core::intrinsics::discriminant_value(self) },
                state,
            ),
        }
    }
}
impl ::core::marker::StructuralEq for Whence {}
#[automatically_derived]
#[allow(unused_qualifications)]
impl ::core::cmp::Eq for Whence {
    #[inline]
    #[doc(hidden)]
    fn assert_receiver_is_total_eq(&self) -> () {
        {}
    }
}
impl ::core::marker::StructuralPartialEq for Whence {}
#[automatically_derived]
#[allow(unused_qualifications)]
impl ::core::cmp::PartialEq for Whence {
    #[inline]
    fn eq(&self, other: &Whence) -> bool {
        {
            let __self_vi = unsafe { ::core::intrinsics::discriminant_value(&*self) } as u8;
            let __arg_1_vi = unsafe { ::core::intrinsics::discriminant_value(&*other) } as u8;
            if true && __self_vi == __arg_1_vi {
                match (&*self, &*other) {
                    _ => true,
                }
            } else {
                false
            }
        }
    }
}
pub type Dircookie = u64;
pub type Dirnamlen = u32;
pub type Inode = u64;
#[repr(u8)]
pub enum Filetype {
    Unknown,
    BlockDevice,
    CharacterDevice,
    Directory,
    RegularFile,
    SocketDgram,
    SocketStream,
    SymbolicLink,
}
#[automatically_derived]
#[allow(unused_qualifications)]
impl ::core::marker::Copy for Filetype {}
#[automatically_derived]
#[allow(unused_qualifications)]
impl ::core::clone::Clone for Filetype {
    #[inline]
    fn clone(&self) -> Filetype {
        {
            *self
        }
    }
}
#[automatically_derived]
#[allow(unused_qualifications)]
impl ::core::fmt::Debug for Filetype {
    fn fmt(&self, f: &mut ::core::fmt::Formatter) -> ::core::fmt::Result {
        match (&*self,) {
            (&Filetype::Unknown,) => {
                let mut debug_trait_builder = f.debug_tuple("Unknown");
                debug_trait_builder.finish()
            }
            (&Filetype::BlockDevice,) => {
                let mut debug_trait_builder = f.debug_tuple("BlockDevice");
                debug_trait_builder.finish()
            }
            (&Filetype::CharacterDevice,) => {
                let mut debug_trait_builder = f.debug_tuple("CharacterDevice");
                debug_trait_builder.finish()
            }
            (&Filetype::Directory,) => {
                let mut debug_trait_builder = f.debug_tuple("Directory");
                debug_trait_builder.finish()
            }
            (&Filetype::RegularFile,) => {
                let mut debug_trait_builder = f.debug_tuple("RegularFile");
                debug_trait_builder.finish()
            }
            (&Filetype::SocketDgram,) => {
                let mut debug_trait_builder = f.debug_tuple("SocketDgram");
                debug_trait_builder.finish()
            }
            (&Filetype::SocketStream,) => {
                let mut debug_trait_builder = f.debug_tuple("SocketStream");
                debug_trait_builder.finish()
            }
            (&Filetype::SymbolicLink,) => {
                let mut debug_trait_builder = f.debug_tuple("SymbolicLink");
                debug_trait_builder.finish()
            }
        }
    }
}
#[automatically_derived]
#[allow(unused_qualifications)]
impl ::core::hash::Hash for Filetype {
    fn hash<__H: ::core::hash::Hasher>(&self, state: &mut __H) -> () {
        match (&*self,) {
            _ => ::core::hash::Hash::hash(
                &unsafe { ::core::intrinsics::discriminant_value(self) },
                state,
            ),
        }
    }
}
impl ::core::marker::StructuralEq for Filetype {}
#[automatically_derived]
#[allow(unused_qualifications)]
impl ::core::cmp::Eq for Filetype {
    #[inline]
    #[doc(hidden)]
    fn assert_receiver_is_total_eq(&self) -> () {
        {}
    }
}
impl ::core::marker::StructuralPartialEq for Filetype {}
#[automatically_derived]
#[allow(unused_qualifications)]
impl ::core::cmp::PartialEq for Filetype {
    #[inline]
    fn eq(&self, other: &Filetype) -> bool {
        {
            let __self_vi = unsafe { ::core::intrinsics::discriminant_value(&*self) } as u8;
            let __arg_1_vi = unsafe { ::core::intrinsics::discriminant_value(&*other) } as u8;
            if true && __self_vi == __arg_1_vi {
                match (&*self, &*other) {
                    _ => true,
                }
            } else {
                false
            }
        }
    }
}
#[repr(C)]
pub struct Dirent {
    pub d_next: Dircookie,
    pub d_ino: Inode,
    pub d_namlen: Dirnamlen,
    pub d_type: Filetype,
}
#[automatically_derived]
#[allow(unused_qualifications)]
impl ::core::marker::Copy for Dirent {}
#[automatically_derived]
#[allow(unused_qualifications)]
impl ::core::clone::Clone for Dirent {
    #[inline]
    fn clone(&self) -> Dirent {
        {
            let _: ::core::clone::AssertParamIsClone<Dircookie>;
            let _: ::core::clone::AssertParamIsClone<Inode>;
            let _: ::core::clone::AssertParamIsClone<Dirnamlen>;
            let _: ::core::clone::AssertParamIsClone<Filetype>;
            *self
        }
    }
}
#[automatically_derived]
#[allow(unused_qualifications)]
impl ::core::fmt::Debug for Dirent {
    fn fmt(&self, f: &mut ::core::fmt::Formatter) -> ::core::fmt::Result {
        match *self {
            Dirent {
                d_next: ref __self_0_0,
                d_ino: ref __self_0_1,
                d_namlen: ref __self_0_2,
                d_type: ref __self_0_3,
            } => {
                let mut debug_trait_builder = f.debug_struct("Dirent");
                let _ = debug_trait_builder.field("d_next", &&(*__self_0_0));
                let _ = debug_trait_builder.field("d_ino", &&(*__self_0_1));
                let _ = debug_trait_builder.field("d_namlen", &&(*__self_0_2));
                let _ = debug_trait_builder.field("d_type", &&(*__self_0_3));
                debug_trait_builder.finish()
            }
        }
    }
}
#[automatically_derived]
#[allow(unused_qualifications)]
impl ::core::hash::Hash for Dirent {
    fn hash<__H: ::core::hash::Hasher>(&self, state: &mut __H) -> () {
        match *self {
            Dirent {
                d_next: ref __self_0_0,
                d_ino: ref __self_0_1,
                d_namlen: ref __self_0_2,
                d_type: ref __self_0_3,
            } => {
                ::core::hash::Hash::hash(&(*__self_0_0), state);
                ::core::hash::Hash::hash(&(*__self_0_1), state);
                ::core::hash::Hash::hash(&(*__self_0_2), state);
                ::core::hash::Hash::hash(&(*__self_0_3), state)
            }
        }
    }
}
impl ::core::marker::StructuralEq for Dirent {}
#[automatically_derived]
#[allow(unused_qualifications)]
impl ::core::cmp::Eq for Dirent {
    #[inline]
    #[doc(hidden)]
    fn assert_receiver_is_total_eq(&self) -> () {
        {
            let _: ::core::cmp::AssertParamIsEq<Dircookie>;
            let _: ::core::cmp::AssertParamIsEq<Inode>;
            let _: ::core::cmp::AssertParamIsEq<Dirnamlen>;
            let _: ::core::cmp::AssertParamIsEq<Filetype>;
        }
    }
}
impl ::core::marker::StructuralPartialEq for Dirent {}
#[automatically_derived]
#[allow(unused_qualifications)]
impl ::core::cmp::PartialEq for Dirent {
    #[inline]
    fn eq(&self, other: &Dirent) -> bool {
        match *other {
            Dirent {
                d_next: ref __self_1_0,
                d_ino: ref __self_1_1,
                d_namlen: ref __self_1_2,
                d_type: ref __self_1_3,
            } => match *self {
                Dirent {
                    d_next: ref __self_0_0,
                    d_ino: ref __self_0_1,
                    d_namlen: ref __self_0_2,
                    d_type: ref __self_0_3,
                } => {
                    (*__self_0_0) == (*__self_1_0)
                        && (*__self_0_1) == (*__self_1_1)
                        && (*__self_0_2) == (*__self_1_2)
                        && (*__self_0_3) == (*__self_1_3)
                }
            },
        }
    }
    #[inline]
    fn ne(&self, other: &Dirent) -> bool {
        match *other {
            Dirent {
                d_next: ref __self_1_0,
                d_ino: ref __self_1_1,
                d_namlen: ref __self_1_2,
                d_type: ref __self_1_3,
            } => match *self {
                Dirent {
                    d_next: ref __self_0_0,
                    d_ino: ref __self_0_1,
                    d_namlen: ref __self_0_2,
                    d_type: ref __self_0_3,
                } => {
                    (*__self_0_0) != (*__self_1_0)
                        || (*__self_0_1) != (*__self_1_1)
                        || (*__self_0_2) != (*__self_1_2)
                        || (*__self_0_3) != (*__self_1_3)
                }
            },
        }
    }
}
#[repr(u8)]
pub enum Advice {
    Normal,
    Sequential,
    Random,
    Willneed,
    Dontneed,
    Noreuse,
}
#[automatically_derived]
#[allow(unused_qualifications)]
impl ::core::marker::Copy for Advice {}
#[automatically_derived]
#[allow(unused_qualifications)]
impl ::core::clone::Clone for Advice {
    #[inline]
    fn clone(&self) -> Advice {
        {
            *self
        }
    }
}
#[automatically_derived]
#[allow(unused_qualifications)]
impl ::core::fmt::Debug for Advice {
    fn fmt(&self, f: &mut ::core::fmt::Formatter) -> ::core::fmt::Result {
        match (&*self,) {
            (&Advice::Normal,) => {
                let mut debug_trait_builder = f.debug_tuple("Normal");
                debug_trait_builder.finish()
            }
            (&Advice::Sequential,) => {
                let mut debug_trait_builder = f.debug_tuple("Sequential");
                debug_trait_builder.finish()
            }
            (&Advice::Random,) => {
                let mut debug_trait_builder = f.debug_tuple("Random");
                debug_trait_builder.finish()
            }
            (&Advice::Willneed,) => {
                let mut debug_trait_builder = f.debug_tuple("Willneed");
                debug_trait_builder.finish()
            }
            (&Advice::Dontneed,) => {
                let mut debug_trait_builder = f.debug_tuple("Dontneed");
                debug_trait_builder.finish()
            }
            (&Advice::Noreuse,) => {
                let mut debug_trait_builder = f.debug_tuple("Noreuse");
                debug_trait_builder.finish()
            }
        }
    }
}
#[automatically_derived]
#[allow(unused_qualifications)]
impl ::core::hash::Hash for Advice {
    fn hash<__H: ::core::hash::Hasher>(&self, state: &mut __H) -> () {
        match (&*self,) {
            _ => ::core::hash::Hash::hash(
                &unsafe { ::core::intrinsics::discriminant_value(self) },
                state,
            ),
        }
    }
}
impl ::core::marker::StructuralEq for Advice {}
#[automatically_derived]
#[allow(unused_qualifications)]
impl ::core::cmp::Eq for Advice {
    #[inline]
    #[doc(hidden)]
    fn assert_receiver_is_total_eq(&self) -> () {
        {}
    }
}
impl ::core::marker::StructuralPartialEq for Advice {}
#[automatically_derived]
#[allow(unused_qualifications)]
impl ::core::cmp::PartialEq for Advice {
    #[inline]
    fn eq(&self, other: &Advice) -> bool {
        {
            let __self_vi = unsafe { ::core::intrinsics::discriminant_value(&*self) } as u8;
            let __arg_1_vi = unsafe { ::core::intrinsics::discriminant_value(&*other) } as u8;
            if true && __self_vi == __arg_1_vi {
                match (&*self, &*other) {
                    _ => true,
                }
            } else {
                false
            }
        }
    }
}
#[repr(transparent)]
pub struct Fdflags(u16);
#[automatically_derived]
#[allow(unused_qualifications)]
impl ::core::marker::Copy for Fdflags {}
#[automatically_derived]
#[allow(unused_qualifications)]
impl ::core::clone::Clone for Fdflags {
    #[inline]
    fn clone(&self) -> Fdflags {
        {
            let _: ::core::clone::AssertParamIsClone<u16>;
            *self
        }
    }
}
#[automatically_derived]
#[allow(unused_qualifications)]
impl ::core::fmt::Debug for Fdflags {
    fn fmt(&self, f: &mut ::core::fmt::Formatter) -> ::core::fmt::Result {
        match *self {
            Fdflags(ref __self_0_0) => {
                let mut debug_trait_builder = f.debug_tuple("Fdflags");
                let _ = debug_trait_builder.field(&&(*__self_0_0));
                debug_trait_builder.finish()
            }
        }
    }
}
#[automatically_derived]
#[allow(unused_qualifications)]
impl ::core::hash::Hash for Fdflags {
    fn hash<__H: ::core::hash::Hasher>(&self, state: &mut __H) -> () {
        match *self {
            Fdflags(ref __self_0_0) => ::core::hash::Hash::hash(&(*__self_0_0), state),
        }
    }
}
impl ::core::marker::StructuralEq for Fdflags {}
#[automatically_derived]
#[allow(unused_qualifications)]
impl ::core::cmp::Eq for Fdflags {
    #[inline]
    #[doc(hidden)]
    fn assert_receiver_is_total_eq(&self) -> () {
        {
            let _: ::core::cmp::AssertParamIsEq<u16>;
        }
    }
}
impl ::core::marker::StructuralPartialEq for Fdflags {}
#[automatically_derived]
#[allow(unused_qualifications)]
impl ::core::cmp::PartialEq for Fdflags {
    #[inline]
    fn eq(&self, other: &Fdflags) -> bool {
        match *other {
            Fdflags(ref __self_1_0) => match *self {
                Fdflags(ref __self_0_0) => (*__self_0_0) == (*__self_1_0),
            },
        }
    }
    #[inline]
    fn ne(&self, other: &Fdflags) -> bool {
        match *other {
            Fdflags(ref __self_1_0) => match *self {
                Fdflags(ref __self_0_0) => (*__self_0_0) != (*__self_1_0),
            },
        }
    }
}
impl Fdflags {
    pub const APPEND: Fdflags = Fdflags(1);
    pub const DSYNC: Fdflags = Fdflags(2);
    pub const NONBLOCK: Fdflags = Fdflags(4);
    pub const RSYNC: Fdflags = Fdflags(8);
    pub const SYNC: Fdflags = Fdflags(16);
}
#[repr(C)]
pub struct Fdstat {
    pub fs_filetype: Filetype,
    pub fs_flags: Fdflags,
    pub fs_rights_base: Rights,
    pub fs_rights_inheriting: Rights,
}
#[automatically_derived]
#[allow(unused_qualifications)]
impl ::core::marker::Copy for Fdstat {}
#[automatically_derived]
#[allow(unused_qualifications)]
impl ::core::clone::Clone for Fdstat {
    #[inline]
    fn clone(&self) -> Fdstat {
        {
            let _: ::core::clone::AssertParamIsClone<Filetype>;
            let _: ::core::clone::AssertParamIsClone<Fdflags>;
            let _: ::core::clone::AssertParamIsClone<Rights>;
            let _: ::core::clone::AssertParamIsClone<Rights>;
            *self
        }
    }
}
#[automatically_derived]
#[allow(unused_qualifications)]
impl ::core::fmt::Debug for Fdstat {
    fn fmt(&self, f: &mut ::core::fmt::Formatter) -> ::core::fmt::Result {
        match *self {
            Fdstat {
                fs_filetype: ref __self_0_0,
                fs_flags: ref __self_0_1,
                fs_rights_base: ref __self_0_2,
                fs_rights_inheriting: ref __self_0_3,
            } => {
                let mut debug_trait_builder = f.debug_struct("Fdstat");
                let _ = debug_trait_builder.field("fs_filetype", &&(*__self_0_0));
                let _ = debug_trait_builder.field("fs_flags", &&(*__self_0_1));
                let _ = debug_trait_builder.field("fs_rights_base", &&(*__self_0_2));
                let _ = debug_trait_builder.field("fs_rights_inheriting", &&(*__self_0_3));
                debug_trait_builder.finish()
            }
        }
    }
}
#[automatically_derived]
#[allow(unused_qualifications)]
impl ::core::hash::Hash for Fdstat {
    fn hash<__H: ::core::hash::Hasher>(&self, state: &mut __H) -> () {
        match *self {
            Fdstat {
                fs_filetype: ref __self_0_0,
                fs_flags: ref __self_0_1,
                fs_rights_base: ref __self_0_2,
                fs_rights_inheriting: ref __self_0_3,
            } => {
                ::core::hash::Hash::hash(&(*__self_0_0), state);
                ::core::hash::Hash::hash(&(*__self_0_1), state);
                ::core::hash::Hash::hash(&(*__self_0_2), state);
                ::core::hash::Hash::hash(&(*__self_0_3), state)
            }
        }
    }
}
impl ::core::marker::StructuralEq for Fdstat {}
#[automatically_derived]
#[allow(unused_qualifications)]
impl ::core::cmp::Eq for Fdstat {
    #[inline]
    #[doc(hidden)]
    fn assert_receiver_is_total_eq(&self) -> () {
        {
            let _: ::core::cmp::AssertParamIsEq<Filetype>;
            let _: ::core::cmp::AssertParamIsEq<Fdflags>;
            let _: ::core::cmp::AssertParamIsEq<Rights>;
            let _: ::core::cmp::AssertParamIsEq<Rights>;
        }
    }
}
impl ::core::marker::StructuralPartialEq for Fdstat {}
#[automatically_derived]
#[allow(unused_qualifications)]
impl ::core::cmp::PartialEq for Fdstat {
    #[inline]
    fn eq(&self, other: &Fdstat) -> bool {
        match *other {
            Fdstat {
                fs_filetype: ref __self_1_0,
                fs_flags: ref __self_1_1,
                fs_rights_base: ref __self_1_2,
                fs_rights_inheriting: ref __self_1_3,
            } => match *self {
                Fdstat {
                    fs_filetype: ref __self_0_0,
                    fs_flags: ref __self_0_1,
                    fs_rights_base: ref __self_0_2,
                    fs_rights_inheriting: ref __self_0_3,
                } => {
                    (*__self_0_0) == (*__self_1_0)
                        && (*__self_0_1) == (*__self_1_1)
                        && (*__self_0_2) == (*__self_1_2)
                        && (*__self_0_3) == (*__self_1_3)
                }
            },
        }
    }
    #[inline]
    fn ne(&self, other: &Fdstat) -> bool {
        match *other {
            Fdstat {
                fs_filetype: ref __self_1_0,
                fs_flags: ref __self_1_1,
                fs_rights_base: ref __self_1_2,
                fs_rights_inheriting: ref __self_1_3,
            } => match *self {
                Fdstat {
                    fs_filetype: ref __self_0_0,
                    fs_flags: ref __self_0_1,
                    fs_rights_base: ref __self_0_2,
                    fs_rights_inheriting: ref __self_0_3,
                } => {
                    (*__self_0_0) != (*__self_1_0)
                        || (*__self_0_1) != (*__self_1_1)
                        || (*__self_0_2) != (*__self_1_2)
                        || (*__self_0_3) != (*__self_1_3)
                }
            },
        }
    }
}
pub type Device = u64;
#[repr(transparent)]
pub struct Fstflags(u16);
#[automatically_derived]
#[allow(unused_qualifications)]
impl ::core::marker::Copy for Fstflags {}
#[automatically_derived]
#[allow(unused_qualifications)]
impl ::core::clone::Clone for Fstflags {
    #[inline]
    fn clone(&self) -> Fstflags {
        {
            let _: ::core::clone::AssertParamIsClone<u16>;
            *self
        }
    }
}
#[automatically_derived]
#[allow(unused_qualifications)]
impl ::core::fmt::Debug for Fstflags {
    fn fmt(&self, f: &mut ::core::fmt::Formatter) -> ::core::fmt::Result {
        match *self {
            Fstflags(ref __self_0_0) => {
                let mut debug_trait_builder = f.debug_tuple("Fstflags");
                let _ = debug_trait_builder.field(&&(*__self_0_0));
                debug_trait_builder.finish()
            }
        }
    }
}
#[automatically_derived]
#[allow(unused_qualifications)]
impl ::core::hash::Hash for Fstflags {
    fn hash<__H: ::core::hash::Hasher>(&self, state: &mut __H) -> () {
        match *self {
            Fstflags(ref __self_0_0) => ::core::hash::Hash::hash(&(*__self_0_0), state),
        }
    }
}
impl ::core::marker::StructuralEq for Fstflags {}
#[automatically_derived]
#[allow(unused_qualifications)]
impl ::core::cmp::Eq for Fstflags {
    #[inline]
    #[doc(hidden)]
    fn assert_receiver_is_total_eq(&self) -> () {
        {
            let _: ::core::cmp::AssertParamIsEq<u16>;
        }
    }
}
impl ::core::marker::StructuralPartialEq for Fstflags {}
#[automatically_derived]
#[allow(unused_qualifications)]
impl ::core::cmp::PartialEq for Fstflags {
    #[inline]
    fn eq(&self, other: &Fstflags) -> bool {
        match *other {
            Fstflags(ref __self_1_0) => match *self {
                Fstflags(ref __self_0_0) => (*__self_0_0) == (*__self_1_0),
            },
        }
    }
    #[inline]
    fn ne(&self, other: &Fstflags) -> bool {
        match *other {
            Fstflags(ref __self_1_0) => match *self {
                Fstflags(ref __self_0_0) => (*__self_0_0) != (*__self_1_0),
            },
        }
    }
}
impl Fstflags {
    pub const ATIM: Fstflags = Fstflags(1);
    pub const ATIM_NOW: Fstflags = Fstflags(2);
    pub const MTIM: Fstflags = Fstflags(4);
    pub const MTIM_NOW: Fstflags = Fstflags(8);
}
#[repr(transparent)]
pub struct Lookupflags(u32);
#[automatically_derived]
#[allow(unused_qualifications)]
impl ::core::marker::Copy for Lookupflags {}
#[automatically_derived]
#[allow(unused_qualifications)]
impl ::core::clone::Clone for Lookupflags {
    #[inline]
    fn clone(&self) -> Lookupflags {
        {
            let _: ::core::clone::AssertParamIsClone<u32>;
            *self
        }
    }
}
#[automatically_derived]
#[allow(unused_qualifications)]
impl ::core::fmt::Debug for Lookupflags {
    fn fmt(&self, f: &mut ::core::fmt::Formatter) -> ::core::fmt::Result {
        match *self {
            Lookupflags(ref __self_0_0) => {
                let mut debug_trait_builder = f.debug_tuple("Lookupflags");
                let _ = debug_trait_builder.field(&&(*__self_0_0));
                debug_trait_builder.finish()
            }
        }
    }
}
#[automatically_derived]
#[allow(unused_qualifications)]
impl ::core::hash::Hash for Lookupflags {
    fn hash<__H: ::core::hash::Hasher>(&self, state: &mut __H) -> () {
        match *self {
            Lookupflags(ref __self_0_0) => ::core::hash::Hash::hash(&(*__self_0_0), state),
        }
    }
}
impl ::core::marker::StructuralEq for Lookupflags {}
#[automatically_derived]
#[allow(unused_qualifications)]
impl ::core::cmp::Eq for Lookupflags {
    #[inline]
    #[doc(hidden)]
    fn assert_receiver_is_total_eq(&self) -> () {
        {
            let _: ::core::cmp::AssertParamIsEq<u32>;
        }
    }
}
impl ::core::marker::StructuralPartialEq for Lookupflags {}
#[automatically_derived]
#[allow(unused_qualifications)]
impl ::core::cmp::PartialEq for Lookupflags {
    #[inline]
    fn eq(&self, other: &Lookupflags) -> bool {
        match *other {
            Lookupflags(ref __self_1_0) => match *self {
                Lookupflags(ref __self_0_0) => (*__self_0_0) == (*__self_1_0),
            },
        }
    }
    #[inline]
    fn ne(&self, other: &Lookupflags) -> bool {
        match *other {
            Lookupflags(ref __self_1_0) => match *self {
                Lookupflags(ref __self_0_0) => (*__self_0_0) != (*__self_1_0),
            },
        }
    }
}
impl Lookupflags {
    pub const SYMLINK_FOLLOW: Lookupflags = Lookupflags(1);
}
#[repr(transparent)]
pub struct Oflags(u16);
#[automatically_derived]
#[allow(unused_qualifications)]
impl ::core::marker::Copy for Oflags {}
#[automatically_derived]
#[allow(unused_qualifications)]
impl ::core::clone::Clone for Oflags {
    #[inline]
    fn clone(&self) -> Oflags {
        {
            let _: ::core::clone::AssertParamIsClone<u16>;
            *self
        }
    }
}
#[automatically_derived]
#[allow(unused_qualifications)]
impl ::core::fmt::Debug for Oflags {
    fn fmt(&self, f: &mut ::core::fmt::Formatter) -> ::core::fmt::Result {
        match *self {
            Oflags(ref __self_0_0) => {
                let mut debug_trait_builder = f.debug_tuple("Oflags");
                let _ = debug_trait_builder.field(&&(*__self_0_0));
                debug_trait_builder.finish()
            }
        }
    }
}
#[automatically_derived]
#[allow(unused_qualifications)]
impl ::core::hash::Hash for Oflags {
    fn hash<__H: ::core::hash::Hasher>(&self, state: &mut __H) -> () {
        match *self {
            Oflags(ref __self_0_0) => ::core::hash::Hash::hash(&(*__self_0_0), state),
        }
    }
}
impl ::core::marker::StructuralEq for Oflags {}
#[automatically_derived]
#[allow(unused_qualifications)]
impl ::core::cmp::Eq for Oflags {
    #[inline]
    #[doc(hidden)]
    fn assert_receiver_is_total_eq(&self) -> () {
        {
            let _: ::core::cmp::AssertParamIsEq<u16>;
        }
    }
}
impl ::core::marker::StructuralPartialEq for Oflags {}
#[automatically_derived]
#[allow(unused_qualifications)]
impl ::core::cmp::PartialEq for Oflags {
    #[inline]
    fn eq(&self, other: &Oflags) -> bool {
        match *other {
            Oflags(ref __self_1_0) => match *self {
                Oflags(ref __self_0_0) => (*__self_0_0) == (*__self_1_0),
            },
        }
    }
    #[inline]
    fn ne(&self, other: &Oflags) -> bool {
        match *other {
            Oflags(ref __self_1_0) => match *self {
                Oflags(ref __self_0_0) => (*__self_0_0) != (*__self_1_0),
            },
        }
    }
}
impl Oflags {
    pub const CREAT: Oflags = Oflags(1);
    pub const DIRECTORY: Oflags = Oflags(2);
    pub const EXCL: Oflags = Oflags(4);
    pub const TRUNC: Oflags = Oflags(8);
}
pub type Linkcount = u64;
#[repr(C)]
pub struct Filestat {
    pub dev: Device,
    pub ino: Inode,
    pub filetype: Filetype,
    pub nlink: Linkcount,
    pub size: Filesize,
    pub atim: Timestamp,
    pub mtim: Timestamp,
    pub ctim: Timestamp,
}
#[automatically_derived]
#[allow(unused_qualifications)]
impl ::core::marker::Copy for Filestat {}
#[automatically_derived]
#[allow(unused_qualifications)]
impl ::core::clone::Clone for Filestat {
    #[inline]
    fn clone(&self) -> Filestat {
        {
            let _: ::core::clone::AssertParamIsClone<Device>;
            let _: ::core::clone::AssertParamIsClone<Inode>;
            let _: ::core::clone::AssertParamIsClone<Filetype>;
            let _: ::core::clone::AssertParamIsClone<Linkcount>;
            let _: ::core::clone::AssertParamIsClone<Filesize>;
            let _: ::core::clone::AssertParamIsClone<Timestamp>;
            let _: ::core::clone::AssertParamIsClone<Timestamp>;
            let _: ::core::clone::AssertParamIsClone<Timestamp>;
            *self
        }
    }
}
#[automatically_derived]
#[allow(unused_qualifications)]
impl ::core::fmt::Debug for Filestat {
    fn fmt(&self, f: &mut ::core::fmt::Formatter) -> ::core::fmt::Result {
        match *self {
            Filestat {
                dev: ref __self_0_0,
                ino: ref __self_0_1,
                filetype: ref __self_0_2,
                nlink: ref __self_0_3,
                size: ref __self_0_4,
                atim: ref __self_0_5,
                mtim: ref __self_0_6,
                ctim: ref __self_0_7,
            } => {
                let mut debug_trait_builder = f.debug_struct("Filestat");
                let _ = debug_trait_builder.field("dev", &&(*__self_0_0));
                let _ = debug_trait_builder.field("ino", &&(*__self_0_1));
                let _ = debug_trait_builder.field("filetype", &&(*__self_0_2));
                let _ = debug_trait_builder.field("nlink", &&(*__self_0_3));
                let _ = debug_trait_builder.field("size", &&(*__self_0_4));
                let _ = debug_trait_builder.field("atim", &&(*__self_0_5));
                let _ = debug_trait_builder.field("mtim", &&(*__self_0_6));
                let _ = debug_trait_builder.field("ctim", &&(*__self_0_7));
                debug_trait_builder.finish()
            }
        }
    }
}
#[automatically_derived]
#[allow(unused_qualifications)]
impl ::core::hash::Hash for Filestat {
    fn hash<__H: ::core::hash::Hasher>(&self, state: &mut __H) -> () {
        match *self {
            Filestat {
                dev: ref __self_0_0,
                ino: ref __self_0_1,
                filetype: ref __self_0_2,
                nlink: ref __self_0_3,
                size: ref __self_0_4,
                atim: ref __self_0_5,
                mtim: ref __self_0_6,
                ctim: ref __self_0_7,
            } => {
                ::core::hash::Hash::hash(&(*__self_0_0), state);
                ::core::hash::Hash::hash(&(*__self_0_1), state);
                ::core::hash::Hash::hash(&(*__self_0_2), state);
                ::core::hash::Hash::hash(&(*__self_0_3), state);
                ::core::hash::Hash::hash(&(*__self_0_4), state);
                ::core::hash::Hash::hash(&(*__self_0_5), state);
                ::core::hash::Hash::hash(&(*__self_0_6), state);
                ::core::hash::Hash::hash(&(*__self_0_7), state)
            }
        }
    }
}
impl ::core::marker::StructuralEq for Filestat {}
#[automatically_derived]
#[allow(unused_qualifications)]
impl ::core::cmp::Eq for Filestat {
    #[inline]
    #[doc(hidden)]
    fn assert_receiver_is_total_eq(&self) -> () {
        {
            let _: ::core::cmp::AssertParamIsEq<Device>;
            let _: ::core::cmp::AssertParamIsEq<Inode>;
            let _: ::core::cmp::AssertParamIsEq<Filetype>;
            let _: ::core::cmp::AssertParamIsEq<Linkcount>;
            let _: ::core::cmp::AssertParamIsEq<Filesize>;
            let _: ::core::cmp::AssertParamIsEq<Timestamp>;
            let _: ::core::cmp::AssertParamIsEq<Timestamp>;
            let _: ::core::cmp::AssertParamIsEq<Timestamp>;
        }
    }
}
impl ::core::marker::StructuralPartialEq for Filestat {}
#[automatically_derived]
#[allow(unused_qualifications)]
impl ::core::cmp::PartialEq for Filestat {
    #[inline]
    fn eq(&self, other: &Filestat) -> bool {
        match *other {
            Filestat {
                dev: ref __self_1_0,
                ino: ref __self_1_1,
                filetype: ref __self_1_2,
                nlink: ref __self_1_3,
                size: ref __self_1_4,
                atim: ref __self_1_5,
                mtim: ref __self_1_6,
                ctim: ref __self_1_7,
            } => match *self {
                Filestat {
                    dev: ref __self_0_0,
                    ino: ref __self_0_1,
                    filetype: ref __self_0_2,
                    nlink: ref __self_0_3,
                    size: ref __self_0_4,
                    atim: ref __self_0_5,
                    mtim: ref __self_0_6,
                    ctim: ref __self_0_7,
                } => {
                    (*__self_0_0) == (*__self_1_0)
                        && (*__self_0_1) == (*__self_1_1)
                        && (*__self_0_2) == (*__self_1_2)
                        && (*__self_0_3) == (*__self_1_3)
                        && (*__self_0_4) == (*__self_1_4)
                        && (*__self_0_5) == (*__self_1_5)
                        && (*__self_0_6) == (*__self_1_6)
                        && (*__self_0_7) == (*__self_1_7)
                }
            },
        }
    }
    #[inline]
    fn ne(&self, other: &Filestat) -> bool {
        match *other {
            Filestat {
                dev: ref __self_1_0,
                ino: ref __self_1_1,
                filetype: ref __self_1_2,
                nlink: ref __self_1_3,
                size: ref __self_1_4,
                atim: ref __self_1_5,
                mtim: ref __self_1_6,
                ctim: ref __self_1_7,
            } => match *self {
                Filestat {
                    dev: ref __self_0_0,
                    ino: ref __self_0_1,
                    filetype: ref __self_0_2,
                    nlink: ref __self_0_3,
                    size: ref __self_0_4,
                    atim: ref __self_0_5,
                    mtim: ref __self_0_6,
                    ctim: ref __self_0_7,
                } => {
                    (*__self_0_0) != (*__self_1_0)
                        || (*__self_0_1) != (*__self_1_1)
                        || (*__self_0_2) != (*__self_1_2)
                        || (*__self_0_3) != (*__self_1_3)
                        || (*__self_0_4) != (*__self_1_4)
                        || (*__self_0_5) != (*__self_1_5)
                        || (*__self_0_6) != (*__self_1_6)
                        || (*__self_0_7) != (*__self_1_7)
                }
            },
        }
    }
}
pub type Userdata = u64;
#[repr(u8)]
pub enum Eventtype {
    Clock,
    FdRead,
    FdWrite,
}
#[automatically_derived]
#[allow(unused_qualifications)]
impl ::core::marker::Copy for Eventtype {}
#[automatically_derived]
#[allow(unused_qualifications)]
impl ::core::clone::Clone for Eventtype {
    #[inline]
    fn clone(&self) -> Eventtype {
        {
            *self
        }
    }
}
#[automatically_derived]
#[allow(unused_qualifications)]
impl ::core::fmt::Debug for Eventtype {
    fn fmt(&self, f: &mut ::core::fmt::Formatter) -> ::core::fmt::Result {
        match (&*self,) {
            (&Eventtype::Clock,) => {
                let mut debug_trait_builder = f.debug_tuple("Clock");
                debug_trait_builder.finish()
            }
            (&Eventtype::FdRead,) => {
                let mut debug_trait_builder = f.debug_tuple("FdRead");
                debug_trait_builder.finish()
            }
            (&Eventtype::FdWrite,) => {
                let mut debug_trait_builder = f.debug_tuple("FdWrite");
                debug_trait_builder.finish()
            }
        }
    }
}
#[automatically_derived]
#[allow(unused_qualifications)]
impl ::core::hash::Hash for Eventtype {
    fn hash<__H: ::core::hash::Hasher>(&self, state: &mut __H) -> () {
        match (&*self,) {
            _ => ::core::hash::Hash::hash(
                &unsafe { ::core::intrinsics::discriminant_value(self) },
                state,
            ),
        }
    }
}
impl ::core::marker::StructuralEq for Eventtype {}
#[automatically_derived]
#[allow(unused_qualifications)]
impl ::core::cmp::Eq for Eventtype {
    #[inline]
    #[doc(hidden)]
    fn assert_receiver_is_total_eq(&self) -> () {
        {}
    }
}
impl ::core::marker::StructuralPartialEq for Eventtype {}
#[automatically_derived]
#[allow(unused_qualifications)]
impl ::core::cmp::PartialEq for Eventtype {
    #[inline]
    fn eq(&self, other: &Eventtype) -> bool {
        {
            let __self_vi = unsafe { ::core::intrinsics::discriminant_value(&*self) } as u8;
            let __arg_1_vi = unsafe { ::core::intrinsics::discriminant_value(&*other) } as u8;
            if true && __self_vi == __arg_1_vi {
                match (&*self, &*other) {
                    _ => true,
                }
            } else {
                false
            }
        }
    }
}
#[repr(transparent)]
pub struct Eventrwflags(u16);
#[automatically_derived]
#[allow(unused_qualifications)]
impl ::core::marker::Copy for Eventrwflags {}
#[automatically_derived]
#[allow(unused_qualifications)]
impl ::core::clone::Clone for Eventrwflags {
    #[inline]
    fn clone(&self) -> Eventrwflags {
        {
            let _: ::core::clone::AssertParamIsClone<u16>;
            *self
        }
    }
}
#[automatically_derived]
#[allow(unused_qualifications)]
impl ::core::fmt::Debug for Eventrwflags {
    fn fmt(&self, f: &mut ::core::fmt::Formatter) -> ::core::fmt::Result {
        match *self {
            Eventrwflags(ref __self_0_0) => {
                let mut debug_trait_builder = f.debug_tuple("Eventrwflags");
                let _ = debug_trait_builder.field(&&(*__self_0_0));
                debug_trait_builder.finish()
            }
        }
    }
}
#[automatically_derived]
#[allow(unused_qualifications)]
impl ::core::hash::Hash for Eventrwflags {
    fn hash<__H: ::core::hash::Hasher>(&self, state: &mut __H) -> () {
        match *self {
            Eventrwflags(ref __self_0_0) => ::core::hash::Hash::hash(&(*__self_0_0), state),
        }
    }
}
impl ::core::marker::StructuralEq for Eventrwflags {}
#[automatically_derived]
#[allow(unused_qualifications)]
impl ::core::cmp::Eq for Eventrwflags {
    #[inline]
    #[doc(hidden)]
    fn assert_receiver_is_total_eq(&self) -> () {
        {
            let _: ::core::cmp::AssertParamIsEq<u16>;
        }
    }
}
impl ::core::marker::StructuralPartialEq for Eventrwflags {}
#[automatically_derived]
#[allow(unused_qualifications)]
impl ::core::cmp::PartialEq for Eventrwflags {
    #[inline]
    fn eq(&self, other: &Eventrwflags) -> bool {
        match *other {
            Eventrwflags(ref __self_1_0) => match *self {
                Eventrwflags(ref __self_0_0) => (*__self_0_0) == (*__self_1_0),
            },
        }
    }
    #[inline]
    fn ne(&self, other: &Eventrwflags) -> bool {
        match *other {
            Eventrwflags(ref __self_1_0) => match *self {
                Eventrwflags(ref __self_0_0) => (*__self_0_0) != (*__self_1_0),
            },
        }
    }
}
impl Eventrwflags {
    pub const FD_READWRITE_HANGUP: Eventrwflags = Eventrwflags(1);
}
#[repr(C)]
pub struct EventFdReadwrite {
    pub nbytes: Filesize,
    pub flags: Eventrwflags,
}
#[automatically_derived]
#[allow(unused_qualifications)]
impl ::core::marker::Copy for EventFdReadwrite {}
#[automatically_derived]
#[allow(unused_qualifications)]
impl ::core::clone::Clone for EventFdReadwrite {
    #[inline]
    fn clone(&self) -> EventFdReadwrite {
        {
            let _: ::core::clone::AssertParamIsClone<Filesize>;
            let _: ::core::clone::AssertParamIsClone<Eventrwflags>;
            *self
        }
    }
}
#[automatically_derived]
#[allow(unused_qualifications)]
impl ::core::fmt::Debug for EventFdReadwrite {
    fn fmt(&self, f: &mut ::core::fmt::Formatter) -> ::core::fmt::Result {
        match *self {
            EventFdReadwrite {
                nbytes: ref __self_0_0,
                flags: ref __self_0_1,
            } => {
                let mut debug_trait_builder = f.debug_struct("EventFdReadwrite");
                let _ = debug_trait_builder.field("nbytes", &&(*__self_0_0));
                let _ = debug_trait_builder.field("flags", &&(*__self_0_1));
                debug_trait_builder.finish()
            }
        }
    }
}
#[automatically_derived]
#[allow(unused_qualifications)]
impl ::core::hash::Hash for EventFdReadwrite {
    fn hash<__H: ::core::hash::Hasher>(&self, state: &mut __H) -> () {
        match *self {
            EventFdReadwrite {
                nbytes: ref __self_0_0,
                flags: ref __self_0_1,
            } => {
                ::core::hash::Hash::hash(&(*__self_0_0), state);
                ::core::hash::Hash::hash(&(*__self_0_1), state)
            }
        }
    }
}
impl ::core::marker::StructuralEq for EventFdReadwrite {}
#[automatically_derived]
#[allow(unused_qualifications)]
impl ::core::cmp::Eq for EventFdReadwrite {
    #[inline]
    #[doc(hidden)]
    fn assert_receiver_is_total_eq(&self) -> () {
        {
            let _: ::core::cmp::AssertParamIsEq<Filesize>;
            let _: ::core::cmp::AssertParamIsEq<Eventrwflags>;
        }
    }
}
impl ::core::marker::StructuralPartialEq for EventFdReadwrite {}
#[automatically_derived]
#[allow(unused_qualifications)]
impl ::core::cmp::PartialEq for EventFdReadwrite {
    #[inline]
    fn eq(&self, other: &EventFdReadwrite) -> bool {
        match *other {
            EventFdReadwrite {
                nbytes: ref __self_1_0,
                flags: ref __self_1_1,
            } => match *self {
                EventFdReadwrite {
                    nbytes: ref __self_0_0,
                    flags: ref __self_0_1,
                } => (*__self_0_0) == (*__self_1_0) && (*__self_0_1) == (*__self_1_1),
            },
        }
    }
    #[inline]
    fn ne(&self, other: &EventFdReadwrite) -> bool {
        match *other {
            EventFdReadwrite {
                nbytes: ref __self_1_0,
                flags: ref __self_1_1,
            } => match *self {
                EventFdReadwrite {
                    nbytes: ref __self_0_0,
                    flags: ref __self_0_1,
                } => (*__self_0_0) != (*__self_1_0) || (*__self_0_1) != (*__self_1_1),
            },
        }
    }
}
#[repr(C)]
#[allow(missing_debug_implementations)]
pub union EventU {
    pub fd_readwrite: EventFdReadwrite,
}
#[automatically_derived]
#[allow(unused_qualifications)]
#[allow(missing_debug_implementations)]
impl ::core::marker::Copy for EventU {}
#[automatically_derived]
#[allow(unused_qualifications)]
#[allow(missing_debug_implementations)]
impl ::core::clone::Clone for EventU {
    #[inline]
    fn clone(&self) -> EventU {
        {
            let _: ::core::clone::AssertParamIsCopy<Self>;
            *self
        }
    }
}
#[repr(C)]
#[allow(missing_debug_implementations)]
pub struct Event {
    pub userdata: Userdata,
    pub error: Errno,
    pub r#type: Eventtype,
    pub u: EventU,
}
#[automatically_derived]
#[allow(unused_qualifications)]
#[allow(missing_debug_implementations)]
impl ::core::marker::Copy for Event {}
#[automatically_derived]
#[allow(unused_qualifications)]
#[allow(missing_debug_implementations)]
impl ::core::clone::Clone for Event {
    #[inline]
    fn clone(&self) -> Event {
        {
            let _: ::core::clone::AssertParamIsClone<Userdata>;
            let _: ::core::clone::AssertParamIsClone<Errno>;
            let _: ::core::clone::AssertParamIsClone<Eventtype>;
            let _: ::core::clone::AssertParamIsClone<EventU>;
            *self
        }
    }
}
#[repr(transparent)]
pub struct Subclockflags(u16);
#[automatically_derived]
#[allow(unused_qualifications)]
impl ::core::marker::Copy for Subclockflags {}
#[automatically_derived]
#[allow(unused_qualifications)]
impl ::core::clone::Clone for Subclockflags {
    #[inline]
    fn clone(&self) -> Subclockflags {
        {
            let _: ::core::clone::AssertParamIsClone<u16>;
            *self
        }
    }
}
#[automatically_derived]
#[allow(unused_qualifications)]
impl ::core::fmt::Debug for Subclockflags {
    fn fmt(&self, f: &mut ::core::fmt::Formatter) -> ::core::fmt::Result {
        match *self {
            Subclockflags(ref __self_0_0) => {
                let mut debug_trait_builder = f.debug_tuple("Subclockflags");
                let _ = debug_trait_builder.field(&&(*__self_0_0));
                debug_trait_builder.finish()
            }
        }
    }
}
#[automatically_derived]
#[allow(unused_qualifications)]
impl ::core::hash::Hash for Subclockflags {
    fn hash<__H: ::core::hash::Hasher>(&self, state: &mut __H) -> () {
        match *self {
            Subclockflags(ref __self_0_0) => ::core::hash::Hash::hash(&(*__self_0_0), state),
        }
    }
}
impl ::core::marker::StructuralEq for Subclockflags {}
#[automatically_derived]
#[allow(unused_qualifications)]
impl ::core::cmp::Eq for Subclockflags {
    #[inline]
    #[doc(hidden)]
    fn assert_receiver_is_total_eq(&self) -> () {
        {
            let _: ::core::cmp::AssertParamIsEq<u16>;
        }
    }
}
impl ::core::marker::StructuralPartialEq for Subclockflags {}
#[automatically_derived]
#[allow(unused_qualifications)]
impl ::core::cmp::PartialEq for Subclockflags {
    #[inline]
    fn eq(&self, other: &Subclockflags) -> bool {
        match *other {
            Subclockflags(ref __self_1_0) => match *self {
                Subclockflags(ref __self_0_0) => (*__self_0_0) == (*__self_1_0),
            },
        }
    }
    #[inline]
    fn ne(&self, other: &Subclockflags) -> bool {
        match *other {
            Subclockflags(ref __self_1_0) => match *self {
                Subclockflags(ref __self_0_0) => (*__self_0_0) != (*__self_1_0),
            },
        }
    }
}
impl Subclockflags {
    pub const SUBSCRIPTION_CLOCK_ABSTIME: Subclockflags = Subclockflags(1);
}
#[repr(C)]
pub struct SubscriptionClock {
    pub id: Clockid,
    pub timeout: Timestamp,
    pub precision: Timestamp,
    pub flags: Subclockflags,
}
#[automatically_derived]
#[allow(unused_qualifications)]
impl ::core::marker::Copy for SubscriptionClock {}
#[automatically_derived]
#[allow(unused_qualifications)]
impl ::core::clone::Clone for SubscriptionClock {
    #[inline]
    fn clone(&self) -> SubscriptionClock {
        {
            let _: ::core::clone::AssertParamIsClone<Clockid>;
            let _: ::core::clone::AssertParamIsClone<Timestamp>;
            let _: ::core::clone::AssertParamIsClone<Timestamp>;
            let _: ::core::clone::AssertParamIsClone<Subclockflags>;
            *self
        }
    }
}
#[automatically_derived]
#[allow(unused_qualifications)]
impl ::core::fmt::Debug for SubscriptionClock {
    fn fmt(&self, f: &mut ::core::fmt::Formatter) -> ::core::fmt::Result {
        match *self {
            SubscriptionClock {
                id: ref __self_0_0,
                timeout: ref __self_0_1,
                precision: ref __self_0_2,
                flags: ref __self_0_3,
            } => {
                let mut debug_trait_builder = f.debug_struct("SubscriptionClock");
                let _ = debug_trait_builder.field("id", &&(*__self_0_0));
                let _ = debug_trait_builder.field("timeout", &&(*__self_0_1));
                let _ = debug_trait_builder.field("precision", &&(*__self_0_2));
                let _ = debug_trait_builder.field("flags", &&(*__self_0_3));
                debug_trait_builder.finish()
            }
        }
    }
}
#[automatically_derived]
#[allow(unused_qualifications)]
impl ::core::hash::Hash for SubscriptionClock {
    fn hash<__H: ::core::hash::Hasher>(&self, state: &mut __H) -> () {
        match *self {
            SubscriptionClock {
                id: ref __self_0_0,
                timeout: ref __self_0_1,
                precision: ref __self_0_2,
                flags: ref __self_0_3,
            } => {
                ::core::hash::Hash::hash(&(*__self_0_0), state);
                ::core::hash::Hash::hash(&(*__self_0_1), state);
                ::core::hash::Hash::hash(&(*__self_0_2), state);
                ::core::hash::Hash::hash(&(*__self_0_3), state)
            }
        }
    }
}
impl ::core::marker::StructuralEq for SubscriptionClock {}
#[automatically_derived]
#[allow(unused_qualifications)]
impl ::core::cmp::Eq for SubscriptionClock {
    #[inline]
    #[doc(hidden)]
    fn assert_receiver_is_total_eq(&self) -> () {
        {
            let _: ::core::cmp::AssertParamIsEq<Clockid>;
            let _: ::core::cmp::AssertParamIsEq<Timestamp>;
            let _: ::core::cmp::AssertParamIsEq<Timestamp>;
            let _: ::core::cmp::AssertParamIsEq<Subclockflags>;
        }
    }
}
impl ::core::marker::StructuralPartialEq for SubscriptionClock {}
#[automatically_derived]
#[allow(unused_qualifications)]
impl ::core::cmp::PartialEq for SubscriptionClock {
    #[inline]
    fn eq(&self, other: &SubscriptionClock) -> bool {
        match *other {
            SubscriptionClock {
                id: ref __self_1_0,
                timeout: ref __self_1_1,
                precision: ref __self_1_2,
                flags: ref __self_1_3,
            } => match *self {
                SubscriptionClock {
                    id: ref __self_0_0,
                    timeout: ref __self_0_1,
                    precision: ref __self_0_2,
                    flags: ref __self_0_3,
                } => {
                    (*__self_0_0) == (*__self_1_0)
                        && (*__self_0_1) == (*__self_1_1)
                        && (*__self_0_2) == (*__self_1_2)
                        && (*__self_0_3) == (*__self_1_3)
                }
            },
        }
    }
    #[inline]
    fn ne(&self, other: &SubscriptionClock) -> bool {
        match *other {
            SubscriptionClock {
                id: ref __self_1_0,
                timeout: ref __self_1_1,
                precision: ref __self_1_2,
                flags: ref __self_1_3,
            } => match *self {
                SubscriptionClock {
                    id: ref __self_0_0,
                    timeout: ref __self_0_1,
                    precision: ref __self_0_2,
                    flags: ref __self_0_3,
                } => {
                    (*__self_0_0) != (*__self_1_0)
                        || (*__self_0_1) != (*__self_1_1)
                        || (*__self_0_2) != (*__self_1_2)
                        || (*__self_0_3) != (*__self_1_3)
                }
            },
        }
    }
}
#[repr(C)]
pub struct SubscriptionFdReadwrite {
    pub file_descriptor: Fd,
}
#[automatically_derived]
#[allow(unused_qualifications)]
impl ::core::marker::Copy for SubscriptionFdReadwrite {}
#[automatically_derived]
#[allow(unused_qualifications)]
impl ::core::clone::Clone for SubscriptionFdReadwrite {
    #[inline]
    fn clone(&self) -> SubscriptionFdReadwrite {
        {
            let _: ::core::clone::AssertParamIsClone<Fd>;
            *self
        }
    }
}
#[automatically_derived]
#[allow(unused_qualifications)]
impl ::core::fmt::Debug for SubscriptionFdReadwrite {
    fn fmt(&self, f: &mut ::core::fmt::Formatter) -> ::core::fmt::Result {
        match *self {
            SubscriptionFdReadwrite {
                file_descriptor: ref __self_0_0,
            } => {
                let mut debug_trait_builder = f.debug_struct("SubscriptionFdReadwrite");
                let _ = debug_trait_builder.field("file_descriptor", &&(*__self_0_0));
                debug_trait_builder.finish()
            }
        }
    }
}
#[automatically_derived]
#[allow(unused_qualifications)]
impl ::core::hash::Hash for SubscriptionFdReadwrite {
    fn hash<__H: ::core::hash::Hasher>(&self, state: &mut __H) -> () {
        match *self {
            SubscriptionFdReadwrite {
                file_descriptor: ref __self_0_0,
            } => ::core::hash::Hash::hash(&(*__self_0_0), state),
        }
    }
}
impl ::core::marker::StructuralEq for SubscriptionFdReadwrite {}
#[automatically_derived]
#[allow(unused_qualifications)]
impl ::core::cmp::Eq for SubscriptionFdReadwrite {
    #[inline]
    #[doc(hidden)]
    fn assert_receiver_is_total_eq(&self) -> () {
        {
            let _: ::core::cmp::AssertParamIsEq<Fd>;
        }
    }
}
impl ::core::marker::StructuralPartialEq for SubscriptionFdReadwrite {}
#[automatically_derived]
#[allow(unused_qualifications)]
impl ::core::cmp::PartialEq for SubscriptionFdReadwrite {
    #[inline]
    fn eq(&self, other: &SubscriptionFdReadwrite) -> bool {
        match *other {
            SubscriptionFdReadwrite {
                file_descriptor: ref __self_1_0,
            } => match *self {
                SubscriptionFdReadwrite {
                    file_descriptor: ref __self_0_0,
                } => (*__self_0_0) == (*__self_1_0),
            },
        }
    }
    #[inline]
    fn ne(&self, other: &SubscriptionFdReadwrite) -> bool {
        match *other {
            SubscriptionFdReadwrite {
                file_descriptor: ref __self_1_0,
            } => match *self {
                SubscriptionFdReadwrite {
                    file_descriptor: ref __self_0_0,
                } => (*__self_0_0) != (*__self_1_0),
            },
        }
    }
}
#[repr(C)]
#[allow(missing_debug_implementations)]
pub union SubscriptionU {
    pub clock: SubscriptionClock,
    pub fd_readwrite: SubscriptionFdReadwrite,
}
#[automatically_derived]
#[allow(unused_qualifications)]
#[allow(missing_debug_implementations)]
impl ::core::marker::Copy for SubscriptionU {}
#[automatically_derived]
#[allow(unused_qualifications)]
#[allow(missing_debug_implementations)]
impl ::core::clone::Clone for SubscriptionU {
    #[inline]
    fn clone(&self) -> SubscriptionU {
        {
            let _: ::core::clone::AssertParamIsCopy<Self>;
            *self
        }
    }
}
#[repr(C)]
#[allow(missing_debug_implementations)]
pub struct Subscription {
    pub userdata: Userdata,
    pub r#type: Eventtype,
    pub u: SubscriptionU,
}
#[automatically_derived]
#[allow(unused_qualifications)]
#[allow(missing_debug_implementations)]
impl ::core::marker::Copy for Subscription {}
#[automatically_derived]
#[allow(unused_qualifications)]
#[allow(missing_debug_implementations)]
impl ::core::clone::Clone for Subscription {
    #[inline]
    fn clone(&self) -> Subscription {
        {
            let _: ::core::clone::AssertParamIsClone<Userdata>;
            let _: ::core::clone::AssertParamIsClone<Eventtype>;
            let _: ::core::clone::AssertParamIsClone<SubscriptionU>;
            *self
        }
    }
}
pub type Exitcode = u32;
#[repr(u8)]
pub enum Signal {
    None,
    Hup,
    Int,
    Quit,
    Ill,
    Trap,
    Abrt,
    Bus,
    Fpe,
    Kill,
    Usr1,
    Segv,
    Usr2,
    Pipe,
    Alrm,
    Term,
    Chld,
    Cont,
    Stop,
    Tstp,
    Ttin,
    Ttou,
    Urg,
    Xcpu,
    Xfsz,
    Vtalrm,
    Prof,
    Winch,
    Poll,
    Pwr,
    Sys,
}
#[automatically_derived]
#[allow(unused_qualifications)]
impl ::core::marker::Copy for Signal {}
#[automatically_derived]
#[allow(unused_qualifications)]
impl ::core::clone::Clone for Signal {
    #[inline]
    fn clone(&self) -> Signal {
        {
            *self
        }
    }
}
#[automatically_derived]
#[allow(unused_qualifications)]
impl ::core::fmt::Debug for Signal {
    fn fmt(&self, f: &mut ::core::fmt::Formatter) -> ::core::fmt::Result {
        match (&*self,) {
            (&Signal::None,) => {
                let mut debug_trait_builder = f.debug_tuple("None");
                debug_trait_builder.finish()
            }
            (&Signal::Hup,) => {
                let mut debug_trait_builder = f.debug_tuple("Hup");
                debug_trait_builder.finish()
            }
            (&Signal::Int,) => {
                let mut debug_trait_builder = f.debug_tuple("Int");
                debug_trait_builder.finish()
            }
            (&Signal::Quit,) => {
                let mut debug_trait_builder = f.debug_tuple("Quit");
                debug_trait_builder.finish()
            }
            (&Signal::Ill,) => {
                let mut debug_trait_builder = f.debug_tuple("Ill");
                debug_trait_builder.finish()
            }
            (&Signal::Trap,) => {
                let mut debug_trait_builder = f.debug_tuple("Trap");
                debug_trait_builder.finish()
            }
            (&Signal::Abrt,) => {
                let mut debug_trait_builder = f.debug_tuple("Abrt");
                debug_trait_builder.finish()
            }
            (&Signal::Bus,) => {
                let mut debug_trait_builder = f.debug_tuple("Bus");
                debug_trait_builder.finish()
            }
            (&Signal::Fpe,) => {
                let mut debug_trait_builder = f.debug_tuple("Fpe");
                debug_trait_builder.finish()
            }
            (&Signal::Kill,) => {
                let mut debug_trait_builder = f.debug_tuple("Kill");
                debug_trait_builder.finish()
            }
            (&Signal::Usr1,) => {
                let mut debug_trait_builder = f.debug_tuple("Usr1");
                debug_trait_builder.finish()
            }
            (&Signal::Segv,) => {
                let mut debug_trait_builder = f.debug_tuple("Segv");
                debug_trait_builder.finish()
            }
            (&Signal::Usr2,) => {
                let mut debug_trait_builder = f.debug_tuple("Usr2");
                debug_trait_builder.finish()
            }
            (&Signal::Pipe,) => {
                let mut debug_trait_builder = f.debug_tuple("Pipe");
                debug_trait_builder.finish()
            }
            (&Signal::Alrm,) => {
                let mut debug_trait_builder = f.debug_tuple("Alrm");
                debug_trait_builder.finish()
            }
            (&Signal::Term,) => {
                let mut debug_trait_builder = f.debug_tuple("Term");
                debug_trait_builder.finish()
            }
            (&Signal::Chld,) => {
                let mut debug_trait_builder = f.debug_tuple("Chld");
                debug_trait_builder.finish()
            }
            (&Signal::Cont,) => {
                let mut debug_trait_builder = f.debug_tuple("Cont");
                debug_trait_builder.finish()
            }
            (&Signal::Stop,) => {
                let mut debug_trait_builder = f.debug_tuple("Stop");
                debug_trait_builder.finish()
            }
            (&Signal::Tstp,) => {
                let mut debug_trait_builder = f.debug_tuple("Tstp");
                debug_trait_builder.finish()
            }
            (&Signal::Ttin,) => {
                let mut debug_trait_builder = f.debug_tuple("Ttin");
                debug_trait_builder.finish()
            }
            (&Signal::Ttou,) => {
                let mut debug_trait_builder = f.debug_tuple("Ttou");
                debug_trait_builder.finish()
            }
            (&Signal::Urg,) => {
                let mut debug_trait_builder = f.debug_tuple("Urg");
                debug_trait_builder.finish()
            }
            (&Signal::Xcpu,) => {
                let mut debug_trait_builder = f.debug_tuple("Xcpu");
                debug_trait_builder.finish()
            }
            (&Signal::Xfsz,) => {
                let mut debug_trait_builder = f.debug_tuple("Xfsz");
                debug_trait_builder.finish()
            }
            (&Signal::Vtalrm,) => {
                let mut debug_trait_builder = f.debug_tuple("Vtalrm");
                debug_trait_builder.finish()
            }
            (&Signal::Prof,) => {
                let mut debug_trait_builder = f.debug_tuple("Prof");
                debug_trait_builder.finish()
            }
            (&Signal::Winch,) => {
                let mut debug_trait_builder = f.debug_tuple("Winch");
                debug_trait_builder.finish()
            }
            (&Signal::Poll,) => {
                let mut debug_trait_builder = f.debug_tuple("Poll");
                debug_trait_builder.finish()
            }
            (&Signal::Pwr,) => {
                let mut debug_trait_builder = f.debug_tuple("Pwr");
                debug_trait_builder.finish()
            }
            (&Signal::Sys,) => {
                let mut debug_trait_builder = f.debug_tuple("Sys");
                debug_trait_builder.finish()
            }
        }
    }
}
#[automatically_derived]
#[allow(unused_qualifications)]
impl ::core::hash::Hash for Signal {
    fn hash<__H: ::core::hash::Hasher>(&self, state: &mut __H) -> () {
        match (&*self,) {
            _ => ::core::hash::Hash::hash(
                &unsafe { ::core::intrinsics::discriminant_value(self) },
                state,
            ),
        }
    }
}
impl ::core::marker::StructuralEq for Signal {}
#[automatically_derived]
#[allow(unused_qualifications)]
impl ::core::cmp::Eq for Signal {
    #[inline]
    #[doc(hidden)]
    fn assert_receiver_is_total_eq(&self) -> () {
        {}
    }
}
impl ::core::marker::StructuralPartialEq for Signal {}
#[automatically_derived]
#[allow(unused_qualifications)]
impl ::core::cmp::PartialEq for Signal {
    #[inline]
    fn eq(&self, other: &Signal) -> bool {
        {
            let __self_vi = unsafe { ::core::intrinsics::discriminant_value(&*self) } as u8;
            let __arg_1_vi = unsafe { ::core::intrinsics::discriminant_value(&*other) } as u8;
            if true && __self_vi == __arg_1_vi {
                match (&*self, &*other) {
                    _ => true,
                }
            } else {
                false
            }
        }
    }
}
#[repr(transparent)]
pub struct Riflags(u16);
#[automatically_derived]
#[allow(unused_qualifications)]
impl ::core::marker::Copy for Riflags {}
#[automatically_derived]
#[allow(unused_qualifications)]
impl ::core::clone::Clone for Riflags {
    #[inline]
    fn clone(&self) -> Riflags {
        {
            let _: ::core::clone::AssertParamIsClone<u16>;
            *self
        }
    }
}
#[automatically_derived]
#[allow(unused_qualifications)]
impl ::core::fmt::Debug for Riflags {
    fn fmt(&self, f: &mut ::core::fmt::Formatter) -> ::core::fmt::Result {
        match *self {
            Riflags(ref __self_0_0) => {
                let mut debug_trait_builder = f.debug_tuple("Riflags");
                let _ = debug_trait_builder.field(&&(*__self_0_0));
                debug_trait_builder.finish()
            }
        }
    }
}
#[automatically_derived]
#[allow(unused_qualifications)]
impl ::core::hash::Hash for Riflags {
    fn hash<__H: ::core::hash::Hasher>(&self, state: &mut __H) -> () {
        match *self {
            Riflags(ref __self_0_0) => ::core::hash::Hash::hash(&(*__self_0_0), state),
        }
    }
}
impl ::core::marker::StructuralEq for Riflags {}
#[automatically_derived]
#[allow(unused_qualifications)]
impl ::core::cmp::Eq for Riflags {
    #[inline]
    #[doc(hidden)]
    fn assert_receiver_is_total_eq(&self) -> () {
        {
            let _: ::core::cmp::AssertParamIsEq<u16>;
        }
    }
}
impl ::core::marker::StructuralPartialEq for Riflags {}
#[automatically_derived]
#[allow(unused_qualifications)]
impl ::core::cmp::PartialEq for Riflags {
    #[inline]
    fn eq(&self, other: &Riflags) -> bool {
        match *other {
            Riflags(ref __self_1_0) => match *self {
                Riflags(ref __self_0_0) => (*__self_0_0) == (*__self_1_0),
            },
        }
    }
    #[inline]
    fn ne(&self, other: &Riflags) -> bool {
        match *other {
            Riflags(ref __self_1_0) => match *self {
                Riflags(ref __self_0_0) => (*__self_0_0) != (*__self_1_0),
            },
        }
    }
}
impl Riflags {
    pub const RECV_PEEK: Riflags = Riflags(1);
    pub const RECV_WAITALL: Riflags = Riflags(2);
}
#[repr(transparent)]
pub struct Roflags(u16);
#[automatically_derived]
#[allow(unused_qualifications)]
impl ::core::marker::Copy for Roflags {}
#[automatically_derived]
#[allow(unused_qualifications)]
impl ::core::clone::Clone for Roflags {
    #[inline]
    fn clone(&self) -> Roflags {
        {
            let _: ::core::clone::AssertParamIsClone<u16>;
            *self
        }
    }
}
#[automatically_derived]
#[allow(unused_qualifications)]
impl ::core::fmt::Debug for Roflags {
    fn fmt(&self, f: &mut ::core::fmt::Formatter) -> ::core::fmt::Result {
        match *self {
            Roflags(ref __self_0_0) => {
                let mut debug_trait_builder = f.debug_tuple("Roflags");
                let _ = debug_trait_builder.field(&&(*__self_0_0));
                debug_trait_builder.finish()
            }
        }
    }
}
#[automatically_derived]
#[allow(unused_qualifications)]
impl ::core::hash::Hash for Roflags {
    fn hash<__H: ::core::hash::Hasher>(&self, state: &mut __H) -> () {
        match *self {
            Roflags(ref __self_0_0) => ::core::hash::Hash::hash(&(*__self_0_0), state),
        }
    }
}
impl ::core::marker::StructuralEq for Roflags {}
#[automatically_derived]
#[allow(unused_qualifications)]
impl ::core::cmp::Eq for Roflags {
    #[inline]
    #[doc(hidden)]
    fn assert_receiver_is_total_eq(&self) -> () {
        {
            let _: ::core::cmp::AssertParamIsEq<u16>;
        }
    }
}
impl ::core::marker::StructuralPartialEq for Roflags {}
#[automatically_derived]
#[allow(unused_qualifications)]
impl ::core::cmp::PartialEq for Roflags {
    #[inline]
    fn eq(&self, other: &Roflags) -> bool {
        match *other {
            Roflags(ref __self_1_0) => match *self {
                Roflags(ref __self_0_0) => (*__self_0_0) == (*__self_1_0),
            },
        }
    }
    #[inline]
    fn ne(&self, other: &Roflags) -> bool {
        match *other {
            Roflags(ref __self_1_0) => match *self {
                Roflags(ref __self_0_0) => (*__self_0_0) != (*__self_1_0),
            },
        }
    }
}
impl Roflags {
    pub const RECV_DATA_TRUNCATED: Roflags = Roflags(1);
}
pub type Siflags = u16;
#[repr(transparent)]
pub struct Sdflags(u8);
#[automatically_derived]
#[allow(unused_qualifications)]
impl ::core::marker::Copy for Sdflags {}
#[automatically_derived]
#[allow(unused_qualifications)]
impl ::core::clone::Clone for Sdflags {
    #[inline]
    fn clone(&self) -> Sdflags {
        {
            let _: ::core::clone::AssertParamIsClone<u8>;
            *self
        }
    }
}
#[automatically_derived]
#[allow(unused_qualifications)]
impl ::core::fmt::Debug for Sdflags {
    fn fmt(&self, f: &mut ::core::fmt::Formatter) -> ::core::fmt::Result {
        match *self {
            Sdflags(ref __self_0_0) => {
                let mut debug_trait_builder = f.debug_tuple("Sdflags");
                let _ = debug_trait_builder.field(&&(*__self_0_0));
                debug_trait_builder.finish()
            }
        }
    }
}
#[automatically_derived]
#[allow(unused_qualifications)]
impl ::core::hash::Hash for Sdflags {
    fn hash<__H: ::core::hash::Hasher>(&self, state: &mut __H) -> () {
        match *self {
            Sdflags(ref __self_0_0) => ::core::hash::Hash::hash(&(*__self_0_0), state),
        }
    }
}
impl ::core::marker::StructuralEq for Sdflags {}
#[automatically_derived]
#[allow(unused_qualifications)]
impl ::core::cmp::Eq for Sdflags {
    #[inline]
    #[doc(hidden)]
    fn assert_receiver_is_total_eq(&self) -> () {
        {
            let _: ::core::cmp::AssertParamIsEq<u8>;
        }
    }
}
impl ::core::marker::StructuralPartialEq for Sdflags {}
#[automatically_derived]
#[allow(unused_qualifications)]
impl ::core::cmp::PartialEq for Sdflags {
    #[inline]
    fn eq(&self, other: &Sdflags) -> bool {
        match *other {
            Sdflags(ref __self_1_0) => match *self {
                Sdflags(ref __self_0_0) => (*__self_0_0) == (*__self_1_0),
            },
        }
    }
    #[inline]
    fn ne(&self, other: &Sdflags) -> bool {
        match *other {
            Sdflags(ref __self_1_0) => match *self {
                Sdflags(ref __self_0_0) => (*__self_0_0) != (*__self_1_0),
            },
        }
    }
}
impl Sdflags {
    pub const RD: Sdflags = Sdflags(1);
    pub const WR: Sdflags = Sdflags(2);
}
#[repr(u8)]
pub enum Preopentype {
    Dir,
}
#[automatically_derived]
#[allow(unused_qualifications)]
impl ::core::marker::Copy for Preopentype {}
#[automatically_derived]
#[allow(unused_qualifications)]
impl ::core::clone::Clone for Preopentype {
    #[inline]
    fn clone(&self) -> Preopentype {
        {
            *self
        }
    }
}
#[automatically_derived]
#[allow(unused_qualifications)]
impl ::core::fmt::Debug for Preopentype {
    fn fmt(&self, f: &mut ::core::fmt::Formatter) -> ::core::fmt::Result {
        match (&*self,) {
            (&Preopentype::Dir,) => {
                let mut debug_trait_builder = f.debug_tuple("Dir");
                debug_trait_builder.finish()
            }
        }
    }
}
#[automatically_derived]
#[allow(unused_qualifications)]
impl ::core::hash::Hash for Preopentype {
    fn hash<__H: ::core::hash::Hasher>(&self, state: &mut __H) -> () {
        match (&*self,) {
            _ => {}
        }
    }
}
impl ::core::marker::StructuralEq for Preopentype {}
#[automatically_derived]
#[allow(unused_qualifications)]
impl ::core::cmp::Eq for Preopentype {
    #[inline]
    #[doc(hidden)]
    fn assert_receiver_is_total_eq(&self) -> () {
        {}
    }
}
impl ::core::marker::StructuralPartialEq for Preopentype {}
#[automatically_derived]
#[allow(unused_qualifications)]
impl ::core::cmp::PartialEq for Preopentype {
    #[inline]
    fn eq(&self, other: &Preopentype) -> bool {
        match (&*self, &*other) {
            _ => true,
        }
    }
}
