//! Running another program and waiting for what it says.

use std::io;
use std::process::{Command, Output};
use std::time::Duration;

/// How many times a program that is momentarily busy is asked again.
const BUSY_TRIES: u32 = 10;

/// Run it to the end, asking again while the executable is busy.
///
/// Linux refuses to start a file that some process still holds open for
/// writing, and that is briefly true of a file just written whenever another
/// thread forks at the same moment: the child holds the descriptor until it
/// execs. Tests that write a stand-in `codex` or `gh` hit it under parallel
/// runs, and so would a real one being replaced by an upgrade. It clears in
/// milliseconds, so a short wait is the whole fix.
pub fn output(command: &mut Command) -> io::Result<Output> {
    let mut tries = 1;
    loop {
        match command.output() {
            Err(e) if e.kind() == io::ErrorKind::ExecutableFileBusy && tries < BUSY_TRIES => {
                tries += 1;
                std::thread::sleep(Duration::from_millis(10 * u64::from(tries)));
            }
            answer => return answer,
        }
    }
}
