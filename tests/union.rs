use proptest::prelude::*;
use wiggle_runtime::{
    GuestError, GuestErrorType, GuestMemory, GuestPtr, GuestPtrMut, GuestRef, GuestRefMut,
};
use wiggle_test::{impl_errno, HostMemory, MemArea, WasiCtx};

wiggle_generate::from_witx!({
    witx: ["tests/union.witx"],
    ctx: WasiCtx,
});

impl_errno!(types::Errno);

impl union_example::UnionExample for WasiCtx {
    fn get_tag(&mut self, u: &types::Reason) -> Result<types::Excuse, types::Errno> {
        println!("GET TAG: {:?}", u);
        Ok(types::Excuse::DogAte)
    }
    fn reason_mult(
        &mut self,
        u: &types::ReasonMut<'_>,
        multiply_by: u32,
    ) -> Result<(), types::Errno> {
        match u {
            types::ReasonMut::DogAte(fptr) => {
                let mut f = fptr.as_ref_mut().expect("valid pointer");
                let val = *f;
                println!("REASON MULT DogAte({})", val);
                *f = val * multiply_by as f32;
            }
            types::ReasonMut::Traffic(iptr) => {
                let mut i = iptr.as_ref_mut().expect("valid pointer");
                let val = *i;
                println!("REASON MULT Traffic({})", val);
                *i = val * multiply_by as i32;
            }
            types::ReasonMut::Sleeping => {
                println!("REASON MULT Sleeping");
            }
        }
        Ok(())
    }
}
