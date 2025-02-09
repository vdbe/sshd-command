use std::{env, io, process::ExitCode};

fn main() -> ExitCode {
    let args = env::args().skip(1);
    let writer = io::stdout();

    if let Err(err) = sshd_command::main(args, &writer) {
        eprintln!("Error: {err}");
        // TODO: impl source
        if let Some(source) = err.source() {
            eprintln!("Caused by: {source}");
        }

        return ExitCode::FAILURE;
    };

    ExitCode::SUCCESS
}
