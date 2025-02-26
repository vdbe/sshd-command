use std::{
    env,
    error::Error,
    fs::File,
    io::{self, BufReader},
    process::ExitCode,
};

use sshd_command::{crate_version, frontmatter::FrontMatter};

fn main() -> Result<ExitCode, Box<dyn Error>> {
    let mut args = env::args().skip(1).peekable();

    'flags: while let Some(arg) = args.next_if(|a| a.starts_with('-')).as_ref()
    {
        match arg.as_str() {
            "-h" | "--help" => {
                print!(
                    "\
{} {}
{}

USAGE:
    sshd-command [FLAGS] [template]

ARGS:
    <template>    Sets the template file to use

FLAGS:
    -h, --help                     Prints help information
    -v, --validate <template>      Validate the template front matter
    -V, --version                  Prints version information
",
                    env!("CARGO_PKG_NAME"),
                    crate_version(),
                    env!("CARGO_PKG_DESCRIPTION"),
                );

                return Ok(ExitCode::FAILURE);
            }
            "-v" | "--validate" => {
                // Validate the frontmatter, this can be done without requiring
                // the token values
                let template_path =
                    args.next().ok_or("No template path provided")?;
                let template = File::open(&template_path)?;

                let mut reader = BufReader::new(template);
                FrontMatter::parse(&mut reader)?.validate()?;

                return Ok(ExitCode::SUCCESS);
            }

            "-V" | "--version" => {
                println!("{} {}", env!("CARGO_PKG_NAME"), crate_version());

                return Ok(ExitCode::SUCCESS);
            }
            "--" => break 'flags,
            _ => {}
        }
    }

    let template_path = args.next().ok_or("No template path provided")?;
    let template = File::open(&template_path)?;
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
