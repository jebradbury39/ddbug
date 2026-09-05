use std::ops::Range;

use parser::{File, FileHash, Function, Type, Unit, Variable};

use crate::Options;
use crate::filter;
use crate::merge::{MergeIterator, MergeResult};
use crate::print::SortList;

#[derive(Debug, Clone, Copy, Eq, PartialEq)]
pub(crate) enum Id {
    None,
    Unit {
        unit_index: usize,
    },
    Type {
        unit_index: usize,
        type_index: usize,
    },
    Function {
        unit_index: usize,
        function_index: usize,
    },
    Variable {
        unit_index: usize,
        variable_index: usize,
    },
}

impl Id {
    pub fn parent(&self, file: &File) -> Option<usize> {
        match self {
            Id::Type { unit_index, .. }
            | Id::Function { unit_index, .. }
            | Id::Variable { unit_index, .. } => {
                let unit = file.units().get(*unit_index)?;
                Some(unit.id())
            }
            _ => None,
        }
    }

    fn unit_index(&self) -> Option<usize> {
        match self {
            Id::Unit { unit_index } => Some(*unit_index),
            _ => None,
        }
    }

    fn type_index(&self) -> Option<usize> {
        match self {
            Id::Type { type_index, .. } => Some(*type_index),
            _ => None,
        }
    }

    fn function_index(&self) -> Option<usize> {
        match self {
            Id::Function { function_index, .. } => Some(*function_index),
            _ => None,
        }
    }

    fn variable_index(&self) -> Option<usize> {
        match self {
            Id::Variable { variable_index, .. } => Some(*variable_index),
            _ => None,
        }
    }
}

pub(crate) struct Index {
    ids: Vec<(Id, Id)>,
    // Indexed by unit id. Currently only the merged unit entries are used.
    units: Vec<UnitIndex>,
}

#[derive(Default)]
struct UnitIndex {
    types: Range<usize>,
    functions: Range<usize>,
    variables: Range<usize>,
}

impl Index {
    pub fn new(file: &FileHash, options: &Options) -> Index {
        assign_merged_ids(file, None, options)
    }

    pub fn new_diff(file_a: &FileHash, file_b: &FileHash, options: &Options) -> Index {
        assign_merged_ids(file_a, Some(file_b), options)
    }

    pub fn get(&self, id: usize) -> Option<Id> {
        self.get_pair(id).map(|x| x.0)
    }

    pub fn get_pair(&self, id: usize) -> Option<(Id, Id)> {
        self.ids.get(id).copied()
    }

    pub fn parent(&self, id: usize, file_a: &File, file_b: Option<&File>) -> Option<usize> {
        let (id_a, id_b) = self.get_pair(id)?;
        id_a.parent(file_a).or_else(|| id_b.parent(file_b?))
    }

    pub fn units<'a, 'input>(&self, file: &'a File<'input>) -> Vec<&'a Unit<'input>> {
        single(
            Side::Left,
            self.unit_ids(),
            |a| file.units().get(a.unit_index()?),
            // TODO: enumerate all units, but lazily merge their contents
            |_| true,
        )
    }

    pub fn merged_units<'a, 'input>(
        &self,
        file_a: &'a File<'input>,
        file_b: &'a File<'input>,
    ) -> Vec<MergeResult<&'a Unit<'input>, &'a Unit<'input>>> {
        merge(
            self.unit_ids(),
            |a| file_a.units().get(a.unit_index()?),
            |b| file_b.units().get(b.unit_index()?),
            // TODO: enumerate all units, but lazily merge their contents
            |_| true,
        )
    }

    fn unit_ids(&self) -> &[(Id, Id)] {
        &self.ids[1..self.units.len()]
    }

    fn unit(&self, unit_id: usize) -> Option<&UnitIndex> {
        self.units.get(unit_id)
    }

    fn unit_side(&self, unit_id: usize) -> Side {
        match self.get_pair(unit_id) {
            Some((Id::Unit { .. }, Id::None)) => Side::Left,
            Some((Id::None, Id::Unit { .. })) => Side::Right,
            _ => panic!("unit is not single sided"),
        }
    }

    pub fn types<'a, 'input>(
        &self,
        unit: &'a Unit<'input>,
        options: &Options,
    ) -> Vec<&'a Type<'input>> {
        single(
            self.unit_side(unit.id()),
            self.type_ids(unit.id()),
            |a| unit.types().get(a.type_index()?),
            |ty| filter::filter_type(ty, options),
        )
    }

