//! What each letter looks like: the art, and where it sits in the atlas.
//!
//! Key types: `GLYPHS`, `ink`, `cell_index`.
//! Depends on: nothing.
//! INVARIANT: the art below **is** the font. It is not generated from anything
//! and nothing is generated from it — one line per character, the character
//! itself first, then seven rows of five. To change a glyph, change its picture.
//! Split from `font.rs` by length (CLAUDE.md), and the seam is a real one: this
//! file is what the letters look like, that one is how they are laid out.

/// Ink width and height of one glyph, in texels.
pub(super) const GLYPH_W: u32 = 5;
pub(super) const GLYPH_H: u32 = 7;

/// One cell of the atlas, in texels: the glyph plus a one-texel transparent
/// border on every side.
///
/// The border is what makes nearest sampling safe at any scale. A fragment
/// landing a hair outside a glyph's rectangle picks up its neighbour's border
/// rather than its neighbour's ink, so a scaled line of text never grows a
/// sliver of the letter next door. It also *is* the letter spacing: two texels
/// of gap between five texels of ink.
pub(super) const CELL_W: u32 = GLYPH_W + 2;
pub(super) const CELL_H: u32 = GLYPH_H + 2;

/// Cells across the atlas. Ninety-six cells hold ninety-five printable
/// characters and the fallback box.
pub(super) const COLUMNS: u32 = 16;
/// Cells down the atlas.
pub(super) const ROWS: u32 = 6;

/// The first character the font has: space.
pub(super) const FIRST: u8 = b' ';

/// Where the fallback box lives — the last cell, after the printable range.
pub(super) const FALLBACK_INDEX: u32 = 95;

