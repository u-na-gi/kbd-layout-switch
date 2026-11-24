//! The GNOME input source list (`org.gnome.desktop.input-sources sources`).

use std::iter::Peekable;
use std::process::Command;
use std::str::Chars;
use std::thread;
use std::time::Duration;

use crate::Result;

const SCHEMA: &str = "org.gnome.desktop.input-sources";
const KEY: &str = "sources";

/// Time given to GNOME Shell to act on the first of the two writes.
const SETTLE: Duration = Duration::from_millis(400);

/// An input source: a kind (`xkb`, `ibus`) and an identifier (`us`, `mozc-on`).
type Source = (String, String);

/// Make `layout` the xkb layout, keeping every other input source.
/// Returns whether anything had to change.
pub fn switch_to(layout: &str) -> Result<bool> {
    let current = parse(&gsettings(&["get", SCHEMA, KEY])?)?;
    let wanted = with_layout(&current, layout);
    if wanted == current {
        return Ok(false);
    }
    // An input method such as Mozc keeps whichever keymap was loaded last, so
    // if it stays selected the new layout never takes effect. Leave only the
    // xkb source first, which makes the shell select it and load its keymap,
    // then put the other sources back.
    let xkb_only = [("xkb".to_owned(), layout.to_owned())];
    gsettings(&["set", SCHEMA, KEY, &format(&xkb_only)])?;
    thread::sleep(SETTLE);
    gsettings(&["set", SCHEMA, KEY, &format(&wanted)])?;
    Ok(true)
}

/// Replace the xkb sources with a single one for `layout`, in the position of
/// the first, and leave the rest as they are.
fn with_layout(sources: &[Source], layout: &str) -> Vec<Source> {
    let mut wanted = Vec::with_capacity(sources.len() + 1);
    let mut placed = false;
    for (kind, id) in sources {
        if kind != "xkb" {
            wanted.push((kind.clone(), id.clone()));
        } else if !placed {
            wanted.push(("xkb".to_owned(), layout.to_owned()));
            placed = true;
        }
    }
    if !placed {
        wanted.insert(0, ("xkb".to_owned(), layout.to_owned()));
    }
    wanted
}

fn gsettings(arguments: &[&str]) -> Result<String> {
    let output = Command::new("gsettings")
        .args(arguments)
        .output()
        .map_err(|error| format!("failed to run gsettings: {error}"))?;
    if !output.status.success() {
        let message = String::from_utf8_lossy(&output.stderr);
        return Err(format!("gsettings {}: {}", arguments[0], message.trim()).into());
    }
    Ok(String::from_utf8(output.stdout)?)
}

/// Format sources as GVariant text of type `a(ss)`.
fn format(sources: &[Source]) -> String {
    let items: Vec<String> = sources
        .iter()
        .map(|(kind, id)| format!("({}, {})", quote(kind), quote(id)))
        .collect();
    format!("[{}]", items.join(", "))
}

fn quote(text: &str) -> String {
    let mut quoted = String::with_capacity(text.len() + 2);
    quoted.push('\'');
    for character in text.chars() {
        if matches!(character, '\'' | '\\') {
            quoted.push('\\');
        }
        quoted.push(character);
    }
    quoted.push('\'');
    quoted
}

/// Parse GVariant text of type `a(ss)`, as printed by `gsettings get`.
fn parse(text: &str) -> Result<Vec<Source>> {
    let invalid = || format!("unexpected input source list: {}", text.trim());
    // An empty list is printed with its type: `@a(ss) []`.
    let body = text.trim();
    let body = body.strip_prefix("@a(ss)").unwrap_or(body);
    let mut input = body.chars().peekable();
    let mut sources = Vec::new();

    expect(&mut input, '[').ok_or_else(invalid)?;
    if expect(&mut input, ']').is_none() {
        loop {
            expect(&mut input, '(').ok_or_else(invalid)?;
            let kind = string(&mut input).ok_or_else(invalid)?;
            expect(&mut input, ',').ok_or_else(invalid)?;
            let id = string(&mut input).ok_or_else(invalid)?;
            expect(&mut input, ')').ok_or_else(invalid)?;
            sources.push((kind, id));
            if expect(&mut input, ',').is_none() {
                expect(&mut input, ']').ok_or_else(invalid)?;
                break;
            }
        }
    }
    if input.any(|character| !character.is_whitespace()) {
        return Err(invalid().into());
    }
    Ok(sources)
}

