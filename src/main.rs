use std::{
    env,
    error::Error,
    fs::File,
    io::{self},
    process::ExitCode,
};

fn main() -> Result<ExitCode, Box<dyn Error>> {
    let mut args = env::args().skip(1);

    let template_path = args.next().ok_or("No template path provided")?;
    let template = File::open(&template_path)?;

    if let Err(err) =
        sshd_command::main(&mut io::stdout(), args, &template_path, template)
    {
        eprintln!("Error: {err}");
        // TODO: impl source
        if let Some(source) = err.source() {
            eprintln!("Caused by: {source}");
        }

        return Ok(ExitCode::FAILURE);
    };

    Ok(ExitCode::SUCCESS)
}