/// What a character looks like — one line each, in ASCII order from space.
pub(super) const GLYPHS: [&str; 95] = [
    "  ..... ..... ..... ..... ..... ..... .....",
    "! ..#.. ..#.. ..#.. ..#.. ..#.. ..... ..#..",
    "\" .#.#. .#.#. ..... ..... ..... ..... .....",
    "# .#.#. .#.#. ##### .#.#. ##### .#.#. .#.#.",
    "$ ..#.. .#### #.#.. .###. ..#.# ####. ..#..",
    "% ##..# ##..# ...#. ..#.. .#... #..## #..##",
    "& .##.. #..#. #.#.. .#... #.#.# #..#. .##.#",
    "' ..#.. ..#.. ..... ..... ..... ..... .....",
    "( ...#. ..#.. .#... .#... .#... ..#.. ...#.",
    ") .#... ..#.. ...#. ...#. ...#. ..#.. .#...",
    "* ..... ..#.. #.#.# .###. #.#.# ..#.. .....",
    "+ ..... ..#.. ..#.. ##### ..#.. ..#.. .....",
    ", ..... ..... ..... ..... ..##. ..#.. .#...",
    "- ..... ..... ..... ##### ..... ..... .....",
    ". ..... ..... ..... ..... ..... .##.. .##..",
    "/ ....# ...#. ...#. ..#.. .#... .#... #....",
    "0 .###. #...# #..## #.#.# ##..# #...# .###.",
    "1 ..#.. .##.. ..#.. ..#.. ..#.. ..#.. .###.",
    "2 .###. #...# ....# ...#. ..#.. .#... #####",
    "3 ####. ....# ....# .###. ....# ....# ####.",
    "4 ...#. ..##. .#.#. #..#. ##### ...#. ...#.",
    "5 ##### #.... ####. ....# ....# #...# .###.",
    "6 ..##. .#... #.... ####. #...# #...# .###.",
    "7 ##### ....# ...#. ..#.. .#... .#... .#...",
    "8 .###. #...# #...# .###. #...# #...# .###.",
    "9 .###. #...# #...# .#### ....# ...#. .##..",
    ": ..... .##.. .##.. ..... .##.. .##.. .....",
    "; ..... .##.. .##.. ..... .##.. ..#.. .#...",
    "< ...#. ..#.. .#... #.... .#... ..#.. ...#.",
    "= ..... ..... ##### ..... ##### ..... .....",
    "> .#... ..#.. ...#. ....# ...#. ..#.. .#...",
    "? .###. #...# ....# ...#. ..#.. ..... ..#..",
    "@ .###. #...# #.### #.#.# #.### #.... .###.",
    "A ..#.. .#.#. #...# #...# ##### #...# #...#",
    "B ####. #...# #...# ####. #...# #...# ####.",
    "C .###. #...# #.... #.... #.... #...# .###.",
    "D ###.. #..#. #...# #...# #...# #..#. ###..",
    "E ##### #.... #.... ####. #.... #.... #####",
    "F ##### #.... #.... ####. #.... #.... #....",
    "G .###. #...# #.... #.### #...# #...# .###.",
    "H #...# #...# #...# ##### #...# #...# #...#",
    "I .###. ..#.. ..#.. ..#.. ..#.. ..#.. .###.",
    "J ....# ....# ....# ....# #...# #...# .###.",
    "K #...# #..#. #.#.. ##... #.#.. #..#. #...#",
    "L #.... #.... #.... #.... #.... #.... #####",
    "M #...# ##.## #.#.# #.#.# #...# #...# #...#",
    "N #...# ##..# #.#.# #..## #...# #...# #...#",
    "O .###. #...# #...# #...# #...# #...# .###.",
    "P ####. #...# #...# ####. #.... #.... #....",
    "Q .###. #...# #...# #...# #.#.# #..#. .##.#",
    "R ####. #...# #...# ####. #.#.. #..#. #...#",
    "S .###. #...# #.... .###. ....# #...# .###.",
    "T ##### ..#.. ..#.. ..#.. ..#.. ..#.. ..#..",
    "U #...# #...# #...# #...# #...# #...# .###.",
    "V #...# #...# #...# #...# #...# .#.#. ..#..",
    "W #...# #...# #...# #.#.# #.#.# ##.## #...#",
    "X #...# #...# .#.#. ..#.. .#.#. #...# #...#",
    "Y #...# #...# .#.#. ..#.. ..#.. ..#.. ..#..",
    "Z ##### ....# ...#. ..#.. .#... #.... #####",
    "[ .###. .#... .#... .#... .#... .#... .###.",
    "\\ #.... .#... .#... ..#.. ...#. ...#. ....#",
    "] .###. ...#. ...#. ...#. ...#. ...#. .###.",
    "^ ..#.. .#.#. #...# ..... ..... ..... .....",
    "_ ..... ..... ..... ..... ..... ..... #####",
    "` .#... ..#.. ..... ..... ..... ..... .....",
    "a ..... ..... .###. ....# .#### #...# .####",
    "b #.... #.... ####. #...# #...# #...# ####.",
    "c ..... ..... .###. #.... #.... #...# .###.",
    "d ....# ....# .#### #...# #...# #...# .####",
    "e ..... ..... .###. #...# ##### #.... .###.",
    "f ..##. .#..# .#... ###.. .#... .#... .#...",
    "g ..... .#### #...# #...# .#### ....# .###.",
    "h #.... #.... ####. #...# #...# #...# #...#",
    "i ..#.. ..... .##.. ..#.. ..#.. ..#.. .###.",
    "j ...#. ..... ..##. ...#. ...#. #..#. .##..",
    "k #.... #.... #..#. #.#.. ##... #.#.. #..#.",
    "l .##.. ..#.. ..#.. ..#.. ..#.. ..#.. .###.",
    "m ..... ..... ##.#. #.#.# #.#.# #.#.# #.#.#",
    "n ..... ..... ####. #...# #...# #...# #...#",
    "o ..... ..... .###. #...# #...# #...# .###.",
    "p ..... ..... ####. #...# ####. #.... #....",
    "q ..... ..... .#### #...# .#### ....# ....#",
    "r ..... ..... #.##. ##..# #.... #.... #....",
    "s ..... ..... .###. #.... .###. ....# ####.",
    "t .#... .#... ###.. .#... .#... .#..# ..##.",
    "u ..... ..... #...# #...# #...# #..## .##.#",
    "v ..... ..... #...# #...# #...# .#.#. ..#..",
    "w ..... ..... #...# #.#.# #.#.# #.#.# .#.#.",
    "x ..... ..... #...# .#.#. ..#.. .#.#. #...#",
    "y ..... ..... #...# #...# .#### ....# .###.",
    "z ..... ..... ##### ...#. ..#.. .#... #####",
    "{ ..##. .#... .#... ##... .#... .#... ..##.",
    "| ..#.. ..#.. ..#.. ..#.. ..#.. ..#.. ..#..",
    "} ##... ..#.. ..#.. ..##. ..#.. ..#.. ##...",
    "~ ..... ..... .#..# #.#.# #..#. ..... .....",
];

