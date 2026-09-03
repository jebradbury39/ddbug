use parser::{Function, Unit};

use crate::index::Id;
use crate::print::{self, DiffState, PrintHeader, PrintState};

pub(crate) fn print_id(id: Id, detail: Option<&str>, state: &mut PrintState) -> Option<()> {
    let file = state.hash().file;
    match id {
        Id::Unit { unit_index } => {
            let unit = file.units().get(unit_index)?;
            print::unit::print_body(unit, state).ok()
        }
        Id::Type {
            unit_index,
            type_index,
        } => {
            let unit = file.units().get(unit_index)?;
            let ty = unit.types().get(type_index)?;
            let kind = print::types::kind(ty).ok()?;
            kind.print_body(state, unit).ok()
        }
        Id::Function {
            unit_index,
            function_index,
        } => {
            let unit = file.units().get(unit_index)?;
            let function = unit.functions().get(function_index)?;
            print_function_id(state, unit, function, detail)
        }
        Id::Variable {
            unit_index,
            variable_index,
        } => {
            let unit = file.units().get(unit_index)?;
            let variable = unit.variables().get(variable_index)?;
            variable.print_body(state, unit).ok()
        }
        _ => None,
    }
}

pub(crate) fn diff_id(id: (Id, Id), detail: Option<&str>, state: &mut DiffState) -> Option<()> {
    let file_a = state.hash_a().file;
    let file_b = state.hash_b().file;
    match id {
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
            print::unit::diff_body(state, unit_a, unit_b).ok()
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
            print::types::diff_body(state, unit_a, type_a, unit_b, type_b).ok()
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
            diff_function_id(state, unit_a, unit_b, function_a, function_b, detail)
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
            PrintHeader::diff_body(state, unit_a, variable_a, unit_b, variable_b).ok()
        }
        (id, Id::None) => print_id(id, detail, &mut state.a()),
        (Id::None, id) => print_id(id, detail, &mut state.b()),
        _ => None,
    }
}

fn print_function_id(
    state: &mut PrintState,
    unit: &Unit,
    function: &Function,
    detail: Option<&str>,
) -> Option<()> {
    match detail {
        None => function.print_body(state, unit).ok(),
        Some("code") => {
            let details = function.details(state.hash());
            print::function::print_instructions(state, function, &details).ok()
        }
        _ => None,
    }
}

fn diff_function_id(
    state: &mut DiffState,
    unit_a: &Unit,
    unit_b: &Unit,
    function_a: &Function,
    function_b: &Function,
    detail: Option<&str>,
) -> Option<()> {
    match detail {
        None => PrintHeader::diff_body(state, unit_a, function_a, unit_b, function_b).ok(),
        Some("code") => {
            let details_a = function_a.details(state.hash_a());
            let details_b = function_b.details(state.hash_b());
            print::function::diff_instructions(
                state, function_a, &details_a, function_b, &details_b,
            )
            .ok()
        }
        _ => None,
    }
}
