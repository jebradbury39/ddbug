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
    let mut ids = Vec::new();
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
}

impl DiffIndex {
    pub fn new(file_a: &FileHash, file_b: &FileHash, options: &Options) -> DiffIndex {
        let ids = assign_merged_ids(file_a, file_b, options);
        DiffIndex { ids }
    }

    pub(crate) fn get(&self, id: usize) -> Option<(Id, Id)> {
        self.ids.get(id).copied()
    }

    pub fn parent(&self, id: usize, file_a: &File, file_b: &File) -> Option<usize> {
        let (id_a, id_b) = self.get(id)?;
        id_a.parent(file_a).or_else(|| id_b.parent(file_b))
    }
}

fn assign_merged_ids(hash_a: &FileHash, hash_b: &FileHash, options: &Options) -> Vec<(Id, Id)> {
    let mut ids = Vec::new();
    let mut units_a = filter::enumerate_and_filter_units(hash_a.file, options);
    units_a.sort_by(|x, y| Unit::cmp_id(hash_a, x.1, hash_a, y.1, options));
    let mut units_b = filter::enumerate_and_filter_units(hash_b.file, options);
    units_b.sort_by(|x, y| Unit::cmp_id(hash_b, x.1, hash_b, y.1, options));
    let units = MergeIterator::new(units_a.into_iter(), units_b.into_iter(), |a, b| {
        Unit::cmp_id(hash_a, a.1, hash_b, b.1, options)
    });
    for unit in units {
        match unit {
            MergeResult::Both((unit_index_a, a), (unit_index_b, b)) => {
                a.set_id(ids.len());
                b.set_id(ids.len());
                ids.push((
                    Id::Unit {
                        unit_index: unit_index_a,
                    },
                    Id::Unit {
                        unit_index: unit_index_b,
                    },
                ));
                assign_merged_ids_in_unit(
                    hash_a,
                    unit_index_a,
                    a,
                    hash_b,
                    unit_index_b,
                    b,
                    options,
                    &mut ids,
                );
            }
            MergeResult::Left((unit_index, unit)) => {
                unit.set_id(ids.len());
                ids.push((Id::Unit { unit_index }, Id::None));
                assign_unmerged_ids_in_unit(unit_index, unit, options, &mut ids, true);
            }
            MergeResult::Right((unit_index, unit)) => {
                unit.set_id(ids.len());
                ids.push((Id::None, Id::Unit { unit_index }));
                assign_unmerged_ids_in_unit(unit_index, unit, options, &mut ids, false);
            }
        }
    }
    ids
}

fn assign_unmerged_ids_in_unit(
    unit_index: usize,
    unit: &Unit,
    _options: &Options,
    ids: &mut Vec<(Id, Id)>,
    left: bool,
) {
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
}

fn assign_merged_ids_in_unit(
    hash_a: &FileHash,
    unit_a_index: usize,
    unit_a: &Unit,
    hash_b: &FileHash,
    unit_b_index: usize,
    unit_b: &Unit,
    options: &Options,
    ids: &mut Vec<(Id, Id)>,
) {
    let mut types_a = filter::enumerate_and_filter_types(unit_a, hash_a, options, true);
    types_a.sort_by(|x, y| Type::cmp_id_for_sort(hash_a, x.1, hash_a, y.1, options));
    let mut types_b = filter::enumerate_and_filter_types(unit_b, hash_b, options, true);
    types_b.sort_by(|x, y| Type::cmp_id_for_sort(hash_b, x.1, hash_b, y.1, options));
    let types = MergeIterator::new(types_a.into_iter(), types_b.into_iter(), |a, b| {
        Type::cmp_id(hash_a, a.1, hash_b, b.1)
    });
    for ty in types {
        match ty {
            MergeResult::Both((type_index_a, a), (type_index_b, b)) => {
                a.set_id(ids.len());
                b.set_id(ids.len());
                ids.push((
                    Id::Type {
                        unit_index: unit_a_index,
                        type_index: type_index_a,
                    },
                    Id::Type {
                        unit_index: unit_b_index,
                        type_index: type_index_b,
                    },
                ));
            }
            MergeResult::Left((type_index, ty)) => {
                ty.set_id(ids.len());
                ids.push((
                    Id::Type {
                        unit_index: unit_a_index,
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
                        unit_index: unit_b_index,
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
    let mut functions = Vec::new();
    let mut inlined_functions = Vec::new();
    for function in MergeIterator::new(functions_a.into_iter(), functions_b.into_iter(), |a, b| {
        <Function as SortList>::cmp_id(hash_a, a.1, hash_b, b.1, options)
    }) {
        let inline = match function {
            MergeResult::Both(a, b) => a.1.size().is_none() || b.1.size().is_none(),
            MergeResult::Left(a) => a.1.size().is_none(),
            MergeResult::Right(b) => b.1.size().is_none(),
        };
        if inline {
            inlined_functions.push(function);
        } else {
            functions.push(function);
        }
    }
    for function in functions.into_iter().chain(inlined_functions) {
        match function {
            MergeResult::Both((function_index_a, a), (function_index_b, b)) => {
                a.set_id(ids.len());
                b.set_id(ids.len());
                ids.push((
                    Id::Function {
                        unit_index: unit_a_index,
                        function_index: function_index_a,
                    },
                    Id::Function {
                        unit_index: unit_b_index,
                        function_index: function_index_b,
                    },
                ));
            }
            MergeResult::Left((function_index, function)) => {
                function.set_id(ids.len());
                ids.push((
                    Id::Function {
                        unit_index: unit_a_index,
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
                        unit_index: unit_b_index,
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
    for variable in variables {
        match variable {
            MergeResult::Both((variable_index_a, a), (variable_index_b, b)) => {
                a.set_id(ids.len());
                b.set_id(ids.len());
                ids.push((
                    Id::Variable {
                        unit_index: unit_a_index,
                        variable_index: variable_index_a,
                    },
                    Id::Variable {
                        unit_index: unit_b_index,
                        variable_index: variable_index_b,
                    },
                ));
            }
            MergeResult::Left((variable_index, variable)) => {
                variable.set_id(ids.len());
                ids.push((
                    Id::Variable {
                        unit_index: unit_a_index,
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
                        unit_index: unit_b_index,
                        variable_index,
                    },
                ));
            }
        }
    }
}
