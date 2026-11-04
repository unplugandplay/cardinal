//! The entries of the project's `.claude/settings.json` that the source manages (`specs/sync.md`, SYN-29 to SYN-36,
//! ADR 0007, ADR 0013): the allow, ask and deny rules and hook objects of `entries/.claude/settings.json`, added to the settings by `sync` and
//! recorded in the lock, every other key left as the project wrote it.

use std::fmt;
use std::fs;
use std::io::ErrorKind;
use std::path::Path;

use anyhow::{bail, Context, Result};
use serde_json::{json, Map, Value};

use crate::Outcome;

/// The settings of the project, relative to its root.
pub const SETTINGS: &str = ".claude/settings.json";
/// The managed entries, relative to the source.
pub const FRAGMENT: &str = "entries/.claude/settings.json";

/// The entry of a rule of a list of `permissions`.
type Rule = fn(String) -> Entry;

/// The lists of `permissions` whose rules are managed entries, with the entry of one of their rules.
const PERMISSIONS: [(&str, Rule); 3] = [
    ("allow", Entry::Allow),
    ("ask", Entry::Ask),
    ("deny", Entry::Deny),
];

/// One managed entry: a rule of `permissions.allow`, `permissions.ask` or `permissions.deny`, or a hook object of an
/// event and matcher of `hooks`.
#[derive(Clone, Debug, PartialEq)]
pub enum Entry {
    Allow(String),
    Ask(String),
    Deny(String),
    Hook {
        event: String,
        matcher: Option<String>,
        hook: Value,
    },
}

impl fmt::Display for Entry {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Entry::Allow(rule) | Entry::Ask(rule) | Entry::Deny(rule) => {
                write!(f, "{} {rule}", self.list())
            }
            Entry::Hook {
                event,
                matcher,
                hook,
            } => {
                write!(f, "hook {event} ")?;
                if let Some(matcher) = matcher {
                    write!(f, "{matcher} ")?;
                }
                match hook["command"].as_str() {
                    Some(command) => f.write_str(command),
                    None => write!(f, "{hook}"),
                }
            }
        }
    }
}

impl Entry {
    /// The list of `permissions` that holds the entry, `hooks` for a hook.
    fn list(&self) -> &'static str {
        match self {
            Entry::Allow(_) => "allow",
            Entry::Ask(_) => "ask",
            Entry::Deny(_) => "deny",
            Entry::Hook { .. } => "hooks",
        }
    }

    /// The entry as one line of JSON, for the lock.
    pub fn to_lock(&self) -> String {
        match self {
            Entry::Allow(rule) | Entry::Ask(rule) | Entry::Deny(rule) => {
                json!({ self.list(): rule })
            }
            Entry::Hook {
                event,
                matcher,
                hook,
            } => json!({ "event": event, "matcher": matcher, "hook": hook }),
        }
        .to_string()
    }

    /// The entry of a line written by `to_lock`.
    pub fn from_lock(line: &str) -> Result<Entry> {
        let value: Value = serde_json::from_str(line)?;
        for (list, entry) in PERMISSIONS {
            if let Some(rule) = value[list].as_str() {
                return Ok(entry(rule.to_string()));
            }
        }
        match (&value["event"], &value["matcher"], &value["hook"]) {
            (Value::String(event), Value::String(_) | Value::Null, hook @ Value::Object(_)) => {
                Ok(Entry::Hook {
                    event: event.clone(),
                    matcher: value["matcher"].as_str().map(String::from),
                    hook: hook.clone(),
                })
            }
            _ => bail!("not a settings entry: {line}"),
        }
    }

    /// The command a hook runs, `None` for a rule.
    pub fn command(&self) -> Option<&str> {
        match self {
            Entry::Hook { hook, .. } => hook["command"].as_str(),
            _ => None,
        }
    }

    /// The report line of the entry, `.claude/settings.json <entry>` (SYN-34).
    pub fn label(&self) -> String {
        format!("{SETTINGS} {self}")
    }
}

/// The entries the source manages, its allow rules, ask rules, deny rules then hooks, each in the order of its fragment; none when it has no fragment. An error when the
/// fragment is not in the shape of SYN-29.
pub fn of_source(source: &Path) -> Result<Vec<Entry>> {
    let path = source.join(FRAGMENT);
    let text = match fs::read_to_string(&path) {
        Err(e) if e.kind() == ErrorKind::NotFound => return Ok(Vec::new()),
        other => other.with_context(|| format!("reading {}", path.display()))?,
    };
    let fragment: Value =
        serde_json::from_str(&text).with_context(|| format!("parsing {}", path.display()))?;
    entries(&fragment)
        .with_context(|| format!("{}: not in the shape of the settings", path.display()))
}

