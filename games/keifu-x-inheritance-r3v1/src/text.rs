//! The text conventions of SPEC §21 and the format convention of `content/README.md`.
//!
//! Every function here assembles content strings; none of them carries a word of
//! its own. The Jai `print` convention (`%`, `%N`, `%%`, `\%`) is implemented by
//! `fmt`, and it is strict: a template given the wrong number of arguments is a
//! bug in the caller, and it panics with the template and the arguments rather
//! than drawing a sentence with a hole in it.

use crate::constants::DOOR_YEARS;
use crate::content::Content;
use crate::ids::Pool;
use jidousha::prelude::Rng;

/// Fill a Jai `print` template.
///
/// `%` is the next argument, `%N` (one digit) is argument N, `%%` is the next
/// two arguments back to back, and `\%` is a literal percent.
///
/// CONTRACT: every argument is used, and no placeholder lacks one.
pub fn fmt(template: &str, args: &[&str]) -> String {
    fill(template, args, true)
}

/// Fill a story whose arguments are numbered and which may leave one out: the Door's
/// lock endings always name the bearer (`%2`) and only some name the party (`%1`) (SPEC
/// §16.2, `content/door.json`). Every placeholder still needs its argument.
pub fn fmt_numbered_story(template: &str, args: &[&str]) -> String {
    fill(template, args, false)
}

/// `fmt`'s filling, with or without the every-argument-used contract.
fn fill(template: &str, args: &[&str], every_used: bool) -> String {
    let mut out = String::with_capacity(template.len() + 16);
    let mut next = 0;
    let mut used = vec![false; args.len()];
    let mut take = |index: usize, out: &mut String| {
        let Some(arg) = args.get(index) else {
            panic!(
                "[keifu_x_inheritance_r3v1] a template wants more arguments than it was given\n  {template:?} \
                 with {args:?}\n  likely cause: the caller passes the wrong number of \
                 arguments for this key\n  fix: read the key's `args` in spec/content"
            );
        };
        used[index] = true;
        out.push_str(arg);
    };
    let mut chars = template.chars().peekable();
    while let Some(ch) = chars.next() {
        match ch {
            '\\' if chars.peek() == Some(&'%') => {
                chars.next();
                out.push('%');
            }
            '%' => match chars.peek().copied() {
                Some('%') => {
                    chars.next();
                    take(next, &mut out);
                    take(next + 1, &mut out);
                    next += 2;
                }
                Some(digit @ '1'..='9') => {
                    chars.next();
                    take(digit as usize - '1' as usize, &mut out);
                }
                _ => {
                    take(next, &mut out);
                    next += 1;
                }
            },
            other => out.push(other),
        }
    }
    if let Some(unused) = used.iter().position(|was| !was).filter(|_| every_used) {
        panic!(
            "[keifu_x_inheritance_r3v1] a template was given an argument it does not use\n  {template:?} with \
             {args:?}, argument {} unused\n  likely cause: the caller passes the wrong \
             arguments for this key\n  fix: read the key's `args` in spec/content",
            unused + 1
        );
    }
    out
}

/// The first byte upper-cased (SPEC §21 "Capitalized").
pub fn capitalized(text: &str) -> String {
    let mut chars = text.chars();
    match chars.next() {
        Some(first) => first.to_ascii_uppercase().to_string() + chars.as_str(),
        None => String::new(),
    }
}

/// The first byte lower-cased (SPEC §21 "Lowered").
pub fn lowered(text: &str) -> String {
    let mut chars = text.chars();
    match chars.next() {
        Some(first) => first.to_ascii_lowercase().to_string() + chars.as_str(),
        None => String::new(),
    }
}

/// "no one yet" / "a" / "a, b and c" (SPEC §21 "Name list").
pub fn name_list(content: &Content, names: &[&str]) -> String {
    let lore = &content.lore;
    match names {
        [] => lore.name_list_empty.clone(),
        [one] => (*one).to_owned(),
        [init @ .., last] => {
            init.join(&lore.name_list_separator) + &lore.name_list_last_separator + last
        }
    }
}

