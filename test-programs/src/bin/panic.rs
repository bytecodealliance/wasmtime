wit_bindgen_guest_rust::generate!({
    default: "../wit/command.wit.md",
    name: "app",
});

struct Exports;
export_app!(Exports);

impl app::App for Exports {
    fn command(_: u32, _: u32) {
        panic!("idk");
    }
}

fn main() {}
