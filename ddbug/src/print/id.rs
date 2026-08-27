use parser::{File, FileHash};

use crate::Options;
use crate::code::Code;
use crate::index::{DiffIndex, Id, PrintIndex};
use crate::print::{self, DiffState, PrintHeader, PrintState, Printer};

pub fn print_id(
    id: usize,
    detail: Option<&str>,
    file: &File,
    printer: &mut dyn Printer,
    options: &Options,
    index: &PrintIndex,
) -> Option<()> {
    match index.get(id)? {
        Id::Unit { unit_index } => {
            let unit = file.units().get(unit_index)?;
            let hash = FileHash::new(file);
            let code = Code::new(file);
            let mut state = PrintState::new(printer, &hash, code.as_ref(), options);
            print::unit::print_body(unit, &mut state).ok()
        }
        Id::Type {
            unit_index,
            type_index,
        } => {
            let unit = file.units().get(unit_index)?;
            let ty = unit.types().get(type_index)?;
            let hash = FileHash::new(file);
            let code = Code::new(file);
            let mut state = PrintState::new(printer, &hash, code.as_ref(), options);
            let kind = print::types::kind(ty).ok()?;
            kind.print_body(&mut state, unit).ok()
        }
        Id::Function {
            unit_index,
            function_index,
        } => {
            let unit = file.units().get(unit_index)?;
            let function = unit.functions().get(function_index)?;
            let hash = FileHash::new(file);
            let code = Code::new(file);
            let mut state = PrintState::new(printer, &hash, code.as_ref(), options);
            match detail {
                None => function.print_body(&mut state, unit).ok(),
                Some("code") => {
                    let details = function.details(state.hash());
                    print::function::print_instructions(&mut state, function, &details).ok()
                }
                _ => None,
            }
        }
        Id::Variable {
            unit_index,
            variable_index,
        } => {
            let unit = file.units().get(unit_index)?;
            let variable = unit.variables().get(variable_index)?;
            let hash = FileHash::new(file);
            let code = Code::new(file);
            let mut state = PrintState::new(printer, &hash, code.as_ref(), options);
            variable.print_body(&mut state, unit).ok()
        }
        _ => None,
    }
}

