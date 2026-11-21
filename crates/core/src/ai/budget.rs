//! Per-call context budget. A payload is a list of sections, each saying when it may give way;
//! `fit` cuts them in that order until the text fits.
use std::collections::BTreeSet;

/// Chars / 4, rounded up: more than the tokenizer counts for most text, so a fitted payload stays under.
pub fn tokens(s: &str) -> usize {
    s.chars().count().div_ceil(4)
}

/// What gives way first when a payload is over budget, in declaration order.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Stage {
    OldMessages,
    ClosedThreads,
    DistantNote,
    OpenThreads,
}

/// When a cut happens: by stage, then by rank within the stage, lowest first.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct Step(pub Stage, pub usize);

/// What a run of dropped sections is counted in, for the marker that stands in for it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Unit {
    Words,
    Messages,
    Threads,
}

/// A section with neither `shorten` nor `drop` is never cut.
#[derive(Debug, Clone, PartialEq)]
pub struct Section {
    pub text: String,
    pub shorten: Option<(Step, String)>,
    pub drop: Option<(Step, Unit)>,
}

impl Section {
    pub fn keep(text: impl Into<String>) -> Self {
        Self { text: text.into(), shorten: None, drop: None }
    }

    pub fn droppable(text: impl Into<String>, at: Step, unit: Unit) -> Self {
        Self { drop: Some((at, unit)), ..Self::keep(text) }
    }

    pub fn shortened(self, at: Step, short: Option<String>) -> Self {
        Self { shorten: short.map(|s| (at, s)), ..self }
    }
}

#[derive(Clone, Copy, PartialEq)]
enum Cut {
    Whole,
    Short,
    Gone,
}

/// Room counted for each omission marker, more than any marker takes.
const MARKER_CHARS: usize = 48;

/// The sections' text cut to `budget_tokens`, and the stages that gave way. Under budget it is the plain concatenation.
/// Sections that are never cut stay even when they alone are over budget.
pub fn fit(sections: &[Section], budget_tokens: usize) -> (String, BTreeSet<Stage>) {
    let limit = budget_tokens.saturating_mul(4);
    let len = |i: usize, cut: Cut| match cut {
        Cut::Whole => sections[i].text.chars().count(),
        Cut::Short => sections[i].shorten.as_ref().map_or(0, |(_, s)| s.chars().count()),
        Cut::Gone => 0,
    };
    let mut cuts = vec![Cut::Whole; sections.len()];
    let mut total: usize = (0..sections.len()).map(|i| len(i, Cut::Whole)).sum();
    let mut steps: Vec<(Step, usize, Cut)> = Vec::new();
    for (i, s) in sections.iter().enumerate() {
        if let Some((at, _)) = &s.shorten {
            steps.push((*at, i, Cut::Short));
        }
        if let Some((at, _)) = &s.drop {
            steps.push((*at, i, Cut::Gone));
        }
    }
    steps.sort_by_key(|(at, i, _)| (*at, *i));
    let unit = |i: usize| sections[i].drop.map(|(_, u)| u);
    let mut cut_stages = BTreeSet::new();
    for (Step(stage, _), i, cut) in steps {
        if total <= limit {
            break;
        }
        let before = len(i, cuts[i]);
        match cut {
            Cut::Short if cuts[i] == Cut::Whole => {
                cuts[i] = Cut::Short;
                total = total - before + len(i, Cut::Short);
            }
            Cut::Gone => {
                let gone = |j: usize| cuts[j] == Cut::Gone && unit(j) == unit(i);
                let joins = (i > 0 && gone(i - 1)) as usize + (i + 1 < sections.len() && gone(i + 1)) as usize;
                cuts[i] = Cut::Gone;
                total = match joins {
                    0 => total - before + MARKER_CHARS,
                    1 => total - before,
                    _ => total - before - MARKER_CHARS,
                };
            }
            _ => continue,
        }
        cut_stages.insert(stage);
    }
    (render(sections, &cuts), cut_stages)
}

fn render(sections: &[Section], cuts: &[Cut]) -> String {
    let mut out = String::new();
    let mut run: Option<(Unit, usize, bool)> = None;
    let flush = |out: &mut String, run: &mut Option<(Unit, usize, bool)>| {
        if let Some((unit, n, newline)) = run.take() {
            out.push_str(&marker(unit, n, newline));
        }
    };
    for (s, cut) in sections.iter().zip(cuts) {
        match cut {
            Cut::Gone => {
                let unit = s.drop.map_or(Unit::Words, |(_, u)| u);
                let n = if unit == Unit::Words { s.text.split_whitespace().count() } else { 1 };
                match &mut run {
                    Some((u, count, newline)) if *u == unit => {
                        *count += n;
                        *newline = s.text.ends_with('\n');
                    }
                    _ => {
                        flush(&mut out, &mut run);
                        run = Some((unit, n, s.text.ends_with('\n')));
                    }
                }
            }
            Cut::Short => {
                flush(&mut out, &mut run);
                out.push_str(s.shorten.as_ref().map_or("", |(_, t)| t));
            }
            Cut::Whole => {
                flush(&mut out, &mut run);
                out.push_str(&s.text);
            }
        }
    }
    flush(&mut out, &mut run);
    out
}

fn marker(unit: Unit, n: usize, newline: bool) -> String {
    if n == 0 {
        return String::new();
    }
    let plural = |word: &str| if n == 1 { word.to_string() } else { format!("{word}s") };
    let what = match unit {
        Unit::Words => plural("word"),
        Unit::Messages => format!("earlier {}", plural("message")),
        Unit::Threads => plural("thread"),
    };
    format!("[… {n} {what} omitted …]{}", if newline { "\n" } else { " " })
}
