use test_programs::wasi::clocks::*;
use test_programs::wasi::io::*;
use test_programs::wasi::sockets::*;

fn main() {
    let network = instance_network::instance_network();

    let addresses =
        ip_name_lookup::resolve_addresses(&network, "example.com", None, false).unwrap();
    let pollable = addresses.subscribe();
    poll::poll_one(&pollable);
    assert!(addresses.resolve_next_address().is_ok());

    let result = ip_name_lookup::resolve_addresses(&network, "a.b<&>", None, false);
    assert!(matches!(result, Err(network::ErrorCode::InvalidArgument)));

    // Try resolving a valid address and ensure that it eventually terminates.
    // To help prevent this test from being flaky this additionally times out
    // the resolution and allows errors.
    let addresses = ip_name_lookup::resolve_addresses(&network, "github.com", None, false).unwrap();
    let lookup = addresses.subscribe();
    let timeout = monotonic_clock::subscribe(1_000_000_000, false);
    let ready = poll::poll_list(&[&lookup, &timeout]);
    assert!(ready.len() > 0);
    match ready[0] {
        0 => loop {
            match addresses.resolve_next_address() {
                Ok(Some(_)) => {}
                Ok(None) => break,
                Err(_) => break,
            }
        },
        1 => {}
        _ => unreachable!(),
    }
}