/// Drawn for anything the font does not have: a box with a dot in it.
///
/// Loud on purpose, in the same spirit as the missing-texture placeholder
/// (renderer.md §5). A character that silently drew nothing would make "my
/// score is not showing up" a mystery instead of a picture.
pub(super) const FALLBACK: &str = "\u{7f} ##### #...# #.#.# #..## #.#.# #...# #####";

/// Which atlas cell holds `character`, falling back to the box.
pub(super) fn cell_index(character: char) -> u32 {
    let code = u32::from(character);
    if code < u32::from(FIRST) || code >= u32::from(FIRST) + GLYPHS.len() as u32 {
        return FALLBACK_INDEX;
    }
    code - u32::from(FIRST)
}

/// Whether the atlas has ink at this texel.
pub(super) fn ink(x: u32, y: u32) -> bool {
    let (column, row) = (x / CELL_W, y / CELL_H);
    let index = row * COLUMNS + column;
    // Inside the cell, the glyph sits one texel in from the top-left corner.
    let (local_x, local_y) = (x % CELL_W, y % CELL_H);
    if local_x == 0 || local_y == 0 || local_x > GLYPH_W || local_y > GLYPH_H {
        return false;
    }
    let art = if index == FALLBACK_INDEX {
        FALLBACK
    } else {
        match GLYPHS.get(index as usize) {
            Some(art) => art,
            None => return false,
        }
    };
    // Each line is `<character> <row> <row> ...`, five characters per row with
    // one space between: the character, a space, then row * 6 to reach the row.
    let offset = 2 + (local_y - 1) * (GLYPH_W + 1) + (local_x - 1);
    art.as_bytes().get(offset as usize) == Some(&b'#')
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_line_of_the_table_is_the_character_it_claims_to_be() {
        // The table is positional: entry N is the character at 32 + N. Writing
        // the character out and checking it here is what turns "someone deleted
        // a line" from a font that is subtly shifted by one into a failure.
        for (index, art) in GLYPHS.iter().enumerate() {
            let expected = char::from(FIRST + index as u8);
            let found = art.chars().next();
            assert_eq!(found, Some(expected), "entry {index}: {art:?}");
        }
        assert_eq!(GLYPHS.len(), 95, "space through tilde");
    }
    #[test]
    fn every_glyph_is_five_by_seven() {
        for art in GLYPHS.iter().chain(core::iter::once(&FALLBACK)) {
            // Indexed rather than split on the first space, because for the
            // space character the prefix *is* a space.
            let (name, rows) = (&art[..1], &art[2..]);
            let rows: Vec<&str> = rows.split(' ').collect();
            assert_eq!(rows.len(), GLYPH_H as usize, "{name:?}");
            for row in rows {
                assert_eq!(row.len(), GLYPH_W as usize, "{name:?} row {row:?}");
                assert!(
                    row.chars().all(|pixel| pixel == '#' || pixel == '.'),
                    "{name:?} row {row:?}"
                );
            }
        }
    }
    #[test]
    fn every_glyph_has_a_clear_border_on_every_side() {
        // What makes nearest sampling safe: a fragment that lands a hair outside
        // a glyph finds its neighbour's border rather than its neighbour's ink.
        for index in 0..COLUMNS * ROWS {
            let (left, top) = ((index % COLUMNS) * CELL_W, (index / COLUMNS) * CELL_H);
            for x in 0..CELL_W {
                assert!(!ink(left + x, top), "cell {index} top edge");
                assert!(!ink(left + x, top + CELL_H - 1), "cell {index} bottom edge");
            }
            for y in 0..CELL_H {
                assert!(!ink(left, top + y), "cell {index} left edge");
                assert!(!ink(left + CELL_W - 1, top + y), "cell {index} right edge");
            }
        }
    }
}
