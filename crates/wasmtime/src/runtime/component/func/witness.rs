use crate::component::func::{LiftContext, Source, TypeFuncIndex};
use crate::prelude::*;
use alloc::sync::Arc;
use core::marker::PhantomData;

pub struct Witness<T> {
    param_witness: Arc<dyn Fn(&mut T, &[String], ParamWitness<'_>) -> Result<()> + Send + Sync>,
    result_witness: Arc<dyn Fn(&mut T, &[String], ResultWitness<'_>) -> Result<()> + Send + Sync>,
    position: Arc<Vec<String>>,
}
impl<T> Witness<T> {
    pub fn new(
        param: impl Fn(&mut T, &[String], ParamWitness<'_>) -> Result<()> + Send + Sync + 'static,
        result: impl Fn(&mut T, &[String], ResultWitness<'_>) -> Result<()> + Send + Sync + 'static,
    ) -> Self {
        Self {
            param_witness: Arc::new(param),
            result_witness: Arc::new(result),
            position: Arc::new(Vec::new()),
        }
    }
    pub fn in_instance(&self, name: &str) -> Self {
        let mut position: Vec<String> = (&*self.position).clone();
        position.push(name.to_owned());
        Self {
            param_witness: self.param_witness.clone(),
            result_witness: self.result_witness.clone(),
            position: Arc::new(position),
        }
    }

    pub fn params(&self, t: &mut T, pw: ParamWitness<'_>) -> Result<()> {
        (self.param_witness)(t, &self.position, pw)
    }
    pub fn results(&self, t: &mut T, rw: ResultWitness<'_>) -> Result<()> {
        (self.result_witness)(t, &self.position, rw)
    }
}

impl<T> Clone for Witness<T> {
    fn clone(&self) -> Self {
        Self {
            param_witness: self.param_witness.clone(),
            result_witness: self.result_witness.clone(),
            position: self.position.clone(),
        }
    }
}

pub struct ParamWitness<'a> {
    cx: LazyCtx<'a>,
}

impl<'a> ParamWitness<'a> {
    pub(super) fn new(cx: &'a LiftContext<'a>, ty: TypeFuncIndex, src: &'a Source<'a>) -> Self {
        Self {
            cx: LazyCtx { cx, ty, src },
        }
    }
    pub fn values(&self) -> impl Iterator<Item = (&'a str, LazyValue<'a>)> {
        // TODO FIXME
        [].into_iter()
    }
}

// Can we make a Func::new_lazy that takes &[LazyValue] and returns Val, and
// then Func::new is implemented in terms of Func::new_lazy

struct LazyCtx<'a> {
    // This will probably need to get threaded into each call by hand because
    // its the store
    cx: &'a LiftContext<'a>,
    ty: TypeFuncIndex,
    // This might remain and be the 'a ...
    src: &'a Source<'a>,
}

pub struct LazyValue<'a> {
    // This will need to be out of here actually and just be a cursor into the
    // sourcer. the 'a might go away
    cx: LazyCtx<'a>,
}

impl<'a> LazyValue<'a> {
    fn ty(&self) -> crate::component::Type {
        todo!()
    }
    fn force(&self) -> Result<LVal<'a>> {
        todo!()
    }
}

// Each leaf of LVal has a private
/*
    fn to_val(&self, store: &mut Store) -> Result<Val> {
        // The resource arms can be turned into a Val only once. Once they
        // have been, its illegal to ever do so again...
    }
*/
// And then there is a Val::from_lazy(LazyValue, &mut Store)
// the Host has to force an owned value from the guest table to the host
// table, or else you break the component model semantics. By taking on the
// use of the Func::new_lazy apis the host has the additional responsibility
// to not get that wrong!
//

// Document saying Resources have extra rules
//
// All of Val built on top of this
//
// Everything related to Lifting a Val is built here.
//
// New linker method gives you access to this raw/lazy world, which is
// documented to be extra dangerous.
//
// All of this is the basis for introspection, after the initial bits land.

// This reflects Val except everywhere there is a Box<Val> or Vec<Val> there
// is instead a LazyValue.
pub enum LVal<'a> {
    U32(u32),
    // .. and all the other atoms ..
    //
    // Each layer of every type has a method to_val: LazyString -> String,
    // LazyList -> WasmList, LazyRecord -> Record,
    //
    String(LazyString<'a>),
    List(LazyList<'a>),
    // XXX actually LazyRecord here...
    Record(Vec<(&'a str, LazyValue<'a>)>),
    Resource(LazyResource<'a>),
}

pub struct LazyString<'a> {
    cx: LazyCtx<'a>,
}

impl<'a> LazyString<'a> {
    pub fn force(&self) -> Result<&str> {
        todo!()
    }
}

pub struct LazyList<'a> {
    cx: LazyCtx<'a>,
}

impl<'a> LazyList<'a> {
    fn elem_ty(&self) -> crate::component::Type {
        todo!()
    }
    pub fn len(&self) -> Result<usize> {
        todo!()
    }
    pub fn get(&self, ix: usize) -> Result<LazyValue<'a>> {
        todo!()
    }
    pub fn elems(&self) -> Result<impl Iterator<Item = LazyValue<'a>>> {
        // TODO FIXME
        Ok([].into_iter())
    }
    pub fn as_slice_u8(&self) -> Result<&[u8]> {
        todo!()
    }
}

pub struct LazyResource<'a> {
    cx: LazyCtx<'a>,
}
impl<'a> LazyResource<'a> {
    pub fn ty(&self) -> crate::component::Type {
        todo!()
    }
    pub fn repr(&self) -> u32 {
        todo!()
    }
    // This is the one operation that needs to mutate the LiftContext, so this
    // would make us need a &mut <wrapper of LiftContext> passed around to all
    // the forces.
    pub fn force<T>(&self, store: &mut crate::Store<T>) -> Result<crate::component::ResourceAny> {
        todo!()
    }
}

pub struct ResultWitness<'a> {
    _marker: PhantomData<&'a ()>,
}

impl<'a> ResultWitness<'a> {}