/// "No one", else the name list (SPEC §21 "Party telling").
pub fn party_telling(content: &Content, first_names: &[&str]) -> String {
    if first_names.is_empty() {
        content.lore.party_of_no_one.clone()
    } else {
        name_list(content, first_names)
    }
}

/// "before the first year" / "in the last summer" / "in year N" (SPEC §21).
pub fn year_telling(content: &Content, year: i32) -> String {
    let lore = &content.lore;
    if year < 1 {
        lore.year_before_first.clone()
    } else if year > DOOR_YEARS {
        lore.year_last_summer.clone()
    } else {
        fmt(&lore.year_in, &[&year.to_string()])
    }
}

/// "never", "once", ... "twelve times", then "N times" (SPEC §21 "Count words").
pub fn count_words(content: &Content, count: usize) -> String {
    match content.lore.count_words.get(count) {
        Some(word) => word.clone(),
        None => fmt(&content.lore.count_words_beyond, &[&count.to_string()]),
    }
}

/// A signed number as the sheets print it: "+1", "-2", "0".
pub fn signed(value: i32) -> String {
    if value > 0 {
        format!("+{value}")
    } else {
        value.to_string()
    }
}

/// Per-pool memory of the line picked last (SPEC §21 "Writing pools"), and the last
/// epitaph frame rolled anywhere in the run (SPEC §3.1 `writing`, §20).
///
/// Each pick is uniform over the pool excluding the line picked last time from
/// that same pool in this run; the first pick is uniform over all of it. An epitaph's
/// frame is rolled the same way against the last frame (`epitaph::roll_wording`).
#[derive(Clone, Debug, Default, PartialEq)]
pub struct WritingMemory {
    last: Vec<Option<usize>>,
    /// The frame the last wording rolled, if any has been.
    pub last_frame: Option<usize>,
}

impl WritingMemory {
    /// Pick a line from `pool`, never the previous pick from that pool.
    pub fn pick<'c>(&mut self, content: &'c Content, pool: Pool, rng: &mut Rng) -> &'c str {
        if self.last.len() < Pool::ALL.len() {
            self.last.resize(Pool::ALL.len(), None);
        }
        let lines = &content.pools[pool.index()];
        let index = match self.last[pool.index()] {
            None => crate::chance::index(rng, lines.len()),
            Some(previous) => crate::chance::fresh_index(rng, lines.len(), previous),
        };
        self.last[pool.index()] = Some(index);
        &lines[index]
    }

    /// The line picked last from `pool`, if any.
    pub fn last(&self, pool: Pool) -> Option<usize> {
        self.last.get(pool.index()).copied().flatten()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_double_percent_takes_two_arguments_and_an_escaped_one_is_literal() {
        assert_eq!(
            fmt("% wanted %%, and did it.", &["She", "x", "y"]),
            "She wanted xy, and did it."
        );
        assert_eq!(
            fmt("%1 %2 of %3, %4%5 for", &["a", "b", "c", "-", "2"]),
            "a b of c, -2 for"
        );
        assert_eq!(fmt("open % in 100 \\%", &["5"]), "open 5 in 100 %");
        assert_eq!(fmt("%2 then %1", &["one", "two"]), "two then one");
    }

    #[test]
    #[should_panic(expected = "more arguments")]
    fn a_template_short_of_arguments_panics() {
        let _ = fmt("% and %", &["one"]);
    }

    #[test]
    #[should_panic(expected = "does not use")]
    fn a_template_given_too_many_arguments_panics() {
        let _ = fmt("%", &["one", "two"]);
    }

    #[test]
    fn capitalised_and_lowered_change_only_the_first_byte() {
        assert_eq!(capitalized("the Barrow"), "The Barrow");
        assert_eq!(lowered("To See the Sea"), "to See the Sea");
        assert_eq!(lowered(""), "");
    }
}
