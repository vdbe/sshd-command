use assert_cmd::Command;

fn cmd() -> Command {
    Command::cargo_bin("sshd-command").expect("binary exists")
}
#[cfg(test)]
mod happy_path {
    use super::*;
    use predicates::prelude::predicate;

    #[test]
    fn argument_help() {
        let mut cmd = cmd();
        cmd.arg("--help");
        cmd.assert()
            .success()
            .stdout(predicate::str::contains("USAGE"));
    }

    #[test]
    fn argument_version() {
        let mut cmd = cmd();
        cmd.arg("-V");
        cmd.assert().success();
    }

    #[test]
    fn validate_principals() {
        let mut cmd = cmd();
        cmd.args(["--validate", "tests/fixtures/happy/principals.tera"]);
        cmd.assert()
            .success()
            .stdout(predicates::str::is_empty())
            .stderr(predicates::str::is_empty());
    }

    #[test]
    fn check_principals() {
        let mut cmd = cmd();
        cmd.args(["--check", "tests/fixtures/happy/principals.tera"]);
        cmd.assert()
            .success()
            .stdout(predicates::str::is_empty())
            .stderr(predicates::str::is_empty());
    }

    #[test]
    fn output_principals() {
        let mut cmd = cmd();
        cmd.args(["tests/fixtures/happy/principals.tera", "1000", "user"]);
        cmd.assert()
            .success()
            .stdout(include_str!("fixtures/happy/principals.out"))
            .stderr(predicates::str::is_empty());
    }

    #[test]
    fn output_json_principals() {
        let mut cmd = cmd();
        cmd.args([
            "tests/fixtures/happy/json-principals.tera",
            "1000",
            "user",
        ]);
        cmd.assert()
            .success()
            .stdout(include_str!("fixtures/happy/principals.out"))
            .stderr(predicates::str::is_empty());
    }
}

#[cfg(test)]
mod sad_path {
    use super::*;
    use predicates::prelude::predicate;

    #[test]
    fn nonexistent_template_arg() {
        cmd()
            .assert()
            .failure()
            .stderr(predicate::str::contains("No template path provided"));
    }

    #[test]
    fn nonexistent_template_path() {
        let mut cmd = cmd();
        cmd.arg("test/file/doesnt/exist");
        cmd.assert()
            .failure()
            .stderr(predicate::str::contains("No such file or directory"));
    }

    #[test]
    fn unsupported_token() {
        let mut cmd = cmd();
        cmd.args(["--validate", "tests/fixtures/sad/unsupported-token.tera"]);
        cmd.assert().failure();
    }

    #[test]
    fn missing_token_complete_user() {
        let mut cmd = cmd();
        cmd.args([
            "--validate",
            "tests/fixtures/sad/missing-token-complete-user.tera",
        ]);
        cmd.assert().failure().stderr(predicate::str::contains(
            "token required for `complete_user",
        ));
    }

    #[test]
    fn missing_tera_context() {
        let mut cmd = cmd();
        cmd.args(["--check", "tests/fixtures/sad/missing-context.tera"]);
        cmd.assert().failure().stderr(predicate::str::contains(
            "Variable `doest_not_exist` not found",
        ));
    }

    #[test]
    fn missing_validate_and_check() {
        let mut cmd1 = cmd();
        cmd1.args(["--validate", "tests/fixtures/sad/missing-context.tera"]);
        cmd1.assert().success();

        let mut cmd2 = cmd();
        cmd2.args([
            "--validate",
            "--check",
            "tests/fixtures/sad/missing-context.tera",
        ]);
        cmd2.assert().failure();
    }
}
