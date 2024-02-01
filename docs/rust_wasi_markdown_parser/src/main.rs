// Import our CLI parsing libraries (And PathBuf for reading paths)
extern crate structopt;

use structopt::StructOpt;
use std::path::PathBuf;

// Import our markdown parser library, crate
extern crate pulldown_cmark;

use pulldown_cmark::{html, Parser};

// Import from the standard library, to allow reading from the file system
use std::fs;

// Define our CLI options using structopt
#[derive(StructOpt)]
#[structopt(name = "rust_wasi_markdown_parser", about = "Markdown to HTML renderer CLI, written with Rust & WASI")]
pub struct Options {
    /// The markdown file to render
    #[structopt(parse(from_os_str))]
    filename: PathBuf,
}

// Our entrypoint into our WASI module
fn main() {

    // Get the passed CLI options
    let options = Options::from_args();

    // Read the markdown file into a string
    let contents = fs::read_to_string(options.filename)
        .expect("Something went wrong reading the file");

    // Run our parsing function to get back an HTML string
    let result = render_markdown(contents);

    // Print out the resulting HTML to standard out
    println!("{}", result);
}

pub fn render_markdown(markdown: String) -> String {
    let mut html_buf = String::new();
    let parser = Parser::new(&markdown[..]);
    html::push_html(&mut html_buf, parser);
    html_buf
}
