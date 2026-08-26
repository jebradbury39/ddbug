use parser::FileHash;

use crate::code::Code;
use crate::merge::{MergeIterator, MergeResult};
use crate::print::{DiffList, DiffPrefix, Print, Printer, SortList, ValuePrinter};
use crate::{Options, Result};

/// The `PrintState` fields that need to be captured by closures.
#[derive(Clone, Copy)]
struct PrintCapture<'a> {
    hash: &'a FileHash<'a>,
    code: Option<&'a Code<'a>>,
    options: &'a Options,
}

impl<'a> PrintCapture<'a> {
    fn state<'b>(self, printer: &'b mut dyn Printer) -> PrintState<'b>
    where
        'a: 'b,
    {
        PrintState {
            printer,
            hash: self.hash,
            code: self.code,
            options: self.options,
        }
    }
}

pub(crate) struct PrintState<'a> {
    // 'w lifetime needed due to invariance
    printer: &'a mut dyn Printer,

    // The remaining fields contain information that is commonly needed in print methods.
    hash: &'a FileHash<'a>,
    code: Option<&'a Code<'a>>,
    options: &'a Options,
}

impl<'a> PrintState<'a> {
    #[inline]
    pub fn hash(&self) -> &'a FileHash<'a> {
        self.hash
    }

    #[inline]
    pub fn code(&self) -> Option<&'a Code<'a>> {
        self.code
    }

    #[inline]
    pub fn options(&self) -> &'a Options {
        self.options
    }

    fn capture(&self) -> PrintCapture<'a> {
        PrintCapture {
            hash: self.hash,
            code: self.code,
            options: self.options,
        }
    }

    pub fn new(
        printer: &'a mut dyn Printer,
        hash: &'a FileHash<'a>,
        code: Option<&'a Code<'a>>,
        options: &'a Options,
    ) -> Self {
        PrintState {
            printer,
            hash,
            code,
            options,
        }
    }

    pub fn id<FHeader, FBody>(
        &mut self,
        id: usize,
        mut header: FHeader,
        mut body: FBody,
    ) -> Result<()>
    where
        FHeader: FnMut(&mut PrintState) -> Result<()>,
        FBody: FnMut(&mut PrintState) -> Result<()>,
    {
        let capture = self.capture();
        self.printer.indent_id(
            id,
            &mut |printer| header(&mut capture.state(printer)),
            &mut |printer| body(&mut capture.state(printer)),
        )
    }

    pub fn field_detail<FBody>(&mut self, id: &str, label: &str, body: FBody) -> Result<()>
    where
        FBody: FnMut(&mut PrintState) -> Result<()>,
    {
        if self.options.http {
            self.printer.indent_detail(id, label)
        } else {
            self.indent_impl(true, true, |state| state.label(label), body)
        }
    }

    // Output the header with an indented body.
    // If optional is true, then only output if the body is not empty.
    fn indent_impl<FHeader, FBody>(
        &mut self,
        optional: bool,
        collapsed: bool,
        mut header: FHeader,
        mut body: FBody,
    ) -> Result<()>
    where
        FHeader: FnMut(&mut PrintState) -> Result<()>,
        FBody: FnMut(&mut PrintState) -> Result<()>,
    {
        let capture = self.capture();
        let not_empty = self
            .printer
            .indent_body(&mut |printer| body(&mut capture.state(printer)))?;
        if not_empty {
            self.printer.indent_header(collapsed, &mut |printer| {
                header(&mut capture.state(printer))
            })?;
        } else if !optional {
            header(self)?;
        }
        Ok(())
    }

    pub fn collapsed<FHeader, FBody>(&mut self, header: FHeader, body: FBody) -> Result<()>
    where
        FHeader: FnMut(&mut PrintState) -> Result<()>,
        FBody: FnMut(&mut PrintState) -> Result<()>,
    {
        self.indent_impl(false, true, header, body)
    }

    pub fn expanded<FHeader, FBody>(&mut self, header: FHeader, body: FBody) -> Result<()>
    where
        FHeader: FnMut(&mut PrintState) -> Result<()>,
        FBody: FnMut(&mut PrintState) -> Result<()>,
    {
        self.indent_impl(false, false, header, body)
    }

    pub fn field_collapsed<FBody>(&mut self, label: &str, body: FBody) -> Result<()>
    where
        FBody: FnMut(&mut PrintState) -> Result<()>,
    {
        self.indent_impl(true, true, |state| state.label(label), body)
    }

    pub fn field_expanded<FBody>(&mut self, label: &str, body: FBody) -> Result<()>
    where
        FBody: FnMut(&mut PrintState) -> Result<()>,
    {
        self.indent_impl(true, false, |state| state.label(label), body)
    }

    pub fn inline<F>(&mut self, mut f: F) -> Result<()>
    where
        F: FnMut(&mut PrintState) -> Result<()>,
    {
        if self.printer.inline_begin() {
            let ret = f(self);
            self.printer.inline_end();
            ret
        } else {
            Ok(())
        }
    }

    fn prefix(
        &mut self,
        prefix: DiffPrefix,
        f: &mut dyn FnMut(&mut PrintState) -> Result<()>,
    ) -> Result<()> {
        self.printer.prefix(prefix);
        f(self)
    }

    pub fn line_break(&mut self) -> Result<()> {
        self.printer.line_break()
    }

    pub fn label(&mut self, label: &str) -> Result<()> {
        self.printer.line(label, &[])
    }

    fn line_impl<F>(&mut self, label: &str, mut f: F) -> Result<()>
    where
        F: FnMut(&mut dyn ValuePrinter, &FileHash) -> Result<()>,
    {
        let mut buf = Vec::new();
        let hash = self.hash;
        self.printer
            .value(&mut buf, &mut |printer| f(printer, hash))?;
        if !buf.is_empty() {
            self.printer.line(label, &buf)?;
        }
        Ok(())
    }

    pub fn line<F>(&mut self, f: F) -> Result<()>
    where
        F: FnMut(&mut dyn ValuePrinter, &FileHash) -> Result<()>,
    {
        self.line_impl("", f)
    }

    pub fn field<F>(&mut self, label: &str, f: F) -> Result<()>
    where
        F: FnMut(&mut dyn ValuePrinter, &FileHash) -> Result<()>,
    {
        self.line_impl(label, f)
    }

    pub fn field_u64(&mut self, label: &str, arg: u64) -> Result<()> {
        self.field(label, |w, _| {
            write!(w, "{}", arg)?;
            Ok(())
        })
    }

    pub fn instruction<F>(&mut self, address: Option<u64>, mnemonic: &str, mut f: F) -> Result<()>
    where
        F: FnMut(&mut dyn ValuePrinter, &FileHash) -> Result<()>,
    {
        let mut buf = Vec::new();
        let hash = self.hash;
        self.printer
            .value(&mut buf, &mut |printer| f(printer, hash))?;
        self.printer.instruction(address, mnemonic, &buf)
    }

    pub fn list<T: Print>(&mut self, arg: &T::Arg, list: &[T]) -> Result<()> {
        for item in list {
            item.print(self, arg)?;
        }
        Ok(())
    }

    pub fn sort_list<T: SortList>(&mut self, arg: &T::Arg, list: &mut [&T]) -> Result<()> {
        list.sort_by(|a, b| T::cmp_by(self.hash, a, self.hash, b, self.options));
        for item in list {
            item.print(self, arg)?;
        }
        Ok(())
    }
}

