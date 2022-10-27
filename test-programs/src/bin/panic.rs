wit_bindgen_guest_rust::generate!({
    default: "../wit/command.wit.md",
    name: "app",
});

struct Exports;
export_app!(Exports);

impl app::App for Exports {
    fn command() {
        panic!("idk");
    }
}

fn main() {}
