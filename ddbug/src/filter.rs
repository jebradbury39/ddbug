use std::cmp;
use std::collections::HashSet;

use parser::{
    BaseType, EnumerationType, File, FileHash, Function, StructType, Type, TypeDef, TypeKind,
    TypeOffset, UnionType, Unit, UnspecifiedType, Variable,
};

use crate::Options;

pub(crate) fn enumerate_index_units<'input, 'file>(
    file: &'file File<'input>,
    options: &Options,
) -> Vec<(usize, &'file Unit<'input>)> {
    file.units()
        .iter()
        .enumerate()
        // TODO: enumerate all units, but lazily merge their contents
        .filter(|a| filter_unit(a.1, options))
        .collect()
}

/// Return true if this unit matches the filter options.
pub(crate) fn filter_unit(unit: &Unit, options: &Options) -> bool {
    if let Some(filter) = options.filter_unit.as_ref() {
        let (prefix, suffix) = options.prefix_map(unit.name().unwrap_or(""));
        let iter = prefix.bytes().chain(suffix.bytes());
        iter.cmp(filter.bytes()) == cmp::Ordering::Equal
    } else {
        true
    }
}

/// The offsets of types that should be printed inline.
fn inline_types(unit: &Unit, hash: &FileHash) -> HashSet<TypeOffset> {
    let mut inline_types = HashSet::new();
    for ty in unit.types() {
        // Assume all anonymous types are inline. We don't actually check
        // that they will be inline, but in future we could (eg for TypeDefs).
        // TODO: is this a valid assumption?
        if ty.is_anon() && ty.offset().is_some() {
            inline_types.insert(ty.offset());
        }

        // Find all inline members.
        for t in ty.members() {
            if t.is_inline(hash) && t.type_offset().is_some() {
                inline_types.insert(t.type_offset());
            }
        }
    }
    inline_types
}

pub(crate) fn enumerate_index_types<'input, 'unit>(
    unit: &'unit Unit<'input>,
    hash: &FileHash,
    diff: bool,
) -> Vec<(usize, &'unit Type<'input>)> {
    let inline_types = inline_types(unit, hash);
    unit.types()
        .iter()
        .enumerate()
        .filter(|a| index_type(a.1, diff, &inline_types))
        .collect()
}

pub(crate) fn enumerate_index_functions<'input, 'unit>(
    unit: &'unit Unit<'input>,
) -> Vec<(usize, &'unit Function<'input>)> {
    unit.functions()
        .iter()
        .enumerate()
        .filter(|a| index_function(a.1))
        .collect()
}

pub(crate) fn enumerate_index_variables<'input, 'unit>(
    unit: &'unit Unit<'input>,
) -> Vec<(usize, &'unit Variable<'input>)> {
    unit.variables()
        .iter()
        .enumerate()
        .filter(|a| index_variable(a.1))
        .collect()
}

/// Return true if this function can be printed in a list.
fn index_function(f: &Function) -> bool {
    if !f.is_inline() && (f.address().is_none() || f.size().is_none()) {
        // This is either a declaration or a dead function that was removed
        // from the code, but wasn't removed from the debuginfo.
        // TODO: make this configurable?
        return false;
    }
    true
}

/// Return true if this function matches the filter options.
pub(crate) fn filter_function(f: &Function, options: &Options) -> bool {
    options.filter_name(f.name())
        && options.filter_namespace(f.namespace())
        && options.filter_function_inline(f.is_inline())
}

/// Return true if this variable can be printed in a list.
fn index_variable(v: &Variable) -> bool {
    if !v.is_declaration() && v.address().is_none() {
        // TODO: make this configurable?
        return false;
    }
    true
}

/// Return true if this variable matches the filter options.
pub(crate) fn filter_variable(v: &Variable, options: &Options) -> bool {
    options.filter_name(v.name()) && options.filter_namespace(v.namespace())
}

/// Return true if this type matches the filter options.
pub(crate) fn filter_type(ty: &Type, options: &Options) -> bool {
    match ty.kind() {
        TypeKind::Base(val) => filter_base(val, options),
        TypeKind::Def(val) => filter_type_def(val, options),
        TypeKind::Struct(val) => filter_struct(val, options),
        TypeKind::Union(val) => filter_union(val, options),
        TypeKind::Enumeration(val) => filter_enumeration(val, options),
        TypeKind::Unspecified(val) => filter_unspecified(val, options),
        TypeKind::Void
        | TypeKind::Array(..)
        | TypeKind::Function(..)
        | TypeKind::PointerToMember(..)
        | TypeKind::Modifier(..)
        | TypeKind::Subrange(..) => options.filter_name.is_none(),
    }
}

/// Return true if this type can be printed in a list.
///
/// Excludes rust closures for diff mode.
fn index_type(ty: &Type, diff: bool, inline_types: &HashSet<TypeOffset>) -> bool {
    match ty.kind() {
        TypeKind::Struct(val) => {
            // Hack for rust closures
            // TODO: is there better way of identifying these, or a
            // a way to match pairs for diffing?
            if diff && val.name() == Some("closure") {
                return false;
            }
        }
        TypeKind::Base(..)
        | TypeKind::Def(..)
        | TypeKind::Union(..)
        | TypeKind::Enumeration(..) => {}
        TypeKind::Void
        | TypeKind::Array(..)
        | TypeKind::Function(..)
        | TypeKind::Unspecified(..)
        | TypeKind::PointerToMember(..)
        | TypeKind::Modifier(..)
        | TypeKind::Subrange(..) => return false,
    }
    // Filter out inline types.
    ty.offset().is_some() && !inline_types.contains(&ty.offset())
}

fn filter_base(ty: &BaseType, options: &Options) -> bool {
    options.filter_name(ty.name()) && options.filter_namespace(None)
}

fn filter_type_def(ty: &TypeDef, options: &Options) -> bool {
    options.filter_name(ty.name()) && options.filter_namespace(ty.namespace())
}

fn filter_struct(ty: &StructType, options: &Options) -> bool {
    options.filter_name(ty.name()) && options.filter_namespace(ty.namespace())
}

fn filter_union(ty: &UnionType, options: &Options) -> bool {
    options.filter_name(ty.name()) && options.filter_namespace(ty.namespace())
}

fn filter_enumeration(ty: &EnumerationType, options: &Options) -> bool {
    options.filter_name(ty.name()) && options.filter_namespace(ty.namespace())
}

fn filter_unspecified(ty: &UnspecifiedType, options: &Options) -> bool {
    options.filter_name(ty.name()) && options.filter_namespace(ty.namespace())
}