/// Skip whitespace, then consume `wanted` if it comes next.
fn expect(input: &mut Peekable<Chars>, wanted: char) -> Option<()> {
    while input
        .next_if(|character| character.is_whitespace())
        .is_some()
    {}
    input.next_if_eq(&wanted).map(|_| ())
}

/// Consume a string quoted with `'` or `"`.
fn string(input: &mut Peekable<Chars>) -> Option<String> {
    while input
        .next_if(|character| character.is_whitespace())
        .is_some()
    {}
    let quote = input.next_if(|character| matches!(character, '\'' | '"'))?;
    let mut text = String::new();
    loop {
        match input.next()? {
            character if character == quote => return Some(text),
            '\\' => text.push(match input.next()? {
                'n' => '\n',
                't' => '\t',
                character => character,
            }),
            character => text.push(character),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sources(items: &[(&str, &str)]) -> Vec<Source> {
        items
            .iter()
            .map(|(kind, id)| (kind.to_string(), id.to_string()))
            .collect()
    }

    #[test]
    fn parses_gsettings_output() {
        assert_eq!(
            parse("[('xkb', 'us'), ('ibus', 'mozc-on')]\n").unwrap(),
            sources(&[("xkb", "us"), ("ibus", "mozc-on")])
        );
    }

    #[test]
    fn parses_an_empty_list() {
        assert_eq!(parse("@a(ss) []\n").unwrap(), sources(&[]));
    }

    #[test]
    fn parses_quotes_and_escapes() {
        assert_eq!(
            parse(r#"[('xkb', "it's"), ('a\\b', 'c\'d')]"#).unwrap(),
            sources(&[("xkb", "it's"), ("a\\b", "c'd")])
        );
    }

    #[test]
    fn rejects_malformed_input() {
        for text in ["", "[", "[('xkb')]", "[('xkb', 'us')] x", "('xkb', 'us')"] {
            assert!(parse(text).is_err(), "{text:?} should not parse");
        }
    }

    #[test]
    fn format_round_trips() {
        let list = sources(&[("xkb", "jp"), ("ibus", "it's"), ("a\\b", "")]);
        assert_eq!(parse(&format(&list)).unwrap(), list);
        assert_eq!(format(&sources(&[("xkb", "jp")])), "[('xkb', 'jp')]");
    }

    #[test]
    fn replaces_the_xkb_layout_in_place() {
        assert_eq!(
            with_layout(&sources(&[("ibus", "mozc-on"), ("xkb", "us")]), "jp"),
            sources(&[("ibus", "mozc-on"), ("xkb", "jp")])
        );
    }

    #[test]
    fn collapses_several_xkb_layouts_into_one() {
        assert_eq!(
            with_layout(
                &sources(&[("xkb", "us"), ("ibus", "mozc-on"), ("xkb", "de")]),
                "jp"
            ),
            sources(&[("xkb", "jp"), ("ibus", "mozc-on")])
        );
    }

    #[test]
    fn adds_an_xkb_layout_when_there_is_none() {
        assert_eq!(
            with_layout(&sources(&[("ibus", "mozc-on")]), "us"),
            sources(&[("xkb", "us"), ("ibus", "mozc-on")])
        );
    }

    #[test]
    fn leaves_a_matching_list_unchanged() {
        let list = sources(&[("xkb", "us"), ("ibus", "mozc-on")]);
        assert_eq!(with_layout(&list, "us"), list);
    }
}
