use std::{collections::HashMap, ops::Range};

use anyhow::{Result, bail, format_err};
use cranelift_isle_veri_aslp::opcode::{self, Opcode};

#[derive(Clone)]
pub struct Bits {
    pub segments: Vec<Segment>,
}

impl Bits {
    pub fn empty() -> Self {
        Bits {
            segments: Vec::new(),
        }
    }

    pub fn from_u32(x: u32) -> Self {
        Bits {
            segments: vec![Segment::from_u32(x)],
        }
    }

    pub fn is_symbolic(&self) -> bool {
        self.segments.iter().any(|s| s.is_symbolic())
    }

    pub fn width(&self) -> usize {
        self.segments.iter().map(|s| s.width()).sum()
    }

    /// Evaluate the bitvector template with the given assignment.
    pub fn eval(&self, assignment: &HashMap<String, u32>) -> Result<u32> {
        let mut result = 0u32;
        let mut offset = 0usize;
        for segment in &self.segments {
            let value = match segment {
                Segment::Symbolic(name, _) => assignment.get(name).ok_or(format_err!(
                    "missing assignment for symbolic segment: {}",
                    name
                ))?,
                Segment::Constant(c, _) => c,
            };
            result |= value << offset;
            offset += segment.width()
        }
        Ok(result)
    }

    pub fn splice(base: &Bits, insert: &Bits, offset: usize) -> Result<Bits> {
        let mut result = Bits::empty();
        if offset > 0 {
            let prefix = base.extract(0, offset)?;
            result.append(prefix);
        }
        result.append(insert.clone());
        if result.width() < base.width() {
            let suffix = base.extract(result.width(), base.width())?;
            result.append(suffix);
        }
        Ok(result)
    }

    pub fn append(&mut self, other: Bits) {
        self.segments.extend(other.segments);
    }

    pub fn extract(&self, lo: usize, hi: usize) -> Result<Bits> {
        let mut result = Bits::empty();
        let mut offset = 0usize;
        for segment in &self.segments {
            // Intersection of this interval with extraction interval.
            let start = std::cmp::max(lo, offset);
            let end = std::cmp::min(hi, offset + segment.width());

            // If the intersection is non-empty, add a segment.
            if start < end {
                result
                    .segments
                    .push(segment.extract(start - offset, end - offset)?);
            }

            // Advance offset.
            offset += segment.width();
        }
        Ok(result)
    }
}

impl std::fmt::Display for Bits {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{}",
            self.segments
                .iter()
                .rev()
                .map(|s| s.to_string())
                .collect::<Vec<_>>()
                .join(" ")
        )
    }
}

impl From<Bits> for opcode::Opcode {
    fn from(bits: Bits) -> Self {
        Opcode {
            segments: bits.segments.into_iter().map(Into::into).collect(),
        }
    }
}

/// Concrete assignment of symbolic bits in a bitvector template.
pub struct Concrete {
    pub assignment: HashMap<String, u32>,
    pub template: Bits,
}

impl Concrete {
    pub fn eval(&self) -> Result<u32> {
        self.template.eval(&self.assignment)
    }
}

pub struct ConcreteIterator {
    fields: Vec<(String, u32)>,
    bits: Range<u32>,
    template: Bits,
}

impl ConcreteIterator {
    fn new(template: Bits) -> Self {
        let mut fields = Vec::new();
        let mut n = 1;
        for segment in &template.segments {
            match segment {
                Segment::Symbolic(name, width) => {
                    fields.push((name.clone(), (*width).try_into().unwrap()));
                    n <<= width;
                }
                Segment::Constant(_, _) => {}
            }
        }

        ConcreteIterator {
            fields,
            bits: 0..n,
            template,
        }
    }
}

impl Iterator for ConcreteIterator {
    type Item = Concrete;

    fn next(&mut self) -> Option<Self::Item> {
        // Advance to next assignment of all symbolic bits.
        let mut bits = self.bits.next()?;

        // Divide into individual fields.
        let mut assignment = HashMap::new();
        for (name, width) in &self.fields {
            let mask = (1 << width) - 1;
            assignment.insert(name.clone(), bits & mask);
            bits >>= width;
        }

        Some(Concrete {
            assignment,
            template: self.template.clone(),
        })
    }
}

impl IntoIterator for &Bits {
    type Item = Concrete;
    type IntoIter = ConcreteIterator;

    fn into_iter(self) -> Self::IntoIter {
        ConcreteIterator::new(self.clone())
    }
}

#[derive(Clone)]
pub enum Segment {
    Symbolic(String, usize),
    Constant(u32, usize),
}

impl Segment {
    pub fn from_u32(x: u32) -> Self {
        Segment::Constant(x, 32)
    }

    pub fn is_symbolic(&self) -> bool {
        matches!(self, Segment::Symbolic(_, _))
    }

    pub fn width(&self) -> usize {
        match self {
            Segment::Symbolic(_, w) | Segment::Constant(_, w) => *w,
        }
    }

    pub fn extract(&self, lo: usize, hi: usize) -> Result<Segment> {
        match *self {
            Segment::Symbolic(_, w) => {
                if !(lo == 0 && hi == w) {
                    bail!("symbolic segments must remain whole");
                }
                Ok(self.clone())
            }
            Segment::Constant(c, w) => {
                if !(lo < hi && hi <= w) {
                    bail!("invalid extraction interval");
                }
                let w = hi - lo;
                let mask = (1 << w) - 1;
                Ok(Segment::Constant((c >> lo) & mask, w))
            }
        }
    }
}

impl std::fmt::Display for Segment {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Segment::Symbolic(s, w) => write!(f, "{s}:{w}"),
            Segment::Constant(c, w) => write!(f, "{c:#x}:{w}"),
        }
    }
}

impl From<Segment> for opcode::Segment {
    fn from(segment: Segment) -> Self {
        match segment {
            Segment::Symbolic(name, width) => opcode::Segment::Symbolic(name, width),
            Segment::Constant(value, width) => opcode::Segment::Constant(value, width),
        }
    }
}
