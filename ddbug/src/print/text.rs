use std::io::Write;

use super::{DiffPrefix, Printer, ValuePrinter};
use crate::Result;

pub struct TextPrinter<'w> {
    w: &'w mut dyn Write,
    buffer: Vec<u8>,
    indent: usize,
    prefix: DiffPrefix,
}

impl<'w> TextPrinter<'w> {
    pub fn new(w: &'w mut dyn Write) -> Self {
        TextPrinter {
            w,
            buffer: Vec::new(),
            indent: 0,
            prefix: DiffPrefix::None,
        }
    }

    fn write_indent(&mut self) -> Result<()> {
        match self.prefix {
            DiffPrefix::None => {}
            DiffPrefix::Equal | DiffPrefix::Modify => write!(self.w, "  ")?,
            DiffPrefix::Delete => {
                write!(self.w, "- ")?;
            }
            DiffPrefix::Add => {
                write!(self.w, "+ ")?;
            }
        }
        for _ in 0..self.indent {
            write!(self.w, "\t")?;
        }
        Ok(())
    }

    fn buffer_impl(
        &mut self,
        indent: usize,
        f: &mut dyn FnMut(&mut dyn Printer) -> Result<()>,
    ) -> Result<bool> {
        self.buffer.clear();
        let mut p = TextPrinter {
            w: &mut self.buffer,
            buffer: Vec::new(),
            indent,
            prefix: self.prefix,
        };
        f(&mut p)?;
        Ok(!self.buffer.is_empty())
    }
}

impl<'w> Printer for TextPrinter<'w> {
    fn value(
        &mut self,
        buf: &mut Vec<u8>,
        f: &mut dyn FnMut(&mut dyn ValuePrinter) -> Result<()>,
    ) -> Result<()> {
        let mut p = TextValuePrinter { w: buf };
        f(&mut p)
    }

    fn buffer(&mut self, f: &mut dyn FnMut(&mut dyn Printer) -> Result<()>) -> Result<bool> {
        self.buffer_impl(self.indent, f)
    }

    fn write_buf(&mut self) -> Result<()> {
        self.w.write_all(&self.buffer)?;
        Ok(())
    }

    fn line_break(&mut self) -> Result<()> {
        writeln!(self.w).map_err(From::from)
    }

    fn line(&mut self, label: &str, buf: &[u8]) -> Result<()> {
        self.write_indent()?;
        if !label.is_empty() {
            write!(self.w, "{}:", label)?;
            if !buf.is_empty() {
                write!(self.w, " ")?;
            }
        }
        self.w.write_all(buf)?;
        writeln!(self.w)?;
        Ok(())
    }

    fn line_diff(&mut self, label: &str, a: &[u8], b: &[u8]) -> Result<()> {
        self.prefix = DiffPrefix::Delete;
        self.line(label, a)?;
        self.prefix = DiffPrefix::Add;
        self.line(label, b)
    }

    fn indent_body(
        &mut self,
        body: &mut dyn FnMut(&mut dyn Printer) -> Result<()>,
    ) -> Result<bool> {
        self.buffer_impl(self.indent + 1, body)
    }

    fn indent_header(
        &mut self,
        _collapsed: bool,
        header: &mut dyn FnMut(&mut dyn Printer) -> Result<()>,
    ) -> Result<()> {
        header(self)?;
        self.write_buf()
    }

    fn indent_id(
        &mut self,
        _id: usize,
        header: &mut dyn FnMut(&mut dyn Printer) -> Result<()>,
        body: &mut dyn FnMut(&mut dyn Printer) -> Result<()>,
    ) -> Result<()> {
        header(self)?;
        self.indent += 1;
        let r = body(self);
        self.indent -= 1;
        r
    }

    fn indent_detail(&mut self, _detail: &str, _label: &str) -> Result<bool> {
        Ok(false)
    }

    fn prefix(&mut self, prefix: DiffPrefix) {
        self.prefix = prefix;
    }

    fn get_prefix(&self) -> DiffPrefix {
        self.prefix
    }

    fn instructions(&mut self, f: &mut dyn FnMut(&mut dyn Printer) -> Result<()>) -> Result<()> {
        f(self)
    }

    fn instruction(&mut self, address: Option<u64>, mnemonic: &str, buf: &[u8]) -> Result<()> {
        self.write_indent()?;
        if let Some(address) = address {
            write!(self.w, "{:3x}:  ", address)?;
        } else {
            write!(self.w, "{:3}   ", "")?;
        }
        if mnemonic.is_empty() {
            // When caller doesn't specify a mnemonic, the operands don't a leading space,
            // so add one here.
            // TODO: fix this in callers instead?
            write!(self.w, "{:6} ", "")?;
        } else {
            write!(self.w, "{:6}", mnemonic)?;
        }
        if !buf.is_empty() {
            write!(self.w, " ")?;
            self.w.write_all(buf)?;
        }
        writeln!(self.w)?;
        Ok(())
    }
}

struct TextValuePrinter<'w> {
    w: &'w mut Vec<u8>,
}

impl<'w> Write for TextValuePrinter<'w> {
    fn write(&mut self, buf: &[u8]) -> std::result::Result<usize, std::io::Error> {
        self.w.write(buf)
    }

    fn flush(&mut self) -> std::result::Result<(), std::io::Error> {
        self.w.flush()
    }
}

impl<'w> ValuePrinter for TextValuePrinter<'w> {
    fn link(
        &mut self,
        _id: usize,
        f: &mut dyn FnMut(&mut dyn ValuePrinter) -> Result<()>,
    ) -> Result<()> {
        f(self)
    }

    fn name(&mut self, name: &str) -> Result<()> {
        self.w.write_all(name.as_bytes())?;
        Ok(())
    }
}
