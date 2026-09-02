"""The sprite grids and their palettes — ninjo's whole art library.

One definition per asset slot in `UI.md` §9, in the shape the approved mockup
drew them: a list of equal-length strings, one character per texel, resolved
against a palette. `.` (and any key the palette does not carry) is transparent.

Carried from giri with the fork. Kept beside `make_art.py` rather than inside it so the *pictures* and the
*encoder* are two files: a hand edit to a grid is a diff of the picture rather
than of a script, and this is where a change to how ninjo looks is made (owner,
2026-08-23 — the generated set is the shipped art, not a stand-in for one).

Palettes are UI.md §2's colour roles: stone for dungeons, the icon palette for
stat and event icons, bone for the skull, and one per-character palette per
portrait (portraits must stay unique per character — UI.md §9).

**Every role the game names has a grid here, including the ones a pack fills.**
`make_art.py` skips a curated slot and `import_pack.py` reads this table for the
role list, so a role missing from it cannot be imported at all — and a role with
no grid has no way back if a pack is ever withdrawn (`make_art.py --restore`).
The cast-art session (2026-09-02) added the fifteen founding-cast roles that way:
six portrait palettes over the shared grid, and nine chip icons drawn to the same
family cue the curated picks carry — the aptitudes as steel-and-timber line-work,
the motivators as one filled warm mass each.
"""

# ── palettes ────────────────────────────────────────────────────────────────
STONE = {"g": "#6e6a8a", "d": "#2a2438", "k": "#12101a", "w": "#e8ddc4", "y": "#e0b34a"}
ICONPAL = {
    "r": "#d4553a",  # ember — desperation
    "o": "#e8853a",
    "p": "#9b6dd6",  # violet — infamy
    "k": "#12101a",
    "w": "#cfd6d4",
    "y": "#e0b34a",  # gold — coin
}
BONE = {"w": "#e8ddc4", "k": "#14121d"}
REGARD = {"r": "#4fae8f", "w": "#8fd9c2"}  # teal — regard

# ── portraits, 16x16, one palette per character (UI.md §9) ──────────────────
PORTRAIT = [
    "................", "....hhhhhhhh....", "...hhhhhhhhhh...", "..hhhhhhhhhhhh..",
    "..hhsssssssshh..", "..hhsssssssshh..", "..hhsEssssEshh..", "..hhsssssssshh..",
    "...hssmmmmssh...", "...hssssssssh...", "....cccccccc....", "...cccccccccc...",
    "..cccccccccccc..", "..cccccccccccc..", "..cccccccccccc..", "................",
]
FACES = {
    "bob": {"h": "#5a4632", "s": "#d9a066", "E": "#12101a", "m": "#a9683f", "c": "#7a3b2e"},
    "alex": {"h": "#3d3d5c", "s": "#c98e5a", "E": "#12101a", "m": "#96603a", "c": "#4a4a7d"},
    "tim": {"h": "#b5651d", "s": "#e0b184", "E": "#12101a", "m": "#a9764c", "c": "#3e6b4f"},
    "steve": {"h": "#6b2d3c", "s": "#d59f7f", "E": "#12101a", "m": "#9c6a52", "c": "#8a3548"},
    # The founding cast's six (CAST.md §4). Each palette echoes the curated pick
    # `kenney-manifest.json` names for that role, so the generated fallback is
    # recognisably the same person rather than a stranger with the same id.
    "rin": {"h": "#c98a4b", "s": "#f0c39a", "E": "#12101a", "m": "#b57a52", "c": "#9b4fae"},
    "goro": {"h": "#7a4a2c", "s": "#e8a56a", "E": "#12101a", "m": "#b06a3c", "c": "#c98552"},
    "hana": {"h": "#8a5a34", "s": "#e8b47f", "E": "#12101a", "m": "#a9683f", "c": "#6b7a92"},
    "ludo": {"h": "#d9b48a", "s": "#e8c49a", "E": "#12101a", "m": "#8a5a3c", "c": "#8a5432"},
    "ines": {"h": "#c3ccd6", "s": "#e0b184", "E": "#12101a", "m": "#a9764c", "c": "#7a4230"},
    "odd": {"h": "#9aa3b0", "s": "#e8b47f", "E": "#12101a", "m": "#a9683f", "c": "#8a929e"},
}

# ── dungeon icons, 12x12, one per quest type (UI.md §2) ─────────────────────
CAVE = [
    "............", "....gggg....", "..gggggggg..", ".gggggggggg.", ".gggggggggg.",
    "gggggkkggggg", "ggggkkkkgggg", "gggkkkkkkggg", "gggkkkkkkggg", "gggkkkkkkggg",
    "dddddddddddd", "............",
]
CRYPT = [
    "............", "...gggggg...", "..gggggggg..", "..gggwwggg..", "..ggwwwwgg..",
    "..gggwwggg..", "..gggwwggg..", "..gggggggg..", "..gggggggg..", ".gggggggggg.",
    "dddddddddddd", "............",
]
TOWER = [
    "...g.gg.g...", "...gggggg...", "...gggggg...", "...ggkkgg...", "...gggggg...",
    "...gggggg...", "...ggkkgg...", "...gggggg...", "..gggggggg..", "..ggkkkkgg..",
    "dddddddddddd", "............",
]
VAULT = [
    "............", ".gggggggggg.", ".gddddddddg.", ".gddddddddg.", ".gddyyyyddg.",
    ".gddykkyddg.", ".gddyyyyddg.", ".gdddkkdddg.", ".gddddddddg.", ".gggggggggg.",
    "dddddddddddd", "............",
]

