// Import our markdown parser library, crate
extern crate pulldown_cmark;

use pulldown_cmark::{html, Parser};

// Import from the standard library, to allow reading CLI arguments, and from the file system
use std::env;
use std::fs;

// Our entrypoint into our WASI module
fn main() {

    // Get the passed CLI arguments
    let args: Vec<String> = env::args().collect();

    if args.len() != 2 {
        eprintln!("error: please pass a markdown file to be parsed");
        return;
    }

    // Get the markdown file name argument
    let filename = &args[1];

    // Read the markdown file into a string
    let contents = fs::read_to_string(filename)
        .expect("Something went wrong reading the file");

    // Run our parsing function to get back an HTML string
    let result = format(contents);

    // Print out the resulting HTML to standard out
    println!("{}", result);
}

// Create a function for converting a markdown string, into an HTML string
pub fn format(markdown: String) -> String {
    let mut html_buf = String::new();
    let parser = Parser::new(&markdown[..]);
    html::push_html(&mut html_buf, parser);
    html_buf
}
