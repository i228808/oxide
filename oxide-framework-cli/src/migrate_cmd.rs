//! Stub for future database migrations.

use anyhow::Result;

pub fn run(args: &[String]) -> Result<()> {
    eprintln!(
        "`oxide migrate` is not implemented yet.\n\
         When an ORM or migration tool is integrated, this will run migrations.\n\
         Received arguments: {:?}",
        args
    );
    Ok(())
}
