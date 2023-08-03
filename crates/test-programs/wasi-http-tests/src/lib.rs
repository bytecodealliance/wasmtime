mod outbound_request;

wit_bindgen::generate!("wasi:preview/command-extended" in "../../wasi/wit");

struct Component;

impl CommandExtended for Component {
    fn run() -> Result<(), ()> {
        outbound_request::main().map_err(|e| eprintln!("{e:?}"))
    }
}

export_command_extended!(Component);
