mod bindings {
    wit_bindgen::generate!({
        inline: "
package echo:echo;
world echo {
    export echo: interface {
        echo: async func(echo: string) -> string;
    }
}
        ",
    });

    use super::Component;
    export!(Component);
}

struct Component;

impl bindings::exports::echo::Guest for Component {
    async fn echo(s: String) -> String {
        s
    }
}

// Unused function; required since this file is built as a `bin`:
fn main() {}
