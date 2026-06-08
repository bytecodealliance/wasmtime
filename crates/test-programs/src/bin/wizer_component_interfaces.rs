/// Stub so that Cargo can build this test as a binary
pub fn main() {
    eprintln!("dont use as a command");
    std::process::exit(-1)
}

wit_bindgen::generate!({
    inline: "
package local:local@0.1.0;

interface init {
    use run.{elem};
    add-string: func(arg: string) -> list<elem>;
    add-int: func(arg: s32) -> list<elem>;
}

interface run {
    variant elem { str(string), int(s32) }
    get-inits: func() -> list<elem>;
}

world wizer-test {
    export init;
    export run;
}
    ",
    world: "wizer-test",
});

pub struct C;
export!(C);

use std::sync::Mutex;

static INITS: Mutex<Vec<Elem>> = Mutex::new(Vec::new());

impl exports::local::local::init::Guest for C {
    fn add_string(s: String) -> Vec<Elem> {
        let mut inits = INITS.lock().unwrap();
        inits.push(Elem::Str(s));
        inits.clone()
    }
    fn add_int(i: i32) -> Vec<Elem> {
        let mut inits = INITS.lock().unwrap();
        inits.push(Elem::Int(i));
        inits.clone()
    }
}

use exports::local::local::run::Elem;
impl exports::local::local::run::Guest for C {
    fn get_inits() -> Vec<Elem> {
        INITS.lock().unwrap().clone()
    }
}
