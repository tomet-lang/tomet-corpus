use std::fmt::Write;

use crate::rng::Rng;
use crate::title_of;
use crate::vocab::{
    CATEGORIES, DIALOGUE, HEADINGS, NARRATION, NOUNS, PARTICLES, PREDICATES, RUBY, TABLE_HEADS,
};

/// Which real-world shape a generated document imitates.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Style {
    /// Encyclopedia article: `@meta` with categories, nested `=` sections,
    /// link-heavy lists, tables.
    Wiki,
    /// Literary text: `@meta` with an author, `= title` / `== author`, long
    /// ruby-annotated narration and `「」` dialogue.
    Prose,
}

/// Relative frequency of each block kind. Only the ratios matter.
#[derive(Debug, Clone, Copy)]
pub struct BlockWeights {
    pub paragraph: u32,
    pub list: u32,
    pub table: u32,
    pub dialogue: u32,
}

/// Every knob of the generator. Two equal `Config`s with the same seed and
/// index always render byte-identical documents.
#[derive(Debug, Clone)]
pub struct Config {
    pub style: Style,
    /// Stop adding blocks once the document reaches this many bytes. The
    /// result overshoots by at most one block.
    pub target_bytes: usize,
    /// Deepest `=` heading level. The title is level 1, so `1` means no
    /// sub-headings.
    pub heading_depth: u8,
    /// Deepest list nesting (`1` = flat lists).
    pub list_depth: u8,
    pub weights: BlockWeights,
    /// Chance (per mille) that the next block is a heading instead.
    pub heading_permille: u16,
    /// Per-noun chance of becoming `@link(ref:"...")[...]`.
    pub link_permille: u16,
    /// Per-noun chance of becoming `*em*` or `**strong**`.
    pub emphasis_permille: u16,
    /// Per-noun chance of becoming `@ruby[...](rt:"...")`.
    pub ruby_permille: u16,
    /// Link targets are `title_of(0..universe)`. Set this to the corpus size
    /// to make every link resolve to a document that exists.
    pub universe: usize,
}

impl Config {
    /// Encyclopedia-like, ~28 KB (the measured mean of the Wikipedia pages).
    pub fn wiki() -> Self {
        Config {
            style: Style::Wiki,
            target_bytes: 28_000,
            heading_depth: 3,
            list_depth: 3,
            weights: BlockWeights {
                paragraph: 36,
                list: 2,
                table: 1,
                dialogue: 0,
            },
            heading_permille: 90,
            link_permille: 90,
            emphasis_permille: 14,
            ruby_permille: 0,
            universe: 1000,
        }
    }

    /// Literary-text-like, ~20 KB.
    pub fn prose() -> Self {
        Config {
            style: Style::Prose,
            target_bytes: 20_000,
            heading_depth: 2,
            list_depth: 1,
            weights: BlockWeights {
                paragraph: 6,
                list: 0,
                table: 0,
                dialogue: 5,
            },
            heading_permille: 0,
            link_permille: 0,
            emphasis_permille: 0,
            ruby_permille: 60,
            universe: 1,
        }
    }

    pub fn with_target_bytes(mut self, bytes: usize) -> Self {
        self.target_bytes = bytes;
        self
    }

    pub fn with_heading_depth(mut self, depth: u8) -> Self {
        self.heading_depth = depth.max(1);
        self
    }

    pub fn with_list_depth(mut self, depth: u8) -> Self {
        self.list_depth = depth.max(1);
        self
    }

    pub fn with_universe(mut self, universe: usize) -> Self {
        self.universe = universe.max(1);
        self
    }
}

pub(crate) fn render(index: usize, seed: u64, cfg: &Config) -> String {
    let mut g = Gen {
        rng: Rng::for_index(seed, index as u64),
        cfg,
        out: String::with_capacity(cfg.target_bytes + 1024),
    };
    g.front_matter(index);
    g.body();
    g.out
}

struct Gen<'a> {
    rng: Rng,
    cfg: &'a Config,
    out: String,
}

