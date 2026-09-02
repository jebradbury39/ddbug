use crate::Result;
use crate::filter;
use crate::print::{DiffState, PrintState};

pub(crate) fn print(state: &mut PrintState) -> Result<()> {
    if state.options().category_file {
        state.collapsed(
            |state| {
                state.line(|w, hash| {
                    write!(w, "file {}", hash.file.path())?;
                    Ok(())
                })
            },
            |state| {
                let file = state.hash().file;
                let ranges = file.ranges(state.hash());
                let size = ranges.size();
                let fn_size = file.function_size();
                let var_size = file.variable_size(state.hash());
                let other_size = size.checked_sub(fn_size + var_size).unwrap_or_else(|| {
                    // TODO: fix our calculations so this doesn't happen
                    debug!("function or variable sizes are too large");
                    0
                });
                if state.options().print_file_address {
                    state.field_collapsed("addresses", |state| state.list(&(), ranges.list()))?;
                }
                state.field_u64("size", size)?;
                state.field_u64("fn size", fn_size)?;
                state.field_u64("var size", var_size)?;
                state.field_u64("other size", other_size)?;
                state.field_collapsed("sections", |state| state.list(&(), file.sections()))?;
                Ok(())
            },
        )?;
        state.line_break()?;
    }

    state.sort_list(
        &(),
        &mut filter::filter_units(state.hash().file, state.options()),
    )
}

pub(crate) fn diff(state: &mut DiffState) -> Result<()> {
    if state.options().category_file {
        state.collapsed(
            |state| {
                state.line(state.hash_a().file, state.hash_b().file, |w, _hash, x| {
                    write!(w, "file {}", x.path())?;
                    Ok(())
                })
            },
            |state| {
                let hash_a = state.hash_a();
                let hash_b = state.hash_b();
                let file_a = hash_a.file;
                let file_b = hash_b.file;
                let ranges_a = file_a.ranges(hash_a);
                let ranges_b = file_b.ranges(hash_b);
                let size_a = ranges_a.size();
                let size_b = ranges_b.size();
                let fn_size_a = file_a.function_size();
                let fn_size_b = file_b.function_size();
                let var_size_a = file_a.variable_size(state.hash_a());
                let var_size_b = file_b.variable_size(state.hash_b());
                let other_size_a = size_a - fn_size_a - var_size_a;
                let other_size_b = size_b - fn_size_b - var_size_b;
                if state.options().print_file_address {
                    state.field_collapsed("addresses", |state| {
                        state.ord_list(&(), ranges_a.list(), &(), ranges_b.list())
                    })?;
                }
                state.field_u64("size", size_a, size_b)?;
                state.field_u64("fn size", fn_size_a, fn_size_b)?;
                state.field_u64("var size", var_size_a, var_size_b)?;
                state.field_u64("other size", other_size_a, other_size_b)?;
                // TODO: sort sections
                state.field_collapsed("sections", |state| {
                    state.list(&(), file_a.sections(), &(), file_b.sections())
                })?;
                Ok(())
            },
        )?;
        state.line_break()?;
    }

    let mut units = state
        .index()
        .merged_units(state.hash_a().file, state.hash_b().file);
    state.sort_list(&(), &(), &mut units)
}