# ── stat and event icons (UI.md §2's signifier table) ───────────────────────
FLAME = ["...r....", "...rr...", "..rrr...", ".rrorr..", ".roorr..", ".roorrr.", ".rrorr..", "..rrr..."]
EYE = ["........", "..pppp..", ".pwwwwp.", "pwwkkwwp", "pwwkkwwp", ".pwwwwp.", "..pppp..", "........"]
COIN = ["..yyyy..", ".yooooy.", "yoyyyyoy", "yoyyyyoy", "yoyyyyoy", "yoyyyyoy", ".yooooy.", "..yyyy.."]
SKULL = [
    "..wwwwww..", ".wwwwwwww.", "wwwwwwwwww", "wwkkwwkkww", "wwkkwwkkww",
    "wwwwwwwwww", ".wwwkkwww.", ".wwwwwwww.", "..w.ww.w..", "..w.ww.w..",
]
# The one signifier §2's table names and the mockup never drew: regard.
HEART = ["........", ".rr..rr.", "rwrrrrrr", "rrrrrrrr", ".rrrrrr.", "..rrrr..", "...rr...", "........"]

# ── trait chip icons, 8x8, two families (CAST.md §3, UI.md §9) ──────────────
# The families are told apart by weight, not by subject: an **aptitude** is
# line-work — a steel-and-timber implement with the panel showing through it —
# and a **motivator** is one filled warm mass that fills its cell. The curated
# picks carry the same cue, so the two paths draw the same two families.
APTITUDE = {"s": "#cfd6d4", "d": "#8d84a0", "h": "#8a5a34", "y": "#e0b34a", "k": "#12101a"}
MOTIVE = {"o": "#e8853a", "r": "#d4553a", "b": "#8a5432", "w": "#e8ddc4", "k": "#12101a"}

FIGHT = ["......ss", ".....ss.", "....ss..", "...ss...", "..ss....", ".dsd....", ".hd.....", "h......."]
LABOR = [".h....h.", ".h....h.", ".hhhhhh.", ".h....h.", ".hhhhhh.", ".h....h.", ".hhhhhh.", ".h....h."]
SCOUT = ["...ss...", "..s..s..", ".ssssss.", ".syyyys.", ".syyyys.", ".syyyys.", ".ssssss.", "........"]
CRAFT = [".....sss", ".....sss", "....hs..", "...hh...", "..hh....", ".hh.....", "hh......", "........"]

INDEBTED = ["........", "..bbbb..", ".bbbbbb.", "boooooob", "oooooooo", "oooooooo", "oooooooo", ".oooooo."]
RENOWN = [".brrrr..", ".brrrr..", ".brrrr..", ".brr....", ".b......", ".b......", ".b......", ".b......"]
CARING = ["......ww", ".....ww.", "....rr..", "..rrrr..", ".rrrrr..", ".rrrrr..", "..rrr...", "........"]
RESTLESS = ["........", "...oo...", "ooooooo.", "oooooooo", "oooooooo", "ooooooo.", "...oo...", "........"]
MAKER = ["........", "........", ".bbbbbb.", "bbbbbbbb", "bbbbbbbb", ".b....b.", ".b....b.", ".b....b."]

# ── the library: role-named lowercase snake_case, as DESIGN §7 requires ─────
LIBRARY = [(f"portrait_{who}", PORTRAIT, pal) for who, pal in sorted(FACES.items())] + [
    ("quest_cave", CAVE, STONE),
    ("quest_crypt", CRYPT, STONE),
    ("quest_tower", TOWER, STONE),
    ("quest_vault", VAULT, STONE),
    ("icon_flame", FLAME, ICONPAL),
    ("icon_eye", EYE, ICONPAL),
    ("icon_coin", COIN, ICONPAL),
    ("icon_skull", SKULL, BONE),
    ("icon_heart", HEART, REGARD),
    ("icon_fight", FIGHT, APTITUDE),
    ("icon_labor", LABOR, APTITUDE),
    ("icon_scout", SCOUT, APTITUDE),
    ("icon_craft", CRAFT, APTITUDE),
    ("icon_indebted", INDEBTED, MOTIVE),
    ("icon_renown", RENOWN, MOTIVE),
    ("icon_caring", CARING, MOTIVE),
    ("icon_restless", RESTLESS, MOTIVE),
    ("icon_maker", MAKER, MOTIVE),
]