/// The `DiffState` fields that need to be captured by closures.
#[derive(Clone, Copy)]
struct DiffCapture<'a> {
    hash_a: &'a FileHash<'a>,
    hash_b: &'a FileHash<'a>,
    code_a: Option<&'a Code<'a>>,
    code_b: Option<&'a Code<'a>>,
    options: &'a Options,
}

impl<'a> DiffCapture<'a> {
    fn state<'b>(self, printer: &'b mut dyn Printer) -> DiffState<'b>
    where
        'a: 'b,
    {
        DiffState {
            printer,
            diff: false,
            hash_a: self.hash_a,
            hash_b: self.hash_b,
            code_a: self.code_a,
            code_b: self.code_b,
            options: self.options,
        }
    }
}

pub(crate) struct DiffState<'a> {
    printer: &'a mut dyn Printer,

    // True if DiffPrefix::Delete or DiffPrefix::Add was printed.
    diff: bool,

    // The remaining fields contain information that is commonly needed in print methods.
    hash_a: &'a FileHash<'a>,
    hash_b: &'a FileHash<'a>,
    code_a: Option<&'a Code<'a>>,
    code_b: Option<&'a Code<'a>>,
    options: &'a Options,
}

impl<'a> DiffState<'a> {
    #[inline]
    fn a(&'_ mut self) -> PrintState<'_> {
        PrintState::new(self.printer, self.hash_a, self.code_a, self.options)
    }

    #[inline]
    fn b(&'_ mut self) -> PrintState<'_> {
        PrintState::new(self.printer, self.hash_b, self.code_b, self.options)
    }

    #[inline]
    pub fn hash_a(&self) -> &'a FileHash<'a> {
        self.hash_a
    }

    #[inline]
    pub fn hash_b(&self) -> &'a FileHash<'a> {
        self.hash_b
    }

    #[inline]
    pub fn code_a(&self) -> Option<&'a Code<'a>> {
        self.code_a
    }

    #[inline]
    pub fn code_b(&self) -> Option<&'a Code<'a>> {
        self.code_b
    }

    #[inline]
    pub fn options(&self) -> &'a Options {
        self.options
    }

    fn capture(&self) -> DiffCapture<'a> {
        DiffCapture {
            hash_a: self.hash_a,
            hash_b: self.hash_b,
            code_a: self.code_a,
            code_b: self.code_b,
            options: self.options,
        }
    }

