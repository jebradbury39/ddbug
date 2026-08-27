use std::collections::HashMap;

use parser::{File, FileHash};

use crate::code::Code;
use crate::print::{self, Id, PrintState, Printer, file};
use crate::{Options, Result};

pub struct BloatIndex {
    ids: Vec<Id>,
    function_totals: Vec<(FunctionId, FunctionTotal)>,
    callers: HashMap<u64, Vec<Caller>>,
}

// Treat functions as copies if they have the same name and source location.
// This isn't exact, but good enough.
#[derive(Clone, PartialEq, Eq, Hash)]
struct FunctionId {
    name: Vec<u8>,
    source: Vec<u8>,
}

#[derive(Default)]
struct FunctionTotal {
    size: u64,
    functions: Vec<(usize, usize)>,
}

struct Caller {
    unit_index: usize,
    function_index: usize,
    _address: u64,
}

impl BloatIndex {
    pub fn new(file: &File, options: &Options) -> BloatIndex {
        let ids = file::assign_ids(file, options);
        let code = Code::new(file);

        // Build a list of copies of functions.
        let mut function_totals = HashMap::new();
        // Also build a map of callers.
        let mut callers = HashMap::new();
        for (unit_index, unit) in file.units().iter().enumerate() {
            for (function_index, function) in unit.functions().iter().enumerate() {
                if let Some(size) = function.size() {
                    let mut name = Vec::new();
                    print::function::print_ref(function, &mut name).unwrap();
                    let mut source = Vec::new();
                    print::source::print(function.source(), &mut source, unit).unwrap();
                    let id = FunctionId { name, source };
                    let function_total = function_totals
                        .entry(id)
                        .or_insert(FunctionTotal::default());
                    function_total.size += size;
                    function_total.functions.push((unit_index, function_index));

                    if let Some(code) = code.as_ref() {
                        for range in function.ranges() {
                            for call in code.calls(*range) {
                                let entry = callers.entry(call.to).or_insert(Vec::new());
                                entry.push(Caller {
                                    unit_index,
                                    function_index,
                                    _address: call.from,
                                });
                            }
                        }
                    }
                }
            }
        }

        let mut function_totals: Vec<_> = function_totals.into_iter().collect();
        function_totals.sort_by(|(id_a, total_a), (id_b, total_b)| {
            (total_b.size.cmp(&total_a.size))
                .then_with(|| id_a.name.cmp(&id_b.name))
                .then_with(|| id_a.source.cmp(&id_b.source))
        });

        BloatIndex {
            ids,
            function_totals,
            callers,
        }
    }
}

pub fn bloat(
    file: &File,
    printer: &mut dyn Printer,
    options: &Options,
    index: &BloatIndex,
) -> Result<()> {
    let hash = FileHash::new(file);

    let state = &mut PrintState::new(printer, &hash, None, options);
    for (id, function_total) in &index.function_totals {
        state.collapsed(
            |state| {
                state.line(|w, _hash| {
                    write!(w, "{} ", function_total.size)?;
                    w.write_all(&id.name)?;
                    if !id.source.is_empty() {
                        write!(w, " ")?;
                        w.write_all(&id.source)?;
                    }
                    Ok(())
                })
            },
            |state| {
                for (unit_index, function_index) in &function_total.functions {
                    let unit = &file.units()[*unit_index];
                    let function = &unit.functions()[*function_index];
                    let address = function.address().unwrap();
                    let size = function.size().unwrap();
                    state.id(
                        function.id(),
                        |state| {
                            state.line(|w, _hash| {
                                write!(w, "{} ", size)?;
                                print::unit::print_ref(unit, w)?;
                                Ok(())
                            })
                        },
                        |state| bloat_callers(state, file, index, address),
                    )?;
                }
                Ok(())
            },
        )?;
    }

    Ok(())
}

fn bloat_callers(
    state: &mut PrintState,
    file: &File,
    index: &BloatIndex,
    address: u64,
) -> Result<()> {
    if let Some(calls) = index.callers.get(&address) {
        for caller in calls {
            state.line(|w, _hash| {
                // TODO: print inlined functions too
                let unit = &file.units()[caller.unit_index];
                let function = &unit.functions()[caller.function_index];
                print::function::print_ref(function, w)?;
                // TODO: print source of from_address instead
                let mut source = Vec::new();
                print::source::print(function.source(), &mut source, unit)?;
                if !source.is_empty() {
                    write!(w, " ")?;
                    w.write_all(&source)?;
                }
                Ok(())
            })?;
        }
    }
    Ok(())
}

pub fn bloat_id(
    id: usize,
    file: &File,
    printer: &mut dyn Printer,
    options: &Options,
    index: &BloatIndex,
) -> Option<()> {
    let id = index.ids.get(id)?;
    match *id {
        Id::Function {
            unit_index,
            function_index,
        } => {
            let unit = file.units().get(unit_index)?;
            let function = unit.functions().get(function_index)?;
            let hash = FileHash::new(file);
            let code = Code::new(file);
            let mut state = PrintState::new(printer, &hash, code.as_ref(), options);
            bloat_callers(&mut state, file, index, function.address().unwrap()).ok()
        }
        _ => None,
    }
}
