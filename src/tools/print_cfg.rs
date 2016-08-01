//! The `print-cfg` sub-command.
//!
//! Read a series of Cretonne IL files and print their control flow graphs
//! in graphviz format.
use std::fs::File;
use std::io::{Read, Write, stdout};

use CommandResult;
use cretonne::ir::Function;
use cretonne::cfg::ControlFlowGraph;
use cretonne::ir::instructions::InstructionData;
use cton_reader::parser::Parser;

pub fn run(files: Vec<String>) -> CommandResult {
    for (i, f) in files.into_iter().enumerate() {
        if i != 0 {
            println!("");
        }
        try!(print_cfg(f))
    }
    Ok(())
}

struct CFGPrinter<T: Write> {
    level: usize,
    writer: T,
    buffer: String,
}

impl<T: Write> CFGPrinter<T> {
    pub fn new(writer: T) -> CFGPrinter<T> {
        CFGPrinter {
            level: 0,
            writer: writer,
            buffer: String::new(),
        }
    }

    pub fn print(&mut self, func: &Function) -> Result<(), String> {
        self.level = 0;
        self.header(func);
        self.push_indent();
        self.ebb_subgraphs(func);
        let cfg = ControlFlowGraph::new(func);
        self.cfg_connections(func, &cfg);
        self.pop_indent();
        self.footer();
        self.write()
    }

    fn write(&mut self) -> Result<(), String> {
        match self.writer.write(self.buffer.as_bytes()) {
            Err(_) => return Err("Write failed!".to_string()),
            _ => (),
        };
        match self.writer.flush() {
            Err(_) => return Err("Flush failed!".to_string()),
            _ => (),
        };
        Ok(())
    }

    fn append(&mut self, s: &str) {
        let mut indent = String::new();
        for _ in 0..self.level {
            indent = indent + "    ";
        }
        self.buffer.push_str(&(indent + s));
    }

    fn push_indent(&mut self) {
        self.level += 1;
    }

    fn pop_indent(&mut self) {
        if self.level > 0 {
            self.level -= 1;
        }
    }

    fn open_paren(&mut self) {
        self.append("{");
    }

    fn close_paren(&mut self) {
        self.append("}");
    }

    fn newline(&mut self) {
        self.append("\n");
    }

    fn header(&mut self, func: &Function) {
        self.append(&format!("digraph {} ", func.name));
        self.open_paren();
        self.newline();
        self.push_indent();
        self.append("{rank=min; ebb0}");
        self.pop_indent();
        self.newline();
    }

    fn footer(&mut self) {
        self.close_paren();
        self.newline();
    }

    fn ebb_subgraphs(&mut self, func: &Function) {
        for ebb in &func.layout {
            let inst_data = func.layout
                .ebb_insts(ebb)
                .filter(|inst| {
                    match func.dfg[*inst] {
                        InstructionData::Branch { ty: _, opcode: _, data: _ } => true,
                        InstructionData::Jump { ty: _, opcode: _, data: _ } => true,
                        _ => false,
                    }
                })
                .map(|inst| {
                    let op = match func.dfg[inst] {
                        InstructionData::Branch { ty: _, opcode, ref data } => {
                            Some((opcode, data.destination))
                        }
                        InstructionData::Jump { ty: _, opcode, ref data } => {
                            Some((opcode, data.destination))
                        }
                        _ => None,
                    };
                    (inst, op)
                })
                .collect::<Vec<_>>();

            let mut insts = vec![format!("{}", ebb)];
            for (inst, data) in inst_data {
                let (op, dest) = data.unwrap();
                insts.push(format!("<{}>{} {}", inst, op, dest));
            }

            self.append(&format!("{} [shape=record, label=\"{}{}{}\"]",
                                 ebb,
                                 "{",
                                 insts.join(" | "),
                                 "}"));
            self.newline();
        }
    }

    fn cfg_connections(&mut self, func: &Function, cfg: &ControlFlowGraph) {
        for ebb in &func.layout {
            for &(parent, inst) in cfg.get_predecessors(ebb) {
                self.append(&format!("{}:{} -> {}", parent, inst, ebb));
                self.newline();
            }
        }
    }
}

fn print_cfg(filename: String) -> CommandResult {
    let mut file = try!(File::open(&filename).map_err(|e| format!("{}: {}", filename, e)));
    let mut buffer = String::new();
    try!(file.read_to_string(&mut buffer)
        .map_err(|e| format!("Couldn't read {}: {}", filename, e)));
    let items = try!(Parser::parse(&buffer).map_err(|e| format!("{}: {}", filename, e)));

    let mut cfg_printer = CFGPrinter::new(stdout());
    for (idx, func) in items.into_iter().enumerate() {
        if idx != 0 {
            println!("");
        }

        try!(cfg_printer.print(&func));
    }

    Ok(())
}