pub fn diff_id(
    id: usize,
    file_a: &File,
    file_b: &File,
    printer: &mut dyn Printer,
    options: &Options,
    index: &DiffIndex,
) -> Option<()> {
    match index.get(id)? {
        (
            Id::Unit {
                unit_index: unit_index_a,
            },
            Id::Unit {
                unit_index: unit_index_b,
            },
        ) => {
            let unit_a = file_a.units().get(unit_index_a)?;
            let unit_b = file_b.units().get(unit_index_b)?;
            let hash_a = FileHash::new(file_a);
            let hash_b = FileHash::new(file_b);
            let code_a = Code::new(file_a);
            let code_b = Code::new(file_b);
            let mut state = DiffState::new(
                printer,
                &hash_a,
                &hash_b,
                code_a.as_ref(),
                code_b.as_ref(),
                options,
            );
            print::unit::diff_body(&mut state, unit_a, unit_b).ok()
        }
        (Id::Unit { unit_index }, Id::None) => {
            let unit = file_a.units().get(unit_index)?;
            let hash = FileHash::new(file_a);
            let code = Code::new(file_a);
            let mut state = PrintState::new(printer, &hash, code.as_ref(), options);
            print::unit::print_body(unit, &mut state).ok()
        }
        (Id::None, Id::Unit { unit_index }) => {
            let unit = file_b.units().get(unit_index)?;
            let hash = FileHash::new(file_b);
            let code = Code::new(file_b);
            let mut state = PrintState::new(printer, &hash, code.as_ref(), options);
            print::unit::print_body(unit, &mut state).ok()
        }
        (
            Id::Type {
                unit_index: unit_index_a,
                type_index: type_index_a,
            },
            Id::Type {
                unit_index: unit_index_b,
                type_index: type_index_b,
            },
        ) => {
            let unit_a = file_a.units().get(unit_index_a)?;
            let unit_b = file_b.units().get(unit_index_b)?;
            let type_a = unit_a.types().get(type_index_a)?;
            let type_b = unit_b.types().get(type_index_b)?;
            let hash_a = FileHash::new(file_a);
            let hash_b = FileHash::new(file_b);
            let code_a = Code::new(file_a);
            let code_b = Code::new(file_b);
            let mut state = DiffState::new(
                printer,
                &hash_a,
                &hash_b,
                code_a.as_ref(),
                code_b.as_ref(),
                options,
            );
            print::types::diff_body(&mut state, unit_a, type_a, unit_b, type_b).ok()
        }
        (
            Id::Type {
                unit_index,
                type_index,
            },
            Id::None,
        ) => {
            let unit = file_a.units().get(unit_index)?;
            let ty = unit.types().get(type_index)?;
            let hash = FileHash::new(file_a);
            let code = Code::new(file_a);
            let mut state = PrintState::new(printer, &hash, code.as_ref(), options);
            print::types::kind(ty)
                .and_then(|kind| kind.print_body(&mut state, unit))
                .ok()
        }
        (
            Id::None,
            Id::Type {
                unit_index,
                type_index,
            },
        ) => {
            let unit = file_b.units().get(unit_index)?;
            let ty = unit.types().get(type_index)?;
            let hash = FileHash::new(file_b);
            let code = Code::new(file_b);
            let mut state = PrintState::new(printer, &hash, code.as_ref(), options);
            print::types::kind(ty)
                .and_then(|kind| kind.print_body(&mut state, unit))
                .ok()
        }
        (
            Id::Function {
                unit_index: unit_index_a,
                function_index: function_index_a,
            },
            Id::Function {
                unit_index: unit_index_b,
                function_index: function_index_b,
            },
        ) => {
            let unit_a = file_a.units().get(unit_index_a)?;
            let unit_b = file_b.units().get(unit_index_b)?;
            let function_a = unit_a.functions().get(function_index_a)?;
            let function_b = unit_b.functions().get(function_index_b)?;
            let hash_a = FileHash::new(file_a);
            let hash_b = FileHash::new(file_b);
            let code_a = Code::new(file_a);
            let code_b = Code::new(file_b);
            let mut state = DiffState::new(
                printer,
                &hash_a,
                &hash_b,
                code_a.as_ref(),
                code_b.as_ref(),
                options,
            );
            PrintHeader::diff_body(&mut state, unit_a, function_a, unit_b, function_b).ok()
        }
        (
            Id::Function {
                unit_index,
                function_index,
            },
            Id::None,
        ) => {
            let unit = file_a.units().get(unit_index)?;
            let function = unit.functions().get(function_index)?;
            let hash = FileHash::new(file_a);
            let code = Code::new(file_a);
            let mut state = PrintState::new(printer, &hash, code.as_ref(), options);
            function.print_body(&mut state, unit).ok()
        }
        (
            Id::None,
            Id::Function {
                unit_index,
                function_index,
            },
        ) => {
            let unit = file_b.units().get(unit_index)?;
            let function = unit.functions().get(function_index)?;
            let hash = FileHash::new(file_b);
            let code = Code::new(file_b);
            let mut state = PrintState::new(printer, &hash, code.as_ref(), options);
            function.print_body(&mut state, unit).ok()
        }
        (
            Id::Variable {
                unit_index: unit_index_a,
                variable_index: variable_index_a,
            },
            Id::Variable {
                unit_index: unit_index_b,
                variable_index: variable_index_b,
            },
        ) => {
            let unit_a = file_a.units().get(unit_index_a)?;
            let unit_b = file_b.units().get(unit_index_b)?;
            let variable_a = unit_a.variables().get(variable_index_a)?;
            let variable_b = unit_b.variables().get(variable_index_b)?;
            let hash_a = FileHash::new(file_a);
            let hash_b = FileHash::new(file_b);
            let code_a = Code::new(file_a);
            let code_b = Code::new(file_b);
            let mut state = DiffState::new(
                printer,
                &hash_a,
                &hash_b,
                code_a.as_ref(),
                code_b.as_ref(),
                options,
            );
            PrintHeader::diff_body(&mut state, unit_a, variable_a, unit_b, variable_b).ok()
        }
        (
            Id::Variable {
                unit_index,
                variable_index,
            },
            Id::None,
        ) => {
            let unit = file_a.units().get(unit_index)?;
            let variable = unit.variables().get(variable_index)?;
            let hash = FileHash::new(file_a);
            let code = Code::new(file_a);
            let mut state = PrintState::new(printer, &hash, code.as_ref(), options);
            variable.print_body(&mut state, unit).ok()
        }
        (
            Id::None,
            Id::Variable {
                unit_index,
                variable_index,
            },
        ) => {
            let unit = file_b.units().get(unit_index)?;
            let variable = unit.variables().get(variable_index)?;
            let hash = FileHash::new(file_b);
            let code = Code::new(file_b);
            let mut state = PrintState::new(printer, &hash, code.as_ref(), options);
            variable.print_body(&mut state, unit).ok()
        }
        _ => None,
    }
}
