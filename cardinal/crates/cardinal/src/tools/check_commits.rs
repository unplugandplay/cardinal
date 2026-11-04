//! `check-commits <base> <head>`: validate the message of every non-merge commit of `<base>..<head>` as
//! `commit-check` does. Usage: `just check-commits <base> <head>` (the CI job on pull requests).

use std::process::Command;

use super::commit_check::{attribution_banned, scopes, violations};

pub(super) fn main(args: &[String]) -> u8 {
    let [base, head] = args else {
        eprintln!("usage: cardinal run check-commits <base> <head>");
        return 2;
    };
    let scopes = match scopes() {
        Ok(scopes) => scopes,
        Err(e) => {
            eprintln!("error: {e}");
            return 2;
        }
    };
    let git = |args: &[&str]| Command::new("git").args(args).output();
    let out = match git(&["rev-list", "--no-merges", &format!("{base}..{head}")]) {
        Ok(out) if out.status.success() => out,
        Ok(out) => {
            eprint!("{}", String::from_utf8_lossy(&out.stderr));
            return 1;
        }
        Err(e) => {
            eprintln!("error: cannot run git: {e}");
            return 2;
        }
    };
    let attribution = attribution_banned();
    let mut failed = false;
    for rev in String::from_utf8_lossy(&out.stdout).lines() {
        println!("== {rev}");
        let message = match git(&["log", "-1", "--format=%B", rev]) {
            Ok(out) if out.status.success() => String::from_utf8_lossy(&out.stdout).into_owned(),
            _ => {
                println!("cannot read the message of {rev}");
                failed = true;
                continue;
            }
        };
        let errors = violations(&message, scopes.as_deref(), attribution);
        for e in &errors {
            println!("{e}");
        }
        failed |= !errors.is_empty();
    }
    u8::from(failed)
}
