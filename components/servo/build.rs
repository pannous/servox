/* This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/. */

use std::process::Command;

fn main() {
    // GStreamer support has been removed
}

/// Tries to find a suitable python, which in Servo is always `uv run python` unless we are running
/// as a descendant of `uv run python`. In that case, we can use either `uv run python` or `python`
/// (uv does not provide a `python3` on Windows).
///
/// More details: <https://book.servo.org/hacking/setting-up-your-environment.html#check-tools>
///
/// Note: This function should be kept in sync with the version in `components/script/build.rs`
fn find_python() -> Command {
    let mut command = Command::new("uv");
    command.args(["run", "--no-project", "python"]);

    if command.output().is_ok_and(|out| out.status.success()) {
        return command;
    }

    panic!("Can't find python (tried `{command:?}`)! Is uv installed and in PATH?")
}
