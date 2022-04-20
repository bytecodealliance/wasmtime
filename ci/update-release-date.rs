fn main() {
    let date = std::env::args().nth(1).unwrap();
    let relnotes = std::fs::read_to_string("RELEASES.md").unwrap();
    let mut new_relnotes = String::new();
    let mut counter = 0;
    for line in relnotes.lines() {
        if line.starts_with("Unreleased") {
            counter += 1;
            if counter == 2 {
                new_relnotes.push_str(&format!("Released {}\n", date));
                continue;
            }
        }
        new_relnotes.push_str(line);
        new_relnotes.push_str("\n");
    }
    std::fs::write("RELEASES.md", new_relnotes).unwrap();
}