    pub fn new(
        printer: &'a mut dyn Printer,
        hash_a: &'a FileHash<'a>,
        hash_b: &'a FileHash<'a>,
        code_a: Option<&'a Code<'a>>,
        code_b: Option<&'a Code<'a>>,
        options: &'a Options,
    ) -> Self {
        DiffState {
            printer,
            diff: false,
            hash_a,
            hash_b,
            code_a,
            code_b,
            options,
        }
    }

    /// Write output of `f` to a temporary buffer, then only
    /// output that buffer if there were any differences.
    fn print_if_diff<F>(&mut self, mut f: F) -> Result<()>
    where
        F: FnMut(&mut DiffState) -> Result<()>,
    {
        let capture = self.capture();
        let mut diff = false;
        self.printer.buffer(&mut |printer| {
            let mut state = capture.state(printer);
            f(&mut state)?;
            diff = state.diff;
            Ok(())
        })?;
        self.diff |= diff;
        if diff || self.options.html {
            self.printer.write_buf()?;
        }
        Ok(())
    }

    // Don't allow `f` to update self.diff if flag is true.
    pub fn ignore_diff<F>(&mut self, flag: bool, mut f: F) -> Result<()>
    where
        F: FnMut(&mut DiffState) -> Result<()>,
    {
        let diff = self.diff;
        f(self)?;
        if flag {
            self.diff = diff;
        }
        Ok(())
    }

    pub fn id<FHeader, FBody>(
        &mut self,
        id: usize,
        mut header: FHeader,
        mut body: FBody,
    ) -> Result<()>
    where
        FHeader: FnMut(&mut DiffState) -> Result<()>,
        FBody: FnMut(&mut DiffState) -> Result<()>,
    {
        let capture = self.capture();

        // Render the body first so that we can determine if there are differences.
        // TODO: this makes the initial HTTP load much slower than it could be.
        let mut diff = false;
        self.printer.indent_body(&mut |printer| {
            printer.prefix(DiffPrefix::Equal);
            let mut state = capture.state(printer);
            body(&mut state)?;
            diff |= state.diff;
            Ok(())
        })?;

        self.printer.indent_id(
            id,
            &mut |printer| {
                if diff {
                    printer.prefix(DiffPrefix::Modify);
                } else {
                    printer.prefix(DiffPrefix::Equal);
                }
                let mut state = capture.state(printer);
                header(&mut state)?;
                diff |= state.diff;
                Ok(())
            },
            &mut |printer| {
                printer.write_buf()?;
                Ok(())
            },
        )?;
        self.diff |= diff;
        Ok(())
    }

    /// Output the header with an indented body.
    ///
    /// If optional is true, then only output if the body is not empty.
    fn indent_impl<FHeader, FBody>(
        &mut self,
        optional: bool,
        collapsed: bool,
        mut header: FHeader,
        mut body: FBody,
    ) -> Result<()>
    where
        FHeader: FnMut(&mut DiffState) -> Result<()>,
        FBody: FnMut(&mut DiffState) -> Result<()>,
    {
        let capture = self.capture();

        // Render the body first so that we can determine if there are differences
        // or if it is empty.
        let mut diff = false;
        let not_empty = self.printer.indent_body(&mut |printer| {
            printer.prefix(DiffPrefix::Equal);
            let mut state = capture.state(printer);
            body(&mut state)?;
            if state.diff {
                diff = true;
            }
            Ok(())
        })?;

        if not_empty {
            self.printer.indent_header(collapsed, &mut |printer| {
                if diff {
                    printer.prefix(DiffPrefix::Modify);
                } else {
                    printer.prefix(DiffPrefix::Equal);
                }
                let mut state = capture.state(printer);
                header(&mut state)?;
                if state.diff {
                    diff = true;
                }
                Ok(())
            })?;
            self.diff |= diff;
        } else if !optional {
            header(self)?;
        }
        Ok(())
    }

