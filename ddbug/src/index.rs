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

pub(crate) struct PrintIndex {
    ids: Vec<Id>,
}

impl PrintIndex {
    pub fn new(file: &File, options: &Options) -> PrintIndex {
        let ids = assign_ids(file, options);
        PrintIndex { ids }
    }

    pub(crate) fn get(&self, id: usize) -> Option<Id> {
        self.ids.get(id).copied()
    }

    pub fn parent(&self, id: usize, file: &File) -> Option<usize> {
        self.get(id)?.parent(file)
    }
}

fn assign_ids(file: &File, options: &Options) -> Vec<Id> {
    // id 0 is reserved for None.
    let mut ids = vec![Id::None];
    for (unit_index, unit) in file.units().iter().enumerate() {
        unit.set_id(ids.len());
        ids.push(Id::Unit { unit_index });
        assign_ids_in_unit(unit_index, unit, options, &mut ids);
    }
    ids
}

fn assign_ids_in_unit(unit_index: usize, unit: &Unit, _options: &Options, ids: &mut Vec<Id>) {
    for (type_index, ty) in unit.types().iter().enumerate() {
        ty.set_id(ids.len());
        ids.push(Id::Type {
            unit_index,
            type_index,
        });
    }
    for (function_index, function) in unit.functions().iter().enumerate() {
        function.set_id(ids.len());
        ids.push(Id::Function {
            unit_index,
            function_index,
        });
    }
    for (variable_index, variable) in unit.variables().iter().enumerate() {
        variable.set_id(ids.len());
        ids.push(Id::Variable {
            unit_index,
            variable_index,
        });
    }
}

pub(crate) struct DiffIndex {
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

impl DiffIndex {
    pub fn new(file_a: &FileHash, file_b: &FileHash, options: &Options) -> DiffIndex {
        assign_merged_ids(file_a, file_b, options)
    }

    pub fn get(&self, id: usize) -> Option<(Id, Id)> {
        self.ids.get(id).copied()
    }

    pub fn parent(&self, id: usize, file_a: &File, file_b: &File) -> Option<usize> {
        let (id_a, id_b) = self.get(id)?;
        id_a.parent(file_a).or_else(|| id_b.parent(file_b))
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
        )
    }

    fn unit_ids(&self) -> &[(Id, Id)] {
        &self.ids[1..self.units.len()]
    }

    fn unit(&self, unit_id: usize) -> Option<&UnitIndex> {
        self.units.get(unit_id)
    }

    pub fn merged_types<'a, 'input>(
        &self,
        unit_a: &'a Unit<'input>,
        unit_b: &'a Unit<'input>,
    ) -> Vec<MergeResult<&'a Type<'input>, &'a Type<'input>>> {
        merge(
            self.type_ids(unit_a.id()),
            |a| unit_a.types().get(a.type_index()?),
            |b| unit_b.types().get(b.type_index()?),
        )
    }

    fn type_ids(&self, unit_id: usize) -> &[(Id, Id)] {
        self.unit(unit_id)
            .map(|unit| &self.ids[unit.types.clone()])
            .unwrap_or_default()
    }

    pub fn merged_functions<'a, 'input>(
        &self,
        unit_a: &'a Unit<'input>,
        unit_b: &'a Unit<'input>,
    ) -> Vec<MergeResult<&'a Function<'input>, &'a Function<'input>>> {
        merge(
            self.function_ids(unit_a.id()),
            |a| unit_a.functions().get(a.function_index()?),
            |b| unit_b.functions().get(b.function_index()?),
        )
    }

    fn function_ids(&self, unit_id: usize) -> &[(Id, Id)] {
        self.unit(unit_id)
            .map(|unit| &self.ids[unit.functions.clone()])
            .unwrap_or_default()
    }

    pub fn merged_variables<'a, 'input>(
        &self,
        unit_a: &'a Unit<'input>,
        unit_b: &'a Unit<'input>,
    ) -> Vec<MergeResult<&'a Variable<'input>, &'a Variable<'input>>> {
        merge(
            self.variable_ids(unit_a.id()),
            |a| unit_a.variables().get(a.variable_index()?),
            |b| unit_b.variables().get(b.variable_index()?),
        )
    }