    pub fn merged_types<'a, 'input>(
        &self,
        unit_a: &'a Unit<'input>,
        unit_b: &'a Unit<'input>,
        options: &Options,
    ) -> Vec<MergeResult<&'a Type<'input>, &'a Type<'input>>> {
        merge(
            self.type_ids(unit_a.id()),
            |a| unit_a.types().get(a.type_index()?),
            |b| unit_b.types().get(b.type_index()?),
            |ty| filter::filter_type(ty, options),
        )
    }

    fn type_ids(&self, unit_id: usize) -> &[(Id, Id)] {
        self.unit(unit_id)
            .map(|unit| &self.ids[unit.types.clone()])
            .unwrap_or_default()
    }

    pub fn functions<'a, 'input>(
        &self,
        unit: &'a Unit<'input>,
        options: &Options,
    ) -> Vec<&'a Function<'input>> {
        single(
            self.unit_side(unit.id()),
            self.function_ids(unit.id()),
            |a| unit.functions().get(a.function_index()?),
            |function| filter::filter_function(function, options),
        )
    }

    pub fn merged_functions<'a, 'input>(
        &self,
        unit_a: &'a Unit<'input>,
        unit_b: &'a Unit<'input>,
        options: &Options,
    ) -> Vec<MergeResult<&'a Function<'input>, &'a Function<'input>>> {
        merge(
            self.function_ids(unit_a.id()),
            |a| unit_a.functions().get(a.function_index()?),
            |b| unit_b.functions().get(b.function_index()?),
            |function| filter::filter_function(function, options),
        )
    }

    fn function_ids(&self, unit_id: usize) -> &[(Id, Id)] {
        self.unit(unit_id)
            .map(|unit| &self.ids[unit.functions.clone()])
            .unwrap_or_default()
    }

    pub fn variables<'a, 'input>(
        &self,
        unit: &'a Unit<'input>,
        options: &Options,
    ) -> Vec<&'a Variable<'input>> {
        single(
            self.unit_side(unit.id()),
            self.variable_ids(unit.id()),
            |a| unit.variables().get(a.variable_index()?),
            |variable| filter::filter_variable(variable, options),
        )
    }

    pub fn merged_variables<'a, 'input>(
        &self,
        unit_a: &'a Unit<'input>,
        unit_b: &'a Unit<'input>,
        options: &Options,
    ) -> Vec<MergeResult<&'a Variable<'input>, &'a Variable<'input>>> {
        merge(
            self.variable_ids(unit_a.id()),
            |a| unit_a.variables().get(a.variable_index()?),
            |b| unit_b.variables().get(b.variable_index()?),
            |variable| filter::filter_variable(variable, options),
        )
    }

    fn variable_ids(&self, unit_id: usize) -> &[(Id, Id)] {
        self.unit(unit_id)
            .map(|unit| &self.ids[unit.variables.clone()])
            .unwrap_or_default()
    }
}

fn assign_merged_ids(hash_a: &FileHash, hash_b: Option<&FileHash>, options: &Options) -> Index {
    let mut units_a = filter::enumerate_index_units(hash_a.file, options);
    let (hash_b, unit_merge) = if let Some(hash_b) = hash_b {
        let mut units_b = filter::enumerate_index_units(hash_b.file, options);
        units_a.sort_by(|x, y| Unit::cmp_id(hash_a, x.1, hash_a, y.1, options));
        units_b.sort_by(|x, y| Unit::cmp_id(hash_b, x.1, hash_b, y.1, options));
        let unit_merge: Vec<_> =
            MergeIterator::new(units_a.into_iter(), units_b.into_iter(), |a, b| {
                Unit::cmp_id(hash_a, a.1, hash_b, b.1, options)
            })
            .collect();
        (hash_b, unit_merge)
    } else {
        // All entries are `Left` so hash_b is never used.
        (hash_a, units_a.into_iter().map(MergeResult::Left).collect())
    };

    // Assign the unit ids first, so both `ids` and `units` can be indexed by unit id.
    // id 0 is reserved for None.
    let mut ids = vec![(Id::None, Id::None)];
    for unit in &unit_merge {
        match *unit {
            MergeResult::Both((unit_index_a, unit_a), (unit_index_b, unit_b)) => {
                unit_a.set_id(ids.len());
                unit_b.set_id(ids.len());
                ids.push((
                    Id::Unit {
                        unit_index: unit_index_a,
                    },
                    Id::Unit {
                        unit_index: unit_index_b,
                    },
                ));
            }
            MergeResult::Left((unit_index, unit)) => {
                unit.set_id(ids.len());
                ids.push((Id::Unit { unit_index }, Id::None));
            }
            MergeResult::Right((unit_index, unit)) => {
                unit.set_id(ids.len());
                ids.push((Id::None, Id::Unit { unit_index }));
            }
        }
    }

    let mut units = vec![UnitIndex::default()];
    for unit in &unit_merge {
        units.push(match *unit {
            MergeResult::Both((unit_index_a, unit_a), (unit_index_b, unit_b)) => {
                assign_merged_ids_in_unit(
                    hash_a,
                    unit_index_a,
                    unit_a,
                    hash_b,
                    unit_index_b,
                    unit_b,
                    options,
                    &mut ids,
                )
            }
            MergeResult::Left((unit_index, unit)) => {
                assign_unmerged_ids_in_unit(hash_a, unit_index, unit, &mut ids, Side::Left)
            }
            MergeResult::Right((unit_index, unit)) => {
                assign_unmerged_ids_in_unit(hash_b, unit_index, unit, &mut ids, Side::Right)
            }
        });
    }

    Index { ids, units }
}

