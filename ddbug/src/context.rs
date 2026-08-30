use parser::{File, FileHash};

use crate::code::Code;
use crate::index::{DiffIndex, PrintIndex};
use crate::print::bloat::BloatIndex;
use crate::print::{self, DiffState, PrintState, Printer};
use crate::{Options, Result};

pub struct PrintContext<'input> {
    hash: FileHash<'input>,
    code: Option<Code<'input>>,
    index: PrintIndex,
    options: Options,
}

impl<'input> PrintContext<'input> {
    pub fn new(file: &'input File<'input>, options: Options) -> Self {
        let hash = FileHash::new(file);
        let code = Code::new(file);
        let index = PrintIndex::new(file, &options);
        PrintContext {
            hash,
            code,
            index,
            options,
        }
    }

    pub fn print(&self, printer: &mut dyn Printer) -> Result<()> {
        let mut state = PrintState::new(printer, &self.hash, self.code.as_ref(), self.options());
        print::file::print(&mut state)
    }

    pub fn print_id(
        &self,
        id: usize,
        detail: Option<&str>,
        printer: &mut dyn Printer,
    ) -> Option<()> {
        let mut state = PrintState::new(printer, &self.hash, self.code.as_ref(), self.options());
        print::id::print_id(self.index.get(id)?, detail, &mut state)
    }

    pub fn options(&self) -> &Options {
        &self.options
    }

    pub fn parent(&self, id: usize) -> Option<usize> {
        self.index.parent(id, self.hash.file)
    }
}

pub struct DiffContext<'input> {
    hash_a: FileHash<'input>,
    hash_b: FileHash<'input>,
    code_a: Option<Code<'input>>,
    code_b: Option<Code<'input>>,
    index: DiffIndex,
    options: Options,
}

impl<'input> DiffContext<'input> {
    pub fn new(
        file_a: &'input File<'input>,
        file_b: &'input File<'input>,
        options: Options,
    ) -> Self {
        let hash_a = FileHash::new(file_a);
        let hash_b = FileHash::new(file_b);
        let code_a = Code::new(file_a);
        let code_b = Code::new(file_b);
        let index = DiffIndex::new(&hash_a, &hash_b, &options);
        DiffContext {
            hash_a,
            hash_b,
            code_a,
            code_b,
            index,
            options,
        }
    }

    pub fn print(&self, printer: &mut dyn Printer) -> Result<()> {
        let mut state = DiffState::new(
            printer,
            &self.hash_a,
            &self.hash_b,
            self.code_a.as_ref(),
            self.code_b.as_ref(),
            self.options(),
        );
        print::file::diff(&mut state)
    }

    pub fn print_id(
        &self,
        id: usize,
        detail: Option<&str>,
        printer: &mut dyn Printer,
    ) -> Option<()> {
        let mut state = DiffState::new(
            printer,
            &self.hash_a,
            &self.hash_b,
            self.code_a.as_ref(),
            self.code_b.as_ref(),
            self.options(),
        );
        print::id::diff_id(self.index.get(id)?, detail, &mut state)
    }

    pub fn options(&self) -> &Options {
        &self.options
    }

    pub fn parent(&self, id: usize) -> Option<usize> {
        self.index.parent(id, self.hash_a.file, self.hash_b.file)
    }
}

pub struct BloatContext<'input> {
    hash: FileHash<'input>,
    index: PrintIndex,
    bloat: BloatIndex,
    options: Options,
}

impl<'input> BloatContext<'input> {
    pub fn new(file: &'input File<'input>, options: Options) -> Self {
        let hash = FileHash::new(file);
        let index = PrintIndex::new(file, &options);
        let bloat = BloatIndex::new(file);
        BloatContext {
            hash,
            index,
            bloat,
            options,
        }
    }

    pub fn print(&self, printer: &mut dyn Printer) -> Result<()> {
        let mut state = PrintState::new(printer, &self.hash, None, self.options());
        self.bloat.print(&mut state)
    }

    pub fn print_id(&self, id: usize, printer: &mut dyn Printer) -> Option<()> {
        let mut state = PrintState::new(printer, &self.hash, None, self.options());
        self.bloat.print_id(self.index.get(id)?, &mut state)
    }

    pub fn options(&self) -> &Options {
        &self.options
    }

    pub fn parent(&self, id: usize) -> Option<usize> {
        self.index.parent(id, self.hash.file)
    }
}