    fn variable_ids(&self, unit_id: usize) -> &[(Id, Id)] {
        self.unit(unit_id)
            .map(|unit| &self.ids[unit.variables.clone()])
            .unwrap_or_default()
    }
}

fn assign_merged_ids(hash_a: &FileHash, hash_b: &FileHash, options: &Options) -> DiffIndex {
    let mut units_a = filter::enumerate_and_filter_units(hash_a.file, options);
    units_a.sort_by(|x, y| Unit::cmp_id(hash_a, x.1, hash_a, y.1, options));
    let mut units_b = filter::enumerate_and_filter_units(hash_b.file, options);
    units_b.sort_by(|x, y| Unit::cmp_id(hash_b, x.1, hash_b, y.1, options));
    let unit_merge: Vec<_> =
        MergeIterator::new(units_a.into_iter(), units_b.into_iter(), |a, b| {
            Unit::cmp_id(hash_a, a.1, hash_b, b.1, options)
        })
        .collect();

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
                assign_unmerged_ids_in_unit(unit_index, unit, options, &mut ids, true)
            }
            MergeResult::Right((unit_index, unit)) => {
                assign_unmerged_ids_in_unit(unit_index, unit, options, &mut ids, false)
            }
        });
    }

    DiffIndex { ids, units }
}

fn assign_unmerged_ids_in_unit(
    unit_index: usize,
    unit: &Unit,
    _options: &Options,
    ids: &mut Vec<(Id, Id)>,
    left: bool,
) -> UnitIndex {
    let types_start = ids.len();
    for (type_index, ty) in unit.types().iter().enumerate() {
        ty.set_id(ids.len());
        let id = Id::Type {
            unit_index,
            type_index,
        };
        if left {
            ids.push((id, Id::None));
        } else {
            ids.push((Id::None, id));
        }
    }
    let functions_start = ids.len();
    for (function_index, function) in unit.functions().iter().enumerate() {
        function.set_id(ids.len());
        let id = Id::Function {
            unit_index,
            function_index,
        };
        if left {
            ids.push((id, Id::None));
        } else {
            ids.push((Id::None, id));
        }
    }
    let variables_start = ids.len();
    for (variable_index, variable) in unit.variables().iter().enumerate() {
        variable.set_id(ids.len());
        let id = Id::Variable {
            unit_index,
            variable_index,
        };
        if left {
            ids.push((id, Id::None));
        } else {
            ids.push((Id::None, id));
        }
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
    let mut types_a = filter::enumerate_and_filter_types(unit_a, hash_a, options, true);
    types_a.sort_by(|x, y| Type::cmp_id_for_sort(hash_a, x.1, hash_a, y.1, options));
    let mut types_b = filter::enumerate_and_filter_types(unit_b, hash_b, options, true);
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

    let mut functions_a = filter::enumerate_and_filter_functions(unit_a, options);
    functions_a.sort_by(|x, y| Function::cmp_id_for_sort(hash_a, x.1, hash_a, y.1, options));
    let mut functions_b = filter::enumerate_and_filter_functions(unit_b, options);
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

    let mut variables_a = filter::enumerate_and_filter_variables(unit_a, options);
    variables_a.sort_by(|x, y| Variable::cmp_id_for_sort(hash_a, x.1, hash_a, y.1, options));
    let mut variables_b = filter::enumerate_and_filter_variables(unit_b, options);
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

/// Convert a range of `DiffIndex::ids` into the merged items that they refer to.
fn merge<'a, T: 'a>(
    ids: &[(Id, Id)],
    get_a: impl Fn(Id) -> Option<&'a T>,
    get_b: impl Fn(Id) -> Option<&'a T>,
) -> Vec<MergeResult<&'a T, &'a T>> {
    ids.iter()
        .filter_map(move |&(id_a, id_b)| merge_result(get_a(id_a), get_b(id_b)))
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