impl Gen<'_> {
    fn front_matter(&mut self, index: usize) {
        let title = title_of(index);
        let o = &mut self.out;
        let _ = writeln!(o, "@meta{{\n  id: {index},\n  title: \"{title}\",");
        match self.cfg.style {
            Style::Wiki => {
                let _ = writeln!(
                    o,
                    "  timestamp: \"2021-05-26T21:25:15Z\",\n  url.wiki: \"https://wiki.example.invalid/{title}\",\n  categories: ["
                );
                for _ in 0..self.rng.range(1, 3) {
                    let _ = writeln!(o, "    \"{}\",", self.rng.pick(CATEGORIES));
                }
                let _ = writeln!(o, "  ],");
            }
            Style::Prose => {
                let _ = writeln!(
                    o,
                    "  author: \"著者{}\",\n  release_date: \"2000-12-04\",\n  kana_type: \"新字新仮名\",",
                    index % 97
                );
            }
        }
        let _ = writeln!(o, "}}\n\n= {title}\n");
        if self.cfg.style == Style::Prose {
            let _ = writeln!(o, "== 著者{}\n", index % 97);
        }
    }

    fn body(&mut self) {
        let mut level: u8 = match self.cfg.style {
            Style::Wiki => 1,
            Style::Prose => 2,
        };
        while self.out.len() < self.cfg.target_bytes {
            if self.cfg.heading_depth >= 2 && self.rng.chance(self.cfg.heading_permille) {
                let deepest = (level + 1).min(self.cfg.heading_depth).max(2);
                level = self.rng.range(2, deepest as usize) as u8;
                let name = *self.rng.pick(HEADINGS);
                let _ = writeln!(self.out, "{} {name}\n", "=".repeat(level as usize));
            } else {
                self.block();
            }
        }
    }

    fn block(&mut self) {
        let w = self.cfg.weights;
        match self
            .rng
            .weighted(&[w.paragraph, w.list, w.table, w.dialogue])
        {
            0 => self.paragraph(),
            1 => self.list(),
            2 => self.table(),
            _ => self.dialogue(),
        }
    }

    fn paragraph(&mut self) {
        let sentences = match self.cfg.style {
            Style::Wiki => self.rng.range(1, 4),
            Style::Prose => self.rng.range(3, 8),
        };
        for _ in 0..sentences {
            self.sentence();
        }
        self.out.push_str("\n\n");
    }

    fn sentence(&mut self) {
        if self.cfg.style == Style::Prose && self.rng.chance(250) {
            let s = *self.rng.pick(NARRATION);
            self.out.push_str(s);
            return;
        }
        for _ in 0..self.rng.range(2, 5) {
            self.noun();
            let p = *self.rng.pick(PARTICLES);
            self.out.push_str(p);
        }
        self.noun();
        let end = *self.rng.pick(PREDICATES);
        self.out.push_str(end);
    }

    /// One noun, possibly wrapped in an inline element.
    fn noun(&mut self) {
        let cfg = self.cfg;
        let word = *self.rng.pick(NOUNS);
        if self.rng.chance(cfg.ruby_permille) {
            let (base, rt) = *self.rng.pick(RUBY);
            let _ = write!(self.out, "@ruby[{base}](rt:\"{rt}\")");
        } else if self.rng.chance(cfg.link_permille) {
            let target = title_of(self.rng.below(cfg.universe));
            let _ = write!(self.out, "@link(ref:\"{target}\")[{word}]");
        } else if self.rng.chance(cfg.emphasis_permille) {
            if self.rng.chance(500) {
                let _ = write!(self.out, "**{word}**");
            } else {
                let _ = write!(self.out, "*{word}*");
            }
        } else {
            self.out.push_str(word);
        }
    }

    /// A list-item sized run: nouns joined by particles, no predicate.
    fn phrase(&mut self) {
        for _ in 0..self.rng.range(0, 2) {
            self.noun();
            let p = *self.rng.pick(PARTICLES);
            self.out.push_str(p);
        }
        self.noun();
    }

    fn list(&mut self) {
        let marker = if self.rng.chance(100) { "-." } else { "-" };
        self.list_items(0, marker);
        self.out.push('\n');
    }

    fn list_items(&mut self, depth: usize, marker: &str) {
        for _ in 0..self.rng.range(2, 8 - 2 * depth.min(2)) {
            let _ = write!(self.out, "{}{marker} ", "  ".repeat(depth));
            self.phrase();
            self.out.push('\n');
            if depth + 1 < self.cfg.list_depth as usize && self.rng.chance(60) {
                self.list_items(depth + 1, marker);
            }
        }
    }

    fn table(&mut self) {
        let cols = self.rng.range(2, 5);
        let rows = self.rng.range(2, 6);
        self.out.push_str("@table\n");
        self.out.push('|');
        for _ in 0..cols {
            let head = *self.rng.pick(TABLE_HEADS);
            let _ = write!(self.out, "[ **{head}** ]");
        }
        self.out.push('\n');
        for _ in 0..rows {
            self.out.push('|');
            for _ in 0..cols {
                self.out.push_str("[ ");
                if self.rng.chance(300) {
                    let n = self.rng.range(1, 3000);
                    let _ = write!(self.out, "{n}");
                } else {
                    self.noun();
                }
                self.out.push_str(" ]");
            }
            self.out.push('\n');
        }
        self.out.push('\n');
    }

    fn dialogue(&mut self) {
        for _ in 0..self.rng.range(1, 4) {
            self.out.push('「');
            if self.rng.chance(500) {
                let line = *self.rng.pick(DIALOGUE);
                self.out.push_str(line);
            } else {
                self.sentence();
            }
            self.out.push_str("」\n");
        }
        self.out.push('\n');
    }
}
