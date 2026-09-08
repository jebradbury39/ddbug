use std::cmp;

use parser::FileHash;

use crate::{Options, Result};

mod text;
pub use text::TextPrinter;

mod html;
pub use html::HtmlPrinter;

mod printer;
pub use printer::{DiffPrefix, Printer, ValuePrinter};

mod state;
pub(crate) use state::{DiffState, PrintState};

pub(crate) mod base_type;
pub(crate) mod bloat;
pub(crate) mod enumeration;
pub(crate) mod file;
pub(crate) mod function;
pub(crate) mod function_call;
pub(crate) mod id;
pub(crate) mod inherit;
pub(crate) mod inlined_function;
pub(crate) mod local_variable;
pub(crate) mod location;
pub(crate) mod member;
pub(crate) mod namespace;
pub(crate) mod parameter;
pub(crate) mod range;
pub(crate) mod register;
pub(crate) mod section;
pub(crate) mod source;
pub(crate) mod struct_type;
pub(crate) mod symbol;
pub(crate) mod type_def;
pub(crate) mod types;
pub(crate) mod union_type;
pub(crate) mod unit;
pub(crate) mod variable;

pub(crate) trait Print {
    type Arg;

    // TODO: need associated type constructor to avoid requiring arg to be a reference?
    fn print(&self, state: &mut PrintState, arg: &Self::Arg) -> Result<()>;

    fn diff(
        state: &mut DiffState,
        arg_a: &Self::Arg,
        a: &Self,
        arg_b: &Self::Arg,
        b: &Self,
    ) -> Result<()>;
}

impl<'a, T> Print for &'a T
where
    T: Print,
{
    type Arg = T::Arg;

    fn print(&self, state: &mut PrintState, arg: &Self::Arg) -> Result<()> {
        T::print(*self, state, arg)
    }

    fn diff(
        state: &mut DiffState,
        arg_a: &Self::Arg,
        a: &Self,
        arg_b: &Self::Arg,
        b: &Self,
    ) -> Result<()> {
        T::diff(state, arg_a, *a, arg_b, *b)
    }
}

pub(crate) trait PrintHeader {
    fn print_header(&self, state: &mut PrintState) -> Result<()>;
    fn print_body(&self, state: &mut PrintState, unit: &parser::Unit) -> Result<()>;
    fn diff_header(state: &mut DiffState, a: &Self, b: &Self) -> Result<()>
    where
        Self: Sized;
    fn diff_body(
        state: &mut DiffState,
        unit_a: &parser::Unit,
        a: &Self,
        unit_b: &parser::Unit,
        b: &Self,
    ) -> Result<()>
    where
        Self: Sized;
}

pub(crate) trait DiffList: Print {
    fn step_cost(&self, state: &DiffState, arg: &Self::Arg) -> usize;

    fn diff_cost(
        state: &DiffState,
        arg_a: &Self::Arg,
        a: &Self,
        arg_b: &Self::Arg,
        b: &Self,
    ) -> usize;
}

pub(crate) trait SortList: Print {
    fn cmp_id(
        hash_a: &FileHash,
        a: &Self,
        hash_b: &FileHash,
        b: &Self,
        options: &Options,
    ) -> cmp::Ordering;

    fn cmp_id_for_sort(
        hash_a: &FileHash,
        a: &Self,
        hash_b: &FileHash,
        b: &Self,
        options: &Options,
    ) -> cmp::Ordering {
        Self::cmp_id(hash_a, a, hash_b, b, options)
    }

    fn cmp_by(
        hash_a: &FileHash,
        a: &Self,
        hash_b: &FileHash,
        b: &Self,
        options: &Options,
    ) -> cmp::Ordering;
}