    pub fn collapsed<FHeader, FBody>(&mut self, header: FHeader, body: FBody) -> Result<()>
    where
        FHeader: FnMut(&mut DiffState) -> Result<()>,
        FBody: FnMut(&mut DiffState) -> Result<()>,
    {
        self.indent_impl(false, true, header, body)
    }

    pub fn expanded<FHeader, FBody>(&mut self, header: FHeader, body: FBody) -> Result<()>
    where
        FHeader: FnMut(&mut DiffState) -> Result<()>,
        FBody: FnMut(&mut DiffState) -> Result<()>,
    {
        self.indent_impl(false, false, header, body)
    }

    pub fn field_collapsed<FBody>(&mut self, label: &str, body: FBody) -> Result<()>
    where
        FBody: FnMut(&mut DiffState) -> Result<()>,
    {
        self.indent_impl(true, true, |state| state.label(label), body)
    }

    pub fn field_expanded<FBody>(&mut self, label: &str, body: FBody) -> Result<()>
    where
        FBody: FnMut(&mut DiffState) -> Result<()>,
    {
        self.indent_impl(true, false, |state| state.label(label), body)
    }

    pub fn inline<F>(&mut self, mut f: F) -> Result<()>
    where
        F: FnMut(&mut DiffState) -> Result<()>,
    {
        if self.printer.inline_begin() {
            let ret = f(self);
            self.printer.inline_end();
            ret
        } else {
            Ok(())
        }
    }

    pub fn prefix_delete<F>(&mut self, mut f: F) -> Result<()>
    where
        F: FnMut(&mut PrintState) -> Result<()>,
    {
        self.a().prefix(DiffPrefix::Delete, &mut f)?;
        // Assume something is always written.
        self.diff = true;
        Ok(())
    }

    pub fn prefix_add<F>(&mut self, mut f: F) -> Result<()>
    where
        F: FnMut(&mut PrintState) -> Result<()>,
    {
        self.b().prefix(DiffPrefix::Add, &mut f)?;
        // Assume something is always written.
        self.diff = true;
        Ok(())
    }

    // Multiline blocks that are always different, but may be empty.
    pub fn block<F, T>(&mut self, arg_a: T, arg_b: T, mut f: F) -> Result<()>
    where
        F: FnMut(&mut PrintState, T) -> Result<()>,
        T: Copy,
    {
        let capture = self.capture();
        let not_empty = self.printer.buffer(&mut |printer| {
            let mut state = capture.state(printer);
            state
                .a()
                .prefix(DiffPrefix::Delete, &mut |state| f(state, arg_a))?;
            state
                .b()
                .prefix(DiffPrefix::Add, &mut |state| f(state, arg_b))?;
            Ok(())
        })?;
        if not_empty {
            self.printer.write_buf()?;
            self.diff = true;
        }
        Ok(())
    }

    pub fn line_break(&mut self) -> Result<()> {
        self.printer.line_break()
    }

    pub fn label(&mut self, label: &str) -> Result<()> {
        if self.printer.get_prefix() != DiffPrefix::Modify {
            self.printer.prefix(DiffPrefix::Equal);
        }
        self.printer.line(label, &[])
    }

    fn line_impl<F, T>(&mut self, label: &str, arg_a: T, arg_b: T, mut f: F) -> Result<()>
    where
        F: FnMut(&mut dyn ValuePrinter, &FileHash, T) -> Result<()>,
        T: Copy,
    {
        let mut a = Vec::new();
        let hash_a = self.hash_a;
        self.printer
            .value(&mut a, &mut |printer| f(printer, hash_a, arg_a))?;

        let mut b = Vec::new();
        let hash_b = self.hash_b;
        self.printer
            .value(&mut b, &mut |printer| f(printer, hash_b, arg_b))?;

        if a == b {
            if !a.is_empty() {
                if self.printer.get_prefix() != DiffPrefix::Modify {
                    self.printer.prefix(DiffPrefix::Equal);
                }
                self.printer.line(label, &a)?;
            }
        } else {
            if a.is_empty() {
                self.printer.prefix(DiffPrefix::Add);
                self.printer.line(label, &b)?;
            } else if b.is_empty() {
                self.printer.prefix(DiffPrefix::Delete);
                self.printer.line(label, &a)?;
            } else {
                self.printer.line_diff(label, &a, &b)?;
            }
            self.diff = true;
        }
        Ok(())
    }

