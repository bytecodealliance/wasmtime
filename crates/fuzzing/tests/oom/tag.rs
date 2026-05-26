#![cfg(arc_try_new)]

use wasmtime::{Config, Engine, FuncType, Result, Store, Tag, TagType};
use wasmtime_fuzzing::oom::OomTest;

#[test]
fn tag_new() -> Result<()> {
    let mut config = Config::new();
    config.enable_compiler(false);
    config.concurrency_support(false);
    let engine = Engine::new(&config)?;

    OomTest::new().allow_alloc_after_oom(true).test(|| {
        let mut store = Store::try_new(&engine, ())?;
        let func_ty = FuncType::try_new(&engine, [], [])?;
        let tag_ty = TagType::new(func_ty);
        let _tag = Tag::new(&mut store, &tag_ty)?;
        Ok(())
    })
}

#[test]
fn tag_ty() -> Result<()> {
    let mut config = Config::new();
    config.enable_compiler(false);
    config.concurrency_support(false);
    let engine = Engine::new(&config)?;

    OomTest::new().allow_alloc_after_oom(true).test(|| {
        let mut store = Store::try_new(&engine, ())?;
        let func_ty = FuncType::try_new(&engine, [], [])?;
        let tag_ty = TagType::new(func_ty);
        let tag = Tag::new(&mut store, &tag_ty)?;
        let _ty = tag.ty(&store);
        Ok(())
    })
}

#[test]
fn tag_eq() -> Result<()> {
    let mut config = Config::new();
    config.enable_compiler(false);
    config.concurrency_support(false);
    let engine = Engine::new(&config)?;

    OomTest::new().allow_alloc_after_oom(true).test(|| {
        let mut store = Store::try_new(&engine, ())?;
        let func_ty = FuncType::try_new(&engine, [], [])?;
        let tag_ty = TagType::new(func_ty);
        let tag1 = Tag::new(&mut store, &tag_ty)?;
        let tag2 = Tag::new(&mut store, &tag_ty)?;
        assert!(Tag::eq(&tag1, &tag1, &store));
        assert!(!Tag::eq(&tag1, &tag2, &store));
        Ok(())
    })
}