fn assign_unmerged_ids_in_unit(
    hash: &FileHash,
    unit_index: usize,
    unit: &Unit,
    ids: &mut Vec<(Id, Id)>,
    side: Side,
) -> UnitIndex {
    let types_start = ids.len();
    for (type_index, ty) in filter::enumerate_index_types(unit, hash, false) {
        ty.set_id(ids.len());
        let id = Id::Type {
            unit_index,
            type_index,
        };
        ids.push(side.set(id));
    }

    let functions_start = ids.len();
    for (function_index, function) in filter::enumerate_index_functions(unit) {
        function.set_id(ids.len());
        let id = Id::Function {
            unit_index,
            function_index,
        };
        ids.push(side.set(id));
    }

    let variables_start = ids.len();
    for (variable_index, variable) in filter::enumerate_index_variables(unit) {
        variable.set_id(ids.len());
        let id = Id::Variable {
            unit_index,
            variable_index,
        };
        ids.push(side.set(id));
    }
    UnitIndex {
        types: types_start..functions_start,
        functions: functions_start..variables_start,
        variables: variables_start..ids.len(),
    }
}

fn assign_merged_ids_in_unit(
    hash_a: &FileHash,
    unit_index_a: usize,
    unit_a: &Unit,
    hash_b: &FileHash,
    unit_index_b: usize,
    unit_b: &Unit,
    options: &Options,
    ids: &mut Vec<(Id, Id)>,
) -> UnitIndex {
    let mut types_a = filter::enumerate_index_types(unit_a, hash_a, true);
    types_a.sort_by(|x, y| Type::cmp_id_for_sort(hash_a, x.1, hash_a, y.1, options));
    let mut types_b = filter::enumerate_index_types(unit_b, hash_b, true);
    types_b.sort_by(|x, y| Type::cmp_id_for_sort(hash_b, x.1, hash_b, y.1, options));
    let types = MergeIterator::new(types_a.into_iter(), types_b.into_iter(), |a, b| {
        Type::cmp_id(hash_a, a.1, hash_b, b.1)
    });
    let types_start = ids.len();
    for ty in types {
        match ty {
            MergeResult::Both((type_index_a, a), (type_index_b, b)) => {
                a.set_id(ids.len());
                b.set_id(ids.len());
                ids.push((
                    Id::Type {
                        unit_index: unit_index_a,
                        type_index: type_index_a,
                    },
                    Id::Type {
                        unit_index: unit_index_b,
                        type_index: type_index_b,
                    },
                ));
            }
            MergeResult::Left((type_index, ty)) => {
                ty.set_id(ids.len());
                ids.push((
                    Id::Type {
                        unit_index: unit_index_a,
                        type_index,
                    },
                    Id::None,
                ));
            }
            MergeResult::Right((type_index, ty)) => {
                ty.set_id(ids.len());
                ids.push((
                    Id::None,
                    Id::Type {
                        unit_index: unit_index_b,
                        type_index,
                    },
                ));
            }
        }
    }

    let mut functions_a = filter::enumerate_index_functions(unit_a);
    functions_a.sort_by(|x, y| Function::cmp_id_for_sort(hash_a, x.1, hash_a, y.1, options));
    let mut functions_b = filter::enumerate_index_functions(unit_b);
    functions_b.sort_by(|x, y| Function::cmp_id_for_sort(hash_b, x.1, hash_b, y.1, options));
    let functions = MergeIterator::new(functions_a.into_iter(), functions_b.into_iter(), |a, b| {
        <Function as SortList>::cmp_id(hash_a, a.1, hash_b, b.1, options)
    });
    let functions_start = ids.len();
    for function in functions {
        match function {
            MergeResult::Both((function_index_a, a), (function_index_b, b)) => {
                a.set_id(ids.len());
                b.set_id(ids.len());
                ids.push((
                    Id::Function {
                        unit_index: unit_index_a,
                        function_index: function_index_a,
                    },
                    Id::Function {
                        unit_index: unit_index_b,
                        function_index: function_index_b,
                    },
                ));
            }
            MergeResult::Left((function_index, function)) => {
                function.set_id(ids.len());
                ids.push((
                    Id::Function {
                        unit_index: unit_index_a,
                        function_index,
                    },
                    Id::None,
                ));
            }
            MergeResult::Right((function_index, function)) => {
                function.set_id(ids.len());
                ids.push((
                    Id::None,
                    Id::Function {
                        unit_index: unit_index_b,
                        function_index,
                    },
                ));
            }
        }
    }

    let mut variables_a = filter::enumerate_index_variables(unit_a);
    variables_a.sort_by(|x, y| Variable::cmp_id_for_sort(hash_a, x.1, hash_a, y.1, options));
    let mut variables_b = filter::enumerate_index_variables(unit_b);
    variables_b.sort_by(|x, y| Variable::cmp_id_for_sort(hash_b, x.1, hash_b, y.1, options));
    let variables = MergeIterator::new(variables_a.into_iter(), variables_b.into_iter(), |a, b| {
        <Variable as SortList>::cmp_id(hash_a, a.1, hash_b, b.1, options)
    });
    let variables_start = ids.len();
    for variable in variables {
        match variable {
            MergeResult::Both((variable_index_a, a), (variable_index_b, b)) => {
                a.set_id(ids.len());
                b.set_id(ids.len());
                ids.push((
                    Id::Variable {
                        unit_index: unit_index_a,
                        variable_index: variable_index_a,
                    },
                    Id::Variable {
                        unit_index: unit_index_b,
                        variable_index: variable_index_b,
                    },
                ));
            }
            MergeResult::Left((variable_index, variable)) => {
                variable.set_id(ids.len());
                ids.push((
                    Id::Variable {
                        unit_index: unit_index_a,
                        variable_index,
                    },
                    Id::None,
                ));
            }
            MergeResult::Right((variable_index, variable)) => {
                variable.set_id(ids.len());
                ids.push((
                    Id::None,
                    Id::Variable {
                        unit_index: unit_index_b,
                        variable_index,
                    },
                ));
            }
        }
    }
    UnitIndex {
        types: types_start..functions_start,
        functions: functions_start..variables_start,
        variables: variables_start..ids.len(),
    }
}

