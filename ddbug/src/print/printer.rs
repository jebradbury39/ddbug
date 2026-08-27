use std::io::Write;

use crate::Result;

#[derive(Debug, Clone, Copy, Eq, PartialEq)]
pub enum DiffPrefix {
    None,
    Equal,
    Delete,
    Add,
    Modify,
}

pub trait Printer {
    fn value(
        &mut self,
        buf: &mut Vec<u8>,
        f: &mut dyn FnMut(&mut dyn ValuePrinter) -> Result<()>,
    ) -> Result<()>;

    /// Calls `f` to write to an internal buffer.
    ///
    /// This is used for items that may need to be omitted if empty.
    ///
    /// Returns true if anything was written to the buffer.
    fn buffer(&mut self, f: &mut dyn FnMut(&mut dyn Printer) -> Result<()>) -> Result<bool>;
    /// Writes the internal buffer created by the previous `buffer` or `indent_body` call.
    fn write_buf(&mut self) -> Result<()>;

    fn line_break(&mut self) -> Result<()>;

    fn line(&mut self, label: &str, buf: &[u8]) -> Result<()>;
    fn line_diff(&mut self, label: &str, a: &[u8], b: &[u8]) -> Result<()>;

    /// Calls `body` to write an indented body to an internal buffer.
    ///
    /// This is used for items that may need to be omitted if empty.
    ///
    /// Returns true if anything was written to the buffer.
    fn indent_body(&mut self, body: &mut dyn FnMut(&mut dyn Printer) -> Result<()>)
    -> Result<bool>;
    /// Calls `header` to write the header, followed by the internal buffer created by
    /// the previous `indent_body` call.
    fn indent_header(
        &mut self,
        collapsed: bool,
        header: &mut dyn FnMut(&mut dyn Printer) -> Result<()>,
    ) -> Result<()>;
    /// Calls `header` and `body` to write the output for an item with the given `id`.
    fn indent_id(
        &mut self,
        id: usize,
        header: &mut dyn FnMut(&mut dyn Printer) -> Result<()>,
        body: &mut dyn FnMut(&mut dyn Printer) -> Result<()>,
    ) -> Result<()>;

    /// Optionally prints a labelled reference to a block of detail information.
    ///
    /// `detail` is the parameter for [`crate::print_id`], with the `id` being
    /// determined by nesting.
    ///
    /// Returns false if the printer does not support lazy detail information.
    fn indent_detail(&mut self, detail: &str, label: &str) -> Result<bool>;

    fn prefix(&mut self, prefix: DiffPrefix);
    fn get_prefix(&self) -> DiffPrefix;

    fn instruction(&mut self, address: Option<u64>, mnemonic: &str, buf: &[u8]) -> Result<()>;
}

pub trait ValuePrinter: Write {
    fn link(
        &mut self,
        id: usize,
        f: &mut dyn FnMut(&mut dyn ValuePrinter) -> Result<()>,
    ) -> Result<()>;

    fn name(&mut self, name: &str) -> Result<()>;
}

impl ValuePrinter for Vec<u8> {
    fn link(
        &mut self,
        _id: usize,
        f: &mut dyn FnMut(&mut dyn ValuePrinter) -> Result<()>,
    ) -> Result<()> {
        f(self)
    }

    fn name(&mut self, name: &str) -> Result<()> {
        self.write_all(name.as_bytes())?;
        Ok(())
    }
}
