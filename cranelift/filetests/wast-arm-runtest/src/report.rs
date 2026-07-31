#[derive(Default)]
pub struct Report {
    passed: u32,
    skipped: u32,
    failed: u32,
}

impl Report {
    pub fn print_and_exit(&self) {
        self.print();
        if self.failed > 0 {
            std::process::exit(1);
        }
    }

    pub fn print(&self) {
        println!(
            "{} passed, {} failed, {} skipped",
            self.passed, self.failed, self.skipped
        );
    }

    pub fn add_passed(&mut self, n: u32) {
        self.passed += n;
    }

    pub fn add_skipped(&mut self, n: u32) {
        self.skipped += n;
    }

    pub fn add_failed(&mut self, n: u32) {
        self.failed += n;
    }
}