/// Every allow, ask and deny rule and hook of `settings`, or why its shape is not the one of SYN-29.
fn entries(settings: &Value) -> Result<Vec<Entry>> {
    let Some(settings) = settings.as_object() else {
        bail!("not an object");
    };
    let mut entries = Vec::new();
    if let Some(permissions) = settings.get("permissions") {
        let Some(permissions) = permissions.as_object() else {
            bail!("`permissions` is not an object");
        };
        for (list, entry) in PERMISSIONS {
            let Some(rules) = permissions.get(list) else {
                continue;
            };
            for rule in rules
                .as_array()
                .with_context(|| format!("`permissions.{list}` is not an array"))?
            {
                let rule = rule
                    .as_str()
                    .with_context(|| format!("a rule of `permissions.{list}` is not a string"))?;
                entries.push(entry(rule.to_string()));
            }
        }
    }
    if let Some(hooks) = settings.get("hooks") {
        for (event, groups) in hooks.as_object().context("`hooks` is not an object")? {
            for group in groups
                .as_array()
                .with_context(|| format!("`hooks.{event}` is not an array"))?
            {
                let matcher = match &group["matcher"] {
                    Value::Null => None,
                    Value::String(matcher) => Some(matcher.clone()),
                    _ => bail!("a matcher of `hooks.{event}` is not a string"),
                };
                let list = group["hooks"]
                    .as_array()
                    .with_context(|| format!("a group of `hooks.{event}` has no `hooks` array"))?;
                for hook in list {
                    entries.push(Entry::Hook {
                        event: event.clone(),
                        matcher: matcher.clone(),
                        hook: hook.clone(),
                    });
                }
            }
        }
    }
    Ok(entries)
}

/// What the merge of the managed entries into the project's settings gives.
pub struct Merge {
    /// The settings as the sync leaves them, `None` when there are none it can merge into.
    pub settings: Option<Value>,
    /// The text to write, `None` when the file stays as it is.
    pub write: Option<String>,
    /// The entries for the lock.
    pub lock: Vec<Entry>,
    /// One line per entry added, removed or in conflict, labelled as SYN-34 says.
    pub outcomes: Vec<(String, Outcome)>,
    /// Why the settings were left alone, when they were not merged although they exist.
    pub warning: Option<String>,
}

/// The merge of `wanted` into the settings `text` (`None`: no settings), the lock having `recorded`; `force` adds back
/// the entries the project removed or edited.
pub fn merge(text: Option<&str>, wanted: &[Entry], recorded: &[Entry], force: bool) -> Merge {
    let left_alone = |settings, warning| Merge {
        settings,
        write: None,
        lock: recorded.to_vec(),
        outcomes: Vec::new(),
        warning,
    };
    let Some(text) = text else {
        return left_alone(None, None);
    };
    let Ok(mut settings) = serde_json::from_str::<Value>(text) else {
        // the warnings on the settings already say it is not valid JSON
        return left_alone(None, None);
    };
    if let Err(e) = entries(&settings) {
        let warning = format!(
            "warning: {SETTINGS} is not in the shape of Claude Code settings ({e}); its managed entries are not synced"
        );
        return left_alone(Some(settings), Some(warning));
    }
    let mut outcomes = Vec::new();
    let mut changed = false;
    // an entry dropped by the source goes when the settings hold it unchanged (SYN-31): a hook the source changed
    // takes its place, the first new hook of its event and matcher
    let mut new: Vec<&Entry> = wanted
        .iter()
        .filter(|entry| !recorded.contains(entry) && !holds(&settings, entry))
        .collect();
    for entry in recorded.iter().filter(|entry| !wanted.contains(entry)) {
        let successor = new.iter().position(|next| same_place(entry, next));
        let by = successor.map(|at| new.remove(at));
        if replace(&mut settings, entry, by) {
            outcomes.push((entry.label(), Outcome::Removed));
            outcomes.extend(by.map(|by| (by.label(), Outcome::Added)));
            changed = true;
        } else {
            outcomes.push((entry.label(), Outcome::Kept));
            if let Some(by) = by {
                new.push(by);
            }
        }
    }
    for entry in wanted {
        if holds(&settings, entry) {
            continue;
        }
        // recorded yet no longer held: the project removed or edited it (SYN-33)
        if recorded.contains(entry) && !force {
            outcomes.push((entry.label(), Outcome::Conflict));
            continue;
        }
        add(&mut settings, entry);
        outcomes.push((entry.label(), Outcome::Added));
        changed = true;
    }
    let write = changed.then(|| {
        let mut text = serde_json::to_string_pretty(&settings).expect("a JSON value serialises");
        text.push('\n');
        text
    });
    Merge {
        settings: Some(settings),
        write,
        lock: wanted.to_vec(),
        outcomes,
        warning: None,
    }
}

/// Whether the `matcher` of a group is `matcher`: none, `""` and `"*"` all match every tool, so they are one value.
fn matches(group: &Value, matcher: Option<&str>) -> bool {
    let all = |m: Option<&str>| m.is_none_or(|m| m.is_empty() || m == "*");
    let have = group["matcher"].as_str();
    have == matcher || all(have) && all(matcher)
}

/// Whether `new` belongs where `old` is: a hook of the same event and matcher.
fn same_place(old: &Entry, new: &Entry) -> bool {
    match (old, new) {
        (
            Entry::Hook { event, matcher, .. },
            Entry::Hook {
                event: next_event,
                matcher: next_matcher,
                ..
            },
        ) => event == next_event && matcher == next_matcher,
        _ => false,
    }
}