    pub fn line<F, T>(&mut self, arg_a: T, arg_b: T, f: F) -> Result<()>
    where
        F: FnMut(&mut dyn ValuePrinter, &FileHash, T) -> Result<()>,
        T: Copy,
    {
        self.line_impl("", arg_a, arg_b, f)
    }

    pub fn field<F, T>(&mut self, label: &str, arg_a: T, arg_b: T, f: F) -> Result<()>
    where
        F: FnMut(&mut dyn ValuePrinter, &FileHash, T) -> Result<()>,
        T: Copy,
    {
        self.line_impl(label, arg_a, arg_b, f)
    }

    pub fn field_u64(&mut self, label: &str, arg_a: u64, arg_b: u64) -> Result<()> {
        let base = arg_a;
        self.field(label, arg_a, arg_b, |w, _hash, arg| {
            write!(w, "{}", arg)?;
            if arg != base {
                write!(w, " ({:+})", arg as i64 - base as i64)?;
            }
            Ok(())
        })
    }

    pub fn list<T: DiffList>(
        &mut self,
        arg_a: &T::Arg,
        list_a: &[T],
        arg_b: &T::Arg,
        list_b: &[T],
    ) -> Result<()> {
        use crate::shortest_path::{Direction, shortest_path};
        let path = shortest_path(
            list_a,
            list_b,
            |a| a.step_cost(self, arg_a),
            |b| b.step_cost(self, arg_b),
            |a, b| T::diff_cost(self, arg_a, a, arg_b, b),
        );
        let mut iter_a = list_a.iter();
        let mut iter_b = list_b.iter();
        for dir in path {
            match dir {
                Direction::None => break,
                Direction::Diagonal => {
                    if let (Some(a), Some(b)) = (iter_a.next(), iter_b.next()) {
                        T::diff(self, arg_a, a, arg_b, b)?;
                    }
                }
                Direction::Horizontal => {
                    if let Some(a) = iter_a.next() {
                        self.prefix_delete(|state| a.print(state, arg_a))?;
                    }
                }
                Direction::Vertical => {
                    if let Some(b) = iter_b.next() {
                        self.prefix_add(|state| b.print(state, arg_b))?;
                    }
                }
            }
        }
        Ok(())
    }

    // This is similar to `list`, but because the items are ordered
    // we can do a greedy search.
    //
    // The caller must provide lists that are already sorted.
    pub fn ord_list<T: Ord + Print>(
        &mut self,
        arg_a: &T::Arg,
        list_a: &[T],
        arg_b: &T::Arg,
        list_b: &[T],
    ) -> Result<()> {
        for item in MergeIterator::new(list_a.iter(), list_b.iter(), <&T>::cmp) {
            match item {
                MergeResult::Both(a, b) => {
                    T::diff(self, arg_a, a, arg_b, b)?;
                }
                MergeResult::Left(a) => {
                    self.prefix_delete(|state| a.print(state, arg_a))?;
                }
                MergeResult::Right(b) => {
                    self.prefix_add(|state| b.print(state, arg_b))?;
                }
            }
        }
        Ok(())
    }

    // Sort then display a merged list of items.
    //
    // Items with no difference are not displayed.
    //
    // Also, self.options controls:
    // - sort order
    // - display of added/deleted options
    pub fn sort_list<'i, T>(
        &mut self,
        arg_a: &T::Arg,
        arg_b: &T::Arg,
        list: &mut [MergeResult<&'i T, &'i T>],
    ) -> Result<()>
    where
        T: SortList + 'i,
    {
        list.sort_by(|x, y| {
            MergeResult::cmp(x, y, &self.hash_a, &self.hash_b, |x, hash_x, y, hash_y| {
                T::cmp_by(hash_x, x, hash_y, y, self.options)
            })
        });

        for item in list {
            match *item {
                MergeResult::Both(a, b) => {
                    self.print_if_diff(|state| T::diff(state, arg_a, a, arg_b, b))?;
                }
                MergeResult::Left(a) => {
                    if !self.options.ignore_deleted {
                        self.prefix_delete(|state| a.print(state, arg_a))?;
                    }
                }
                MergeResult::Right(b) => {
                    if !self.options.ignore_added {
                        self.prefix_add(|state| b.print(state, arg_b))?;
                    }
                }
            }
        }
        Ok(())
    }
}