#[derive(Clone, Copy, PartialEq)]
enum Side {
    Left,
    Right,
}

impl Side {
    fn get(self, id: (Id, Id)) -> Id {
        match self {
            Side::Left => id.0,
            Side::Right => id.1,
        }
    }

    fn set(self, id: Id) -> (Id, Id) {
        match self {
            Side::Left => (id, Id::None),
            Side::Right => (Id::None, id),
        }
    }
}

/// Convert a range of `Index::ids` into the items of a single file.
///
/// `keep` applies the user specified filter options.
fn single<'a, T: 'a>(
    side: Side,
    ids: &[(Id, Id)],
    get: impl Fn(Id) -> Option<&'a T>,
    keep: impl Fn(&&T) -> bool,
) -> Vec<&'a T> {
    ids.iter()
        .filter_map(move |ids| get(side.get(*ids)))
        .filter(keep)
        .collect()
}

/// Convert a range of `Index::ids` into the merged items that they refer to.
///
/// `keep` applies the user specified filter options.
fn merge<'a, T: 'a>(
    ids: &[(Id, Id)],
    get_a: impl Fn(Id) -> Option<&'a T>,
    get_b: impl Fn(Id) -> Option<&'a T>,
    keep: impl Fn(&T) -> bool,
) -> Vec<MergeResult<&'a T, &'a T>> {
    ids.iter()
        .filter_map(move |&(id_a, id_b)| merge_result(get_a(id_a), get_b(id_b)))
        .filter_map(|item| match item {
            // If only one side of a merged pair matches, demote the pair to an
            // added/deleted item. This matters for function-inline.
            MergeResult::Both(a, b) => match (keep(a), keep(b)) {
                (true, true) => Some(MergeResult::Both(a, b)),
                (true, false) => Some(MergeResult::Left(a)),
                (false, true) => Some(MergeResult::Right(b)),
                (false, false) => None,
            },
            MergeResult::Left(a) => keep(a).then_some(MergeResult::Left(a)),
            MergeResult::Right(b) => keep(b).then_some(MergeResult::Right(b)),
        })
        .collect()
}

fn merge_result<T>(a: Option<T>, b: Option<T>) -> Option<MergeResult<T, T>> {
    match (a, b) {
        (Some(a), Some(b)) => Some(MergeResult::Both(a, b)),
        (Some(a), None) => Some(MergeResult::Left(a)),
        (None, Some(b)) => Some(MergeResult::Right(b)),
        (None, None) => None,
    }
}
