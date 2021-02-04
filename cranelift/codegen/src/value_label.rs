use crate::ir::{Function, SourceLoc, Value, ValueLabel, ValueLabelAssignments, ValueLoc};
use crate::isa::TargetIsa;
use crate::machinst::MachCompileResult;
use crate::regalloc::{Context, RegDiversions};
use crate::HashMap;
use alloc::collections::BTreeMap;
use alloc::vec::Vec;
use core::cmp::Ordering;
use core::convert::From;
use core::iter::Iterator;
use core::ops::Bound::*;
use core::ops::Deref;
use regalloc::Reg;

#[cfg(feature = "enable-serde")]
use serde::{Deserialize, Serialize};

/// Value location range.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(feature = "enable-serde", derive(Serialize, Deserialize))]
pub struct ValueLocRange {
    /// The ValueLoc containing a ValueLabel during this range.
    pub loc: LabelValueLoc,
    /// The start of the range. It is an offset in the generated code.
    pub start: u32,
    /// The end of the range. It is an offset in the generated code.
    pub end: u32,
}

/// The particular location for a value.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(feature = "enable-serde", derive(Serialize, Deserialize))]
pub enum LabelValueLoc {
    /// Old-backend location: RegUnit, StackSlot, or Unassigned.
    ValueLoc(ValueLoc),
    /// New-backend Reg.
    Reg(Reg),
    /// New-backend offset from stack pointer.
    SPOffset(i64),
}

impl From<ValueLoc> for LabelValueLoc {
    fn from(v: ValueLoc) -> Self {
        LabelValueLoc::ValueLoc(v)
    }
}

/// Resulting map of Value labels and their ranges/locations.
pub type ValueLabelsRanges = HashMap<ValueLabel, Vec<ValueLocRange>>;

fn build_value_labels_index<T>(func: &Function) -> BTreeMap<T, (Value, ValueLabel)>
where
    T: From<SourceLoc> + Deref<Target = SourceLoc> + Ord + Copy,
{
    if func.dfg.values_labels.is_none() {
        return BTreeMap::new();
    }
    let values_labels = func.dfg.values_labels.as_ref().unwrap();

    // Index values_labels by srcloc/from
    let mut sorted = BTreeMap::new();
    for (val, assigns) in values_labels {
        match assigns {
            ValueLabelAssignments::Starts(labels) => {
                for label in labels {
                    if label.from.is_default() {
                        continue;
                    }
                    let srcloc = T::from(label.from);
                    let label = label.label;
                    sorted.insert(srcloc, (*val, label));
                }
            }
            ValueLabelAssignments::Alias { from, value } => {
                if from.is_default() {
                    continue;
                }
                let mut aliased_value = *value;
                while let Some(ValueLabelAssignments::Alias { value, .. }) =
                    values_labels.get(&aliased_value)
                {
                    // TODO check/limit recursion?
                    aliased_value = *value;
                }
                let from = T::from(*from);
                if let Some(ValueLabelAssignments::Starts(labels)) =
                    values_labels.get(&aliased_value)
                {
                    for label in labels {
                        let srcloc = if label.from.is_default() {
                            from
                        } else {
                            from.max(T::from(label.from))
                        };
                        let label = label.label;
                        sorted.insert(srcloc, (*val, label));
                    }
                }
            }
        }
    }
    sorted
}

