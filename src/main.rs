use std::{
    env,
    error::Error,
    fs::File,
    io::{self, BufReader, Seek, Write},
    process::ExitCode,
};

use sshd_command::{crate_version, frontmatter::FrontMatter, Token};

fn main() -> Result<ExitCode, Box<dyn Error>> {
    let mut args = env::args().skip(1).peekable();
    let mut check_arg = false;
    let mut validate_arg = false;

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
    -c, --check <template>         Check the template (superset of validate)
    -V, --version                  Prints version information
",
                    env!("CARGO_PKG_NAME"),
                    crate_version(),
                    env!("CARGO_PKG_DESCRIPTION"),
                );

                return Ok(ExitCode::SUCCESS);
            }
            "-v" | "--validate" => {
                validate_arg = true;
            }
            "-c" | "--check" => {
                check_arg = true;
            }
            "-V" | "--version" => {
                println!("{} {}", env!("CARGO_PKG_NAME"), crate_version());

                return Ok(ExitCode::SUCCESS);
            }
            "--" => break 'flags,
            _ => {}
        }
    }

    // No need to validate separately since it done inside `render_to`.
    validate_arg = validate_arg && !check_arg;

    let template_path = args.next().ok_or("No template path provided")?;
    let template = File::open(&template_path)?;
    let mut reader = BufReader::new(template);

    if validate_arg {
        FrontMatter::parse(&mut reader)?.validate()?;

        return Ok(ExitCode::SUCCESS);
    }

    #[expect(clippy::if_not_else)]
    let (writer, args): (
        &mut dyn Write,
        &mut dyn Iterator<Item = String>,
    ) = if !check_arg {
        (&mut io::stdout(), &mut args)
    } else {
        let front_matter = FrontMatter::parse(&mut reader)?;
        front_matter.validate()?;

        let placeholder_args = Token::get_template_args(front_matter.tokens());

        // Rewind reader
        _ = reader.seek(io::SeekFrom::Start(0))?;

        (&mut io::empty(), &mut args.chain(placeholder_args))
    };

    if let Err(err) =
        sshd_command::render_to(writer, args, &template_path, reader)
    {
        print_error_chain(&err);

        return Ok(ExitCode::FAILURE);
    };

    Ok(ExitCode::SUCCESS)
}

fn print_error_chain(mut err: &dyn Error) {
    eprintln!("Error: {err}");

    while let Some(source) = err.source() {
        eprintln!("Caused by: {source}");
        err = source;
    }
}
