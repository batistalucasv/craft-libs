//! Multi-signal lexical and punctuation tone analysis.

use std::collections::BTreeMap;

use super::word_spans;

const MAX_DOMINANT_TONES: usize = 3;
const MAX_EVIDENCE_SPANS: usize = 8;

/// Validated UTF-8 byte range inside the analyzed text.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct TextSpan {
    /// Byte offset where the span starts.
    pub start: usize,
    /// Byte offset just past the span's last byte.
    pub end: usize,
}

impl TextSpan {
    /// Builds a validated UTF-8 span inside `text`.
    pub fn new(text: &str, start: usize, end: usize) -> Option<Self> {
        (start <= end && end <= text.len() && text.is_char_boundary(start) && text.is_char_boundary(end)).then_some(Self { start, end })
    }

    /// Slices `text` at this span if still within bounds and UTF-8 boundaries.
    pub fn slice(self, text: &str) -> Option<&str> {
        text.get(self.start..self.end)
    }
}

/// Tone categories detected by multi-signal analysis.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum ToneSignalKind {
    /// Assertive phrasing (`must`, `clearly`, `devemos`).
    Assertive,
    /// Formal register (`therefore`, `portanto`, `prezado`).
    Formal,
    /// Cordial or polite tone (`please`, `thank`, `obrigado`).
    Cordial,
    /// Hedging or hesitant tone (`maybe`, `perhaps`, `talvez`).
    Hesitant,
    /// Urgent tone (`urgent`, `immediately`, `!`).
    Urgent,
    /// Analytical reasoning (`however`, `evidence`, `contudo`).
    Analytical,
    /// Empathetic tone (`understand`, `compreendo`).
    Empathetic,
    /// Critical tone (`failed`, `broken`, `falhou`).
    Critical,
}

impl ToneSignalKind {
    /// Canonical lowercase identifier (`"assertive"`, `"formal"`, ...).
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Assertive => "assertive",
            Self::Formal => "formal",
            Self::Cordial => "cordial",
            Self::Hesitant => "hesitant",
            Self::Urgent => "urgent",
            Self::Analytical => "analytical",
            Self::Empathetic => "empathetic",
            Self::Critical => "critical",
        }
    }
}

/// A single detected tone with relative strength (`1..=100`) and evidence spans.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ToneSignal {
    /// Tone category.
    pub kind: ToneSignalKind,
    /// Relative percentage strength among all matched cues (`1..=100`).
    pub strength: u8,
    /// Byte spans in the input text that triggered this signal.
    pub evidence_spans: Vec<TextSpan>,
}

/// Top dominant tones detected in a text (up to 3).
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct ToneProfile {
    /// Dominant tones sorted from strongest to weakest.
    pub dominant: Vec<ToneSignal>,
}

const CUE_WORDS: &[(&str, ToneSignalKind)] = &[
    ("maybe", ToneSignalKind::Hesitant),
    ("perhaps", ToneSignalKind::Hesitant),
    ("possibly", ToneSignalKind::Hesitant),
    ("talvez", ToneSignalKind::Hesitant),
    ("aparentemente", ToneSignalKind::Hesitant),
    ("urgent", ToneSignalKind::Urgent),
    ("immediately", ToneSignalKind::Urgent),
    ("asap", ToneSignalKind::Urgent),
    ("urgente", ToneSignalKind::Urgent),
    ("imediatamente", ToneSignalKind::Urgent),
    ("therefore", ToneSignalKind::Formal),
    ("furthermore", ToneSignalKind::Formal),
    ("regarding", ToneSignalKind::Formal),
    ("portanto", ToneSignalKind::Formal),
    ("conforme", ToneSignalKind::Formal),
    ("prezado", ToneSignalKind::Formal),
    ("please", ToneSignalKind::Cordial),
    ("thank", ToneSignalKind::Cordial),
    ("thanks", ToneSignalKind::Cordial),
    ("appreciate", ToneSignalKind::Cordial),
    ("obrigado", ToneSignalKind::Cordial),
    ("obrigada", ToneSignalKind::Cordial),
    ("agradeço", ToneSignalKind::Cordial),
    ("must", ToneSignalKind::Assertive),
    ("clearly", ToneSignalKind::Assertive),
    ("guarantee", ToneSignalKind::Assertive),
    ("devemos", ToneSignalKind::Assertive),
    ("claramente", ToneSignalKind::Assertive),
    ("garantimos", ToneSignalKind::Assertive),
    ("however", ToneSignalKind::Analytical),
    ("consequently", ToneSignalKind::Analytical),
    ("evidence", ToneSignalKind::Analytical),
    ("contudo", ToneSignalKind::Analytical),
    ("dados", ToneSignalKind::Analytical),
    ("evidência", ToneSignalKind::Analytical),
    ("understand", ToneSignalKind::Empathetic),
    ("compreendo", ToneSignalKind::Empathetic),
    ("failed", ToneSignalKind::Critical),
    ("broken", ToneSignalKind::Critical),
    ("falhou", ToneSignalKind::Critical),
];

pub(super) fn analyze_tone(text: &str) -> ToneProfile {
    if text.trim().is_empty() {
        return ToneProfile::default();
    }
    let mut buckets: BTreeMap<ToneSignalKind, Vec<TextSpan>> = BTreeMap::new();
    for (start, end, word) in word_spans(text) {
        let lower = word.to_lowercase();
        if let Some((_, kind)) = CUE_WORDS.iter().find(|(cue, _)| *cue == lower)
            && let Some(span) = TextSpan::new(text, start, end)
        {
            push_span(&mut buckets, *kind, span);
        }
    }
    for (idx, ch) in text.char_indices() {
        if ch == '!'
            && let Some(span) = TextSpan::new(text, idx, idx + ch.len_utf8())
        {
            push_span(&mut buckets, ToneSignalKind::Urgent, span);
        }
    }

    let total_hits: usize = buckets.values().map(Vec::len).sum();
    if total_hits == 0 {
        return ToneProfile::default();
    }

    let mut signals: Vec<ToneSignal> = buckets
        .into_iter()
        .map(|(kind, evidence_spans)| {
            let share = (evidence_spans.len() * 100) / total_hits;
            let strength = u8::try_from(share.clamp(1, 100)).unwrap_or(100);
            ToneSignal { kind, strength, evidence_spans }
        })
        .collect();
    signals.sort_by(|a, b| b.strength.cmp(&a.strength).then_with(|| a.kind.cmp(&b.kind)));
    signals.truncate(MAX_DOMINANT_TONES);

    ToneProfile { dominant: signals }
}

fn push_span(buckets: &mut BTreeMap<ToneSignalKind, Vec<TextSpan>>, kind: ToneSignalKind, span: TextSpan) {
    let entry = buckets.entry(kind).or_default();
    if entry.len() < MAX_EVIDENCE_SPANS {
        entry.push(span);
    }
}