/// Builds ranges and location for specified value labels.
/// The labels specified at DataFlowGraph's values_labels collection.
pub fn build_value_labels_ranges<T>(
    func: &Function,
    regalloc: &Context,
    mach_compile_result: Option<&MachCompileResult>,
    isa: &dyn TargetIsa,
) -> ValueLabelsRanges
where
    T: From<SourceLoc> + Deref<Target = SourceLoc> + Ord + Copy,
{
    if let Some(mach_compile_result) = mach_compile_result {
        return mach_compile_result.value_labels_ranges.clone();
    }

    let values_labels = build_value_labels_index::<T>(func);

    let mut blocks = func.layout.blocks().collect::<Vec<_>>();
    blocks.sort_by_key(|block| func.offsets[*block]); // Ensure inst offsets always increase
    let encinfo = isa.encoding_info();
    let values_locations = &func.locations;
    let liveness_ranges = regalloc.liveness().ranges();

    let mut ranges = HashMap::new();
    let mut add_range = |label, range: (u32, u32), loc: ValueLoc| {
        if range.0 >= range.1 || !loc.is_assigned() {
            return;
        }
        ranges
            .entry(label)
            .or_insert_with(Vec::new)
            .push(ValueLocRange {
                loc: loc.into(),
                start: range.0,
                end: range.1,
            });
    };

    let mut end_offset = 0;
    let mut tracked_values: Vec<(Value, ValueLabel, u32, ValueLoc)> = Vec::new();
    let mut divert = RegDiversions::new();
    for block in blocks {
        divert.at_block(&func.entry_diversions, block);
        let mut last_srcloc: Option<T> = None;
        for (offset, inst, size) in func.inst_offsets(block, &encinfo) {
            divert.apply(&func.dfg[inst]);
            end_offset = offset + size;
            // Remove killed values.
            tracked_values.retain(|(x, label, start_offset, last_loc)| {
                let range = liveness_ranges.get(*x);
                if range.expect("value").killed_at(inst, block, &func.layout) {
                    add_range(*label, (*start_offset, end_offset), *last_loc);
                    return false;
                }
                true
            });

            let srcloc = func.srclocs[inst];
            if srcloc.is_default() {
                // Don't process instructions without srcloc.
                continue;
            }
            let srcloc = T::from(srcloc);

            // Record and restart ranges if Value location was changed.
            for (val, label, start_offset, last_loc) in &mut tracked_values {
                let new_loc = divert.get(*val, values_locations);
                if new_loc == *last_loc {
                    continue;
                }
                add_range(*label, (*start_offset, end_offset), *last_loc);
                *start_offset = end_offset;
                *last_loc = new_loc;
            }

            // New source locations range started: abandon all tracked values.
            if last_srcloc.is_some() && last_srcloc.unwrap() > srcloc {
                for (_, label, start_offset, last_loc) in &tracked_values {
                    add_range(*label, (*start_offset, end_offset), *last_loc);
                }
                tracked_values.clear();
                last_srcloc = None;
            }

            // Get non-processed Values based on srcloc
            let range = (
                match last_srcloc {
                    Some(a) => Excluded(a),
                    None => Unbounded,
                },
                Included(srcloc),
            );
            let active_values = values_labels.range(range);
            let active_values = active_values.filter(|(_, (v, _))| {
                // Ignore dead/inactive Values.
                let range = liveness_ranges.get(*v);
                match range {
                    Some(r) => r.reaches_use(inst, block, &func.layout),
                    None => false,
                }
            });
            // Append new Values to the tracked_values.
            for (_, (val, label)) in active_values {
                let loc = divert.get(*val, values_locations);
                tracked_values.push((*val, *label, end_offset, loc));
            }

            last_srcloc = Some(srcloc);
        }
        // Finish all started ranges.
        for (_, label, start_offset, last_loc) in &tracked_values {
            add_range(*label, (*start_offset, end_offset), *last_loc);
        }
    }

    // Optimize ranges in-place
    for (_, label_ranges) in ranges.iter_mut() {
        assert!(!label_ranges.is_empty());
        label_ranges.sort_by(|a, b| a.start.cmp(&b.start).then_with(|| a.end.cmp(&b.end)));

        // Merge ranges
        let mut i = 1;
        let mut j = 0;
        while i < label_ranges.len() {
            assert!(label_ranges[j].start <= label_ranges[i].end);
            if label_ranges[j].loc != label_ranges[i].loc {
                // Different location
                if label_ranges[j].end >= label_ranges[i].end {
                    // Consumed by previous range, skipping
                    i += 1;
                    continue;
                }
                j += 1;
                label_ranges[j] = label_ranges[i];
                i += 1;
                continue;
            }
            if label_ranges[j].end < label_ranges[i].start {
                // Gap in the range location
                j += 1;
                label_ranges[j] = label_ranges[i];
                i += 1;
                continue;
            }
            // Merge i-th and j-th ranges
            if label_ranges[j].end < label_ranges[i].end {
                label_ranges[j].end = label_ranges[i].end;
            }
            i += 1;
        }
        label_ranges.truncate(j + 1);

        // Cut/move start position of next range, if two neighbor ranges intersect.
        for i in 0..j {
            if label_ranges[i].end > label_ranges[i + 1].start {
                label_ranges[i + 1].start = label_ranges[i].end;
                assert!(label_ranges[i + 1].start < label_ranges[i + 1].end);
            }
            assert!(label_ranges[i].end <= label_ranges[i + 1].start);
        }
    }
    ranges
}

#[derive(Eq, Clone, Copy)]
pub struct ComparableSourceLoc(SourceLoc);

impl From<SourceLoc> for ComparableSourceLoc {
    fn from(s: SourceLoc) -> Self {
        Self(s)
    }
}

impl Deref for ComparableSourceLoc {
    type Target = SourceLoc;
    fn deref(&self) -> &SourceLoc {
        &self.0
    }
}

impl PartialOrd for ComparableSourceLoc {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for ComparableSourceLoc {
    fn cmp(&self, other: &Self) -> Ordering {
        self.0.bits().cmp(&other.0.bits())
    }
}

impl PartialEq for ComparableSourceLoc {
    fn eq(&self, other: &Self) -> bool {
        self.0 == other.0
    }
}
