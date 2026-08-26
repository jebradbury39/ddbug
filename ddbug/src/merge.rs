use std::cmp;

pub(crate) enum MergeResult<T, U> {
    Left(T),
    Right(U),
    Both(T, U),
}

impl<T> MergeResult<T, T> {
    pub(crate) fn cmp<Arg, F>(
        x: &Self,
        y: &Self,
        arg_left: Arg,
        arg_right: Arg,
        f: F,
    ) -> cmp::Ordering
    where
        Arg: Copy,
        F: Fn(&T, Arg, &T, Arg) -> cmp::Ordering,
    {
        match x {
            MergeResult::Both(_, x_right) => match y {
                MergeResult::Both(_, y_right) => f(x_right, arg_right, y_right, arg_right),
                MergeResult::Left(y_left) => f(x_right, arg_right, y_left, arg_left),
                MergeResult::Right(y_right) => f(x_right, arg_right, y_right, arg_right),
            },
            MergeResult::Left(x_left) => match y {
                MergeResult::Both(_, y_right) => f(x_left, arg_left, y_right, arg_right),
                MergeResult::Left(y_left) => f(x_left, arg_left, y_left, arg_left),
                MergeResult::Right(y_right) => f(x_left, arg_left, y_right, arg_right),
            },
            MergeResult::Right(x_right) => match y {
                MergeResult::Both(_, y_right) => f(x_right, arg_right, y_right, arg_right),
                MergeResult::Left(y_left) => f(x_right, arg_right, y_left, arg_left),
                MergeResult::Right(y_right) => f(x_right, arg_right, y_right, arg_right),
            },
        }
    }
}

pub(crate) struct MergeIterator<T, U, L, R, C>
where
    L: Iterator<Item = T>,
    R: Iterator<Item = U>,
    C: Fn(&T, &U) -> cmp::Ordering,
{
    iter_left: L,
    iter_right: R,
    item_left: Option<T>,
    item_right: Option<U>,
    item_cmp: C,
}

impl<T, U, L, R, C> MergeIterator<T, U, L, R, C>
where
    L: Iterator<Item = T>,
    R: Iterator<Item = U>,
    C: Fn(&T, &U) -> cmp::Ordering,
{
    pub(crate) fn new(mut iter_left: L, mut iter_right: R, item_cmp: C) -> Self {
        let item_left = iter_left.next();
        let item_right = iter_right.next();
        MergeIterator {
            iter_left,
            iter_right,
            item_left,
            item_right,
            item_cmp,
        }
    }
}

impl<T, U, L, R, C> Iterator for MergeIterator<T, U, L, R, C>
where
    L: Iterator<Item = T>,
    R: Iterator<Item = U>,
    C: Fn(&T, &U) -> cmp::Ordering,
{
    type Item = MergeResult<T, U>;

    fn next(&mut self) -> Option<MergeResult<T, U>> {
        match (self.item_left.take(), self.item_right.take()) {
            (Some(left), Some(right)) => match (self.item_cmp)(&left, &right) {
                cmp::Ordering::Equal => {
                    self.item_left = self.iter_left.next();
                    self.item_right = self.iter_right.next();
                    Some(MergeResult::Both(left, right))
                }
                cmp::Ordering::Less => {
                    self.item_left = self.iter_left.next();
                    self.item_right = Some(right);
                    Some(MergeResult::Left(left))
                }
                cmp::Ordering::Greater => {
                    self.item_left = Some(left);
                    self.item_right = self.iter_right.next();
                    Some(MergeResult::Right(right))
                }
            },
            (Some(left), None) => {
                self.item_left = self.iter_left.next();
                Some(MergeResult::Left(left))
            }
            (None, Some(right)) => {
                self.item_right = self.iter_right.next();
                Some(MergeResult::Right(right))
            }
            (None, None) => None,
        }
    }
}
