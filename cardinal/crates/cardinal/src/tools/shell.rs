//! The commands of a shell line as the shell reads them, and whether one stages every file (REV-21 of
//! `specs/review-gate.md`, BRE-21 of `specs/rule-breaches.md`): shared by the `review-gate check` hook and
//! `rule-breaches`, so the gate and the measure agree on what a breach is. A reading of the text only: the gates catch
//! the mistakes of a cooperative agent, not deliberate bypasses (ADR 0005).

/// Global options of `git` that take the next word as value.
pub(super) const GIT_VALUE_OPTIONS: [&str; 7] = [
    "-C",
    "-c",
    "--git-dir",
    "--work-tree",
    "--namespace",
    "--exec-path",
    "--super-prefix",
];

/// The characters that end a command of a line, besides the newline.
const SEPARATORS: &str = ";&|()`";

/// The commands of a shell line, each as its words: the content of a quoted string is dropped (the string stays one
/// word, empty when it stood alone), as are the body of a here-document (opened by `<<` outside quotes with a
/// terminator starting with a letter or `_`), a comment (`#` starting a word) and the backslash of an escape; a
/// continued line is joined. So a commit message that names a command runs none.
pub(super) fn commands(line: &str) -> Vec<Vec<String>> {
    let (mut commands, mut command, mut word) = (Vec::new(), Vec::new(), None::<String>);
    let mut quote = None;
    let joined = line.replace("\\\n", " ");
    let mut lines = joined.split('\n');
    while let Some(line) = lines.next() {
        // the terminator of a here-document opened on this line, outside quotes; its body is the next lines
        let mut terminator: Option<String> = None;
        let mut chars = line.chars().peekable();
        while let Some(c) = chars.next() {
            match quote {
                Some('"') if c == '\\' => {
                    chars.next();
                }
                Some(q) if c == q => quote = None,
                Some(_) => {}
                None if c == '\'' || c == '"' => {
                    quote = Some(c);
                    word.get_or_insert_with(String::new);
                }
                None if c == '\\' => {
                    let escaped = chars.next();
                    word.get_or_insert_with(String::new).extend(escaped);
                }
                None if c == '#' && word.is_none() => break,
                None if c == '<' && chars.peek() == Some(&'<') => {
                    chars.next();
                    let name: String = chars
                        .clone()
                        .skip_while(|c| *c == '-' || c.is_whitespace())
                        .filter(|c| !matches!(c, '\'' | '"'))
                        .take_while(|c| {
                            !c.is_whitespace() && !SEPARATORS.contains(*c) && *c != '<' && *c != '>'
                        })
                        .collect();
                    terminator = terminator.or(name
                        .starts_with(|c: char| c.is_alphabetic() || c == '_')
                        .then_some(name));
                    command.extend(word.take());
                }
                None if SEPARATORS.contains(c) => {
                    command.extend(word.take());
                    commands.extend((!command.is_empty()).then(|| std::mem::take(&mut command)));
                }
                None if c.is_whitespace() || c == '<' || c == '>' => command.extend(word.take()),
                None => word.get_or_insert_with(String::new).push(c),
            }
        }
        if quote.is_none() {
            command.extend(word.take());
            commands.extend((!command.is_empty()).then(|| std::mem::take(&mut command)));
        }
        if let Some(end) = terminator.filter(|_| quote.is_none()) {
            lines.by_ref().find(|l| l.trim() == end);
        }
    }
    command.extend(word);
    commands.extend((!command.is_empty()).then_some(command));
    commands
}

/// Each `git` invocation of a command: its subcommand, the first word after `git` that is not a global option
/// (`GIT_VALUE_OPTIONS` taking the next word), with the words after it and the directory of its last `-C`.
pub(super) fn git_invocations(
    command: &[String],
) -> impl Iterator<Item = (&str, &[String], Option<&str>)> {
    let is_git = |w: &String| w.rsplit('/').next() == Some("git");
    (0..command.len())
        .filter(move |i| is_git(&command[*i]))
        .filter_map(move |at| {
            let (mut i, mut directory) = (at + 1, None);
            while let Some(word) = command.get(i) {
                if GIT_VALUE_OPTIONS.contains(&word.as_str()) {
                    if word == "-C" {
                        directory = command.get(i + 1).map(String::as_str);
                    }
                    i += 2;
                } else if word.starts_with('-') {
                    i += 1;
                } else {
                    return Some((word.as_str(), &command[i + 1..], directory));
                }
            }
            None
        })
}

/// Whether a `git add` of the `commands` of a line stages every file: `-A` (alone or in a bundle of short options), `--all`, or a
/// pathspec of the whole tree (`.`, `./`, `:/`), unless it is a dry run (`-n`, `--dry-run`).
pub(super) fn stages_everything(commands: &[Vec<String>]) -> bool {
    commands.iter().any(|command| {
        git_invocations(command).any(|(subcommand, args, _)| {
            let short = |a: &str, flag: char| {
                a.strip_prefix('-')
                    .is_some_and(|s| !s.starts_with('-') && s.contains(flag))
            };
            subcommand == "add"
                && !args.iter().any(|a| short(a, 'n') || a == "--dry-run")
                && args
                    .iter()
                    .any(|a| short(a, 'A') || matches!(a.as_str(), "--all" | "." | "./" | ":/"))
        })
    })
}
