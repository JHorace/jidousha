"""The sprite grids and their palettes — giri's whole art library.

One definition per asset slot in `UI.md` §9, in the shape the approved mockup
drew them: a list of equal-length strings, one character per texel, resolved
against a palette. `.` (and any key the palette does not carry) is transparent.

Kept beside `make_art.py` rather than inside it so the *pictures* and the
*encoder* are two files: a hand edit to a grid is a diff of the picture rather
than of a script, and this is where a change to how giri looks is made (owner,
2026-08-23 — the generated set is the shipped art, not a stand-in for one).

Palettes are UI.md §2's colour roles: stone for dungeons, the icon palette for
stat and event icons, bone for the skull, and one per-character palette per
portrait (portraits must stay unique per character — UI.md §9).
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
]
