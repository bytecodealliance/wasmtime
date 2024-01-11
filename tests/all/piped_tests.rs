#![cfg(not(miri))]

use super::cli_tests::get_wasmtime_command;
use anyhow::Result;
use std::process::{ChildStdout, Command, Stdio};

fn producer_command(component_path: &str) -> Result<Command> {
    let mut cmd = get_wasmtime_command()?;
    cmd.arg("run")
        .arg("-Wcomponent-model")
        .arg("--env")
        .arg("PIPED_SIDE=PRODUCER")
        .arg(component_path)
        .stdout(Stdio::piped());
    Ok(cmd)
}

fn consumer_command(component_path: &str, input: ChildStdout) -> Result<Command> {
    let mut cmd = get_wasmtime_command()?;
    cmd.arg("run")
        .arg("-Wcomponent-model")
        .arg("--env")
        .arg("PIPED_SIDE=CONSUMER")
        .arg(component_path)
        .stdin(input);
    Ok(cmd)
}

pub fn run_wasmtime_piped(component_path: &str) -> Result<()> {
    let mut producer = producer_command(component_path)?.spawn()?;
    let stdout = producer.stdout.take().unwrap();
    let mut consumer = consumer_command(component_path, stdout)?.spawn()?;

    let producer = producer.wait()?;
    if !producer.success() {
        // make sure the consumer gets killed off
        if consumer.try_wait().is_err() {
            consumer.kill().expect("Failed to kill consumer");
        }

        panic!("Producer failed");
    }

    assert!(consumer.wait()?.success(), "Consumer failed");

    Ok(())
}

mod test_programs {
    use super::run_wasmtime_piped;
    use test_programs_artifacts::*;

    macro_rules! assert_test_exists {
        ($name:ident) => {
            #[allow(unused_imports)]
            use self::$name as _;
        };
    }
    foreach_piped!(assert_test_exists);

    // Below here is mechanical: there should be one test for every binary in
    // wasi-tests.
    #[test]
    fn piped_simple() {
        run_wasmtime_piped(PIPED_SIMPLE_COMPONENT).unwrap()
    }

    #[test]
    fn piped_polling() {
        run_wasmtime_piped(PIPED_POLLING_COMPONENT).unwrap()
    }
}
