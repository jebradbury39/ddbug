use parser::{FileHash, Register};

use crate::Result;
use crate::print::ValuePrinter;

pub(crate) fn print(register: Register, w: &mut dyn ValuePrinter, hash: &FileHash) -> Result<()> {
    match register.name(hash) {
        Some(name) => write!(w, "{}", name)?,
        None => write!(w, "r{}", register.0)?,
    };
    Ok(())
}