/// Where `settings` hold the hook `hook` of `event` and `matcher`: the group's index and the hook's in it.
fn locate(
    settings: &Value,
    event: &str,
    matcher: Option<&str>,
    hook: &Value,
) -> Option<(usize, usize)> {
    settings["hooks"][event]
        .as_array()?
        .iter()
        .enumerate()
        .filter(|(_, group)| matches(group, matcher))
        .find_map(|(g, group)| {
            Some((
                g,
                group["hooks"].as_array()?.iter().position(|h| h == hook)?,
            ))
        })
}

/// Whether `settings` hold `entry` (JSON equality, key order ignored).
fn holds(settings: &Value, entry: &Entry) -> bool {
    match entry {
        Entry::Allow(rule) | Entry::Ask(rule) | Entry::Deny(rule) => settings["permissions"]
            [entry.list()]
        .as_array()
        .is_some_and(|rules| rules.iter().any(|r| r.as_str() == Some(rule))),
        Entry::Hook {
            event,
            matcher,
            hook,
        } => locate(settings, event, matcher.as_deref(), hook).is_some(),
    }
}

/// `settings` with `entry` appended (SYN-29); `settings` is in the shape of SYN-29.
fn add(settings: &mut Value, entry: &Entry) {
    let settings = settings.as_object_mut().expect("checked by `entries`");
    match entry {
        Entry::Allow(rule) | Entry::Ask(rule) | Entry::Deny(rule) => {
            let permissions = object(settings, "permissions");
            array(permissions, entry.list()).push(Value::String(rule.clone()));
        }
        Entry::Hook {
            event,
            matcher,
            hook,
        } => {
            let groups = array(object(settings, "hooks"), event);
            if let Some(group) = groups
                .iter_mut()
                .find(|group| matches(group, matcher.as_deref()))
            {
                group["hooks"]
                    .as_array_mut()
                    .expect("checked by `entries`")
                    .push(hook.clone());
            } else {
                let mut group = Map::new();
                if let Some(matcher) = matcher {
                    group.insert("matcher".into(), Value::String(matcher.clone()));
                }
                group.insert("hooks".into(), json!([hook]));
                groups.push(Value::Object(group));
            }
        }
    }
}

/// `settings` with the first occurrence of `entry` replaced by the hook of `by`, or removed when there is none, its
/// group and event then dropped when left empty (SYN-31); whether `settings` held `entry`. `by` is a hook of the same
/// event and matcher, and `settings` are in the shape of SYN-29.
fn replace(settings: &mut Value, entry: &Entry, by: Option<&Entry>) -> bool {
    match entry {
        Entry::Allow(rule) | Entry::Ask(rule) | Entry::Deny(rule) => {
            let Some(rules) = settings["permissions"][entry.list()].as_array_mut() else {
                return false;
            };
            let Some(at) = rules.iter().position(|r| r.as_str() == Some(rule)) else {
                return false;
            };
            rules.remove(at);
            true
        }
        Entry::Hook {
            event,
            matcher,
            hook,
        } => {
            let Some((g, at)) = locate(settings, event, matcher.as_deref(), hook) else {
                return false;
            };
            let hooks = settings["hooks"].as_object_mut().expect("located");
            let groups = hooks[event].as_array_mut().expect("located");
            let held = groups[g]["hooks"].as_array_mut().expect("located");
            if let Some(Entry::Hook { hook: by, .. }) = by {
                held[at] = by.clone();
                return true;
            }
            held.remove(at);
            if held.is_empty() {
                groups.remove(g);
            }
            if groups.is_empty() {
                // `shift_remove`, not `remove`: the other events keep their order
                hooks.shift_remove(event.as_str());
            }
            true
        }
    }
}

/// One `missing: .claude/settings.json <entry>` per entry of `recorded` the settings `text` no longer hold (SYN-36);
/// none when there are no settings (the next sync seeds them again, entries included), or when they cannot be parsed
/// or are not in the shape of SYN-29 (the warnings say so).
pub fn missing(text: Option<&str>, recorded: &[Entry]) -> Vec<String> {
    let Some(Ok(settings)) = text.map(serde_json::from_str::<Value>) else {
        return Vec::new();
    };
    if entries(&settings).is_err() {
        return Vec::new();
    }
    recorded
        .iter()
        .filter(|entry| !holds(&settings, entry))
        .map(|entry| format!("missing: {}", entry.label()))
        .collect()
}

/// The object at `key` of `map`, created empty when missing.
fn object<'a>(map: &'a mut Map<String, Value>, key: &str) -> &'a mut Map<String, Value> {
    map.entry(key)
        .or_insert_with(|| Value::Object(Map::new()))
        .as_object_mut()
        .expect("checked by `entries`")
}

/// The array at `key` of `map`, created empty when missing.
fn array<'a>(map: &'a mut Map<String, Value>, key: &str) -> &'a mut Vec<Value> {
    map.entry(key)
        .or_insert_with(|| Value::Array(Vec::new()))
        .as_array_mut()
        .expect("checked by `entries`")
}
