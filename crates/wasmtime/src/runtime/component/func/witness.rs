
use crate::prelude::*;
use core::marker::PhantomData;
use alloc::sync::Arc;

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
    _marker: PhantomData<&'a ()>,
}

impl<'a> ParamWitness<'a> {

}

pub struct ResultWitness<'a> {
    _marker: PhantomData<&'a ()>,
}

impl<'a> ResultWitness<'a> {

}
