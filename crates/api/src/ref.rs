use std::any::Any;
use std::cell::{self, RefCell};
use std::fmt;
use std::rc::{Rc, Weak};

pub trait HostInfo {
    fn finalize(&mut self) {}
}

trait InternalRefBase: Any {
    fn as_any(&self) -> &dyn Any;
    fn host_info(&self) -> Option<cell::RefMut<Box<dyn HostInfo>>>;
    fn set_host_info(&self, info: Option<Box<dyn HostInfo>>);
    fn ptr_eq(&self, other: &dyn InternalRefBase) -> bool;
}

#[derive(Clone)]
pub struct InternalRef(Rc<dyn InternalRefBase>);

impl InternalRef {
    pub fn is_ref<T: 'static>(&self) -> bool {
        let r = self.0.as_any();
        Any::is::<HostRef<T>>(r)
    }
    pub fn get_ref<T: 'static>(&self) -> HostRef<T> {
        let r = self.0.as_any();
        r.downcast_ref::<HostRef<T>>()
            .expect("reference is not T type")
            .clone()
    }
}

struct AnyAndHostInfo {
    any: Box<dyn Any>,
    host_info: Option<Box<dyn HostInfo>>,
}

impl Drop for AnyAndHostInfo {
    fn drop(&mut self) {
        if let Some(info) = &mut self.host_info {
            info.finalize();
        }
    }
}

#[derive(Clone)]
pub struct OtherRef(Rc<RefCell<AnyAndHostInfo>>);

#[derive(Clone)]
pub enum AnyRef {
    Null,
    Ref(InternalRef),
    Other(OtherRef),
}

impl AnyRef {
    pub fn new(data: Box<dyn Any>) -> Self {
        let info = AnyAndHostInfo {
            any: data,
            host_info: None,
        };
        AnyRef::Other(OtherRef(Rc::new(RefCell::new(info))))
    }

    pub fn null() -> Self {
        AnyRef::Null
    }

    pub fn data(&self) -> cell::Ref<Box<dyn Any>> {
        match self {
            AnyRef::Other(OtherRef(r)) => cell::Ref::map(r.borrow(), |r| &r.any),
            _ => panic!("expected AnyRef::Other"),
        }
    }

    pub fn ptr_eq(&self, other: &AnyRef) -> bool {
        match (self, other) {
            (AnyRef::Null, AnyRef::Null) => true,
            (AnyRef::Ref(InternalRef(ref a)), AnyRef::Ref(InternalRef(ref b))) => {
                a.ptr_eq(b.as_ref())
            }
            (AnyRef::Other(OtherRef(ref a)), AnyRef::Other(OtherRef(ref b))) => Rc::ptr_eq(a, b),
            _ => false,
        }
    }

    pub fn host_info(&self) -> Option<cell::RefMut<Box<dyn HostInfo>>> {
        match self {
            AnyRef::Null => panic!("null"),
            AnyRef::Ref(r) => r.0.host_info(),
            AnyRef::Other(r) => {
                let info = cell::RefMut::map(r.0.borrow_mut(), |b| &mut b.host_info);
                if info.is_none() {
                    return None;
                }
                Some(cell::RefMut::map(info, |info| info.as_mut().unwrap()))
            }
        }
    }

    pub fn set_host_info(&self, info: Option<Box<dyn HostInfo>>) {
        match self {
            AnyRef::Null => panic!("null"),
            AnyRef::Ref(r) => r.0.set_host_info(info),
            AnyRef::Other(r) => {
                r.0.borrow_mut().host_info = info;
            }
        }
    }
}

impl fmt::Debug for AnyRef {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            AnyRef::Null => write!(f, "null"),
            AnyRef::Ref(_) => write!(f, "anyref"),
            AnyRef::Other(_) => write!(f, "other ref"),
        }
    }
}

struct ContentBox<T> {
    content: T,
    host_info: Option<Box<dyn HostInfo>>,
    anyref_data: Weak<dyn InternalRefBase>,
}

impl<T> Drop for ContentBox<T> {
    fn drop(&mut self) {
        if let Some(info) = &mut self.host_info {
            info.finalize();
        }
    }
}

pub struct HostRef<T>(Rc<RefCell<ContentBox<T>>>);

impl<T: 'static> HostRef<T> {
    pub fn new(item: T) -> HostRef<T> {
        let anyref_data: Weak<HostRef<T>> = Weak::new();
        let content = ContentBox {
            content: item,
            host_info: None,
            anyref_data,
        };
        HostRef(Rc::new(RefCell::new(content)))
    }

    pub fn borrow(&self) -> cell::Ref<T> {
        cell::Ref::map(self.0.borrow(), |b| &b.content)
    }

    pub fn borrow_mut(&self) -> cell::RefMut<T> {
        cell::RefMut::map(self.0.borrow_mut(), |b| &mut b.content)
    }

    pub fn ptr_eq(&self, other: &HostRef<T>) -> bool {
        Rc::ptr_eq(&self.0, &other.0)
    }

    pub fn anyref(&self) -> AnyRef {
        let r = self.0.borrow_mut().anyref_data.upgrade();
        if let Some(r) = r {
            return AnyRef::Ref(InternalRef(r));
        }
        let anyref_data: Rc<dyn InternalRefBase> = Rc::new(self.clone());
        self.0.borrow_mut().anyref_data = Rc::downgrade(&anyref_data);
        AnyRef::Ref(InternalRef(anyref_data))
    }
}

impl<T: 'static> InternalRefBase for HostRef<T> {
    fn ptr_eq(&self, other: &dyn InternalRefBase) -> bool {
        if let Some(other) = other.as_any().downcast_ref() {
            self.ptr_eq(other)
        } else {
            false
        }
    }

    fn as_any(&self) -> &dyn Any {
        self
    }

    fn host_info(&self) -> Option<cell::RefMut<Box<dyn HostInfo>>> {
        let info = cell::RefMut::map(self.0.borrow_mut(), |b| &mut b.host_info);
        if info.is_none() {
            return None;
        }
        Some(cell::RefMut::map(info, |info| info.as_mut().unwrap()))
    }

    fn set_host_info(&self, info: Option<Box<dyn HostInfo>>) {
        self.0.borrow_mut().host_info = info;
    }
}

impl<T> Clone for HostRef<T> {
    fn clone(&self) -> HostRef<T> {
        HostRef(self.0.clone())
    }
}

impl<T: fmt::Debug> fmt::Debug for HostRef<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Ref(")?;
        self.0.borrow().content.fmt(f)?;
        write!(f, ")")
    }
}
