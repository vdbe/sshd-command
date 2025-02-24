use std::{
    env,
    error::Error,
    fs::File,
    io::{self, BufReader},
    process::ExitCode,
};

use sshd_command::frontmatter::FrontMatter;

fn main() -> Result<ExitCode, Box<dyn Error>> {
    let mut args = env::args().skip(1);

    let template_path = args.next().ok_or("No template path provided")?;
    let template = File::open(&template_path)?;

    // Validate the frontmatter, this can be done without requiring the token
    // values
    let mut args = args.peekable();
    if args.next_if(|a| a == "--validate").is_some() {
        let mut reader = BufReader::new(template);
        FrontMatter::parse(&mut reader)?.validate()?;
        return Ok(ExitCode::SUCCESS);
    }

    if let Err(err) = sshd_command::render_to(
        &mut io::stdout(),
        args,
        &template_path,
        template,
    ) {
        eprintln!("Error: {err}");
        // TODO: impl source
        if let Some(source) = err.source() {
            eprintln!("Caused by: {source}");
        }

        return Ok(ExitCode::FAILURE);
    };

    Ok(ExitCode::SUCCESS)
}
