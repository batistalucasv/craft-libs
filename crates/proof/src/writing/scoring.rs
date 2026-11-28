//! Transparent, open-formula writing score and readability metrics.

use crate::hyphen::{Limits, hyphen_points_in};
use crate::{Issue, IssueKind, ProofLang};

use super::word_spans;

const READING_WPM: f64 = 200.0;
const SPEAKING_WPM: f64 = 130.0;

/// Qualitative readability band derived from open formulas.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum ReadabilityLevel {
    /// Very easy prose (`Flesch >= 80`).
    VeryEasy,
    /// Easy prose (`Flesch >= 65`).
    Easy,
    /// Standard prose (`Flesch >= 50`).
    #[default]
    Standard,
    /// Difficult prose (`Flesch >= 30`).
    Difficult,
    /// Academic or dense prose (`Flesch < 30`).
    Academic,
}

impl ReadabilityLevel {
    /// Canonical lowercase identifier (`"very_easy"`, `"easy"`, `"standard"`, `"difficult"`, `"academic"`).
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::VeryEasy => "very_easy",
            Self::Easy => "easy",
            Self::Standard => "standard",
            Self::Difficult => "difficult",
            Self::Academic => "academic",
        }
    }
}

/// Open readability metrics and reading/speaking time estimates for a text.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct ReadabilityMetrics {
    /// Flesch Reading Ease (`0.0..=100.0`).
    pub flesch_reading_ease: f64,
    /// Flesch-Kincaid Grade Level (`>= 0.0`).
    pub flesch_kincaid_grade: f64,
    /// Gulpease readability index (`0.0..=100.0`).
    pub gulpease_index: f64,
    /// Estimated silent reading duration in minutes (200 WPM).
    pub reading_minutes: f64,
    /// Estimated aloud speaking duration in minutes (130 WPM).
    pub speaking_minutes: f64,
    /// Qualitative readability band.
    pub level: ReadabilityLevel,
}

/// Transparent, open-formula overall writing score (`0..=100`).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct WritingScore {
    /// Overall score (`0..=100`; empty or issue-free texts score `100`).
    pub score: u8,
    /// Total weighted penalty points across detected issues.
    pub penalty_points: f64,
    /// Count of spelling issues in the analyzed text.
    pub spelling_issues: usize,
    /// Count of grammar and clarity issues in the analyzed text.
    pub grammar_issues: usize,
}

impl Default for WritingScore {
    fn default() -> Self {
        Self { score: 100, penalty_points: 0.0, spelling_issues: 0, grammar_issues: 0 }
    }
}

impl WritingScore {
    /// Computes an open `0..=100` writing quality score normalized by word count.
    pub fn compute(issues: &[Issue], words: usize) -> Self {
        let mut spelling_issues = 0usize;
        let mut grammar_issues = 0usize;
        let mut penalty_points = 0.0f64;
        for issue in issues {
            match issue.kind {
                IssueKind::Spelling => {
                    spelling_issues = spelling_issues.saturating_add(1);
                    penalty_points += 5.0;
                }
                IssueKind::Grammar => {
                    grammar_issues = grammar_issues.saturating_add(1);
                    penalty_points += 2.5;
                }
            }
        }
        if words == 0 || issues.is_empty() {
            return Self { score: 100, penalty_points: 0.0, spelling_issues, grammar_issues };
        }
        let word_scale = (words.max(20) as f64) / 20.0;
        let normalized_penalty = (penalty_points / word_scale) * 4.0;
        let raw_score = (100.0 - normalized_penalty).clamp(0.0, 100.0);
        Self { score: raw_score.round() as u8, penalty_points, spelling_issues, grammar_issues }
    }
}

#[derive(Clone, Copy, Debug, Default)]
pub(super) struct TextCounts {
    pub words: usize,
    pub sentences: usize,
    pub paragraphs: usize,
}

pub(super) fn measure_text(text: &str) -> TextCounts {
    if text.is_empty() {
        return TextCounts::default();
    }
    let words = text.split_whitespace().filter(|token| token.chars().any(char::is_alphanumeric)).count();
    let paragraphs = text.lines().filter(|line| !line.trim().is_empty()).count();
    let sentences = count_sentences(text, words);
    TextCounts { words, sentences, paragraphs }
}

fn count_sentences(text: &str, words: usize) -> usize {
    if words == 0 {
        return 0;
    }
    let mut sentences = 0usize;
    let mut saw_word_char = false;
    let mut chars = text.chars().peekable();
    while let Some(ch) = chars.next() {
        if ch.is_alphanumeric() {
            saw_word_char = true;
            continue;
        }
        if matches!(ch, '.' | '!' | '?' | '…')
            && saw_word_char
            && chars.peek().is_none_or(|next| next.is_whitespace() || matches!(*next, '"' | '\'' | '”' | '’' | '»' | ')' | ']'))
        {
            sentences = sentences.saturating_add(1);
            saw_word_char = false;
        }
    }
    sentences.max(1)
}

pub(super) fn compute_readability(text: &str, counts: TextCounts, lang: ProofLang) -> ReadabilityMetrics {
    if counts.words == 0 || counts.sentences == 0 {
        return ReadabilityMetrics::default();
    }
    let words = counts.words as f64;
    let sentences = counts.sentences as f64;
    let syllables = count_total_syllables(text, lang).max(counts.words) as f64;
    let letters = text.chars().filter(|c| c.is_alphabetic()).count() as f64;

    let words_per_sentence = words / sentences;
    let syllables_per_word = syllables / words;

    let flesch_reading_ease = (206.835 - 1.015 * words_per_sentence - 84.6 * syllables_per_word).clamp(0.0, 100.0);
    let flesch_kincaid_grade = (0.39 * words_per_sentence + 11.8 * syllables_per_word - 15.59).max(0.0);
    let gulpease_index = (89.0 + (300.0 * sentences - 10.0 * letters) / words).clamp(0.0, 100.0);

    ReadabilityMetrics {
        flesch_reading_ease,
        flesch_kincaid_grade,
        gulpease_index,
        reading_minutes: words / READING_WPM,
        speaking_minutes: words / SPEAKING_WPM,
        level: classify_level(flesch_reading_ease),
    }
}

fn count_total_syllables(text: &str, lang: ProofLang) -> usize {
    let limits = Limits::default();
    word_spans(text)
        .into_iter()
        .map(|(_, _, word)| {
            let hyphen_syllables = hyphen_points_in(word, &limits, lang).len().saturating_add(1);
            let vowel_syllables = count_vowel_groups(word);
            hyphen_syllables.max(vowel_syllables).max(1)
        })
        .sum()
}

fn count_vowel_groups(word: &str) -> usize {
    let mut groups = 0usize;
    let mut in_vowel = false;
    for ch in word.chars() {
        let vowel = is_vowel(ch);
        if vowel && !in_vowel {
            groups = groups.saturating_add(1);
        }
        in_vowel = vowel;
    }
    groups.max(1)
}

const fn is_vowel(ch: char) -> bool {
    matches!(
        ch,
        'a' | 'e'
            | 'i'
            | 'o'
            | 'u'
            | 'y'
            | 'A'
            | 'E'
            | 'I'
            | 'O'
            | 'U'
            | 'Y'
            | 'á'
            | 'à'
            | 'â'
            | 'ã'
            | 'é'
            | 'ê'
            | 'í'
            | 'ó'
            | 'ô'
            | 'õ'
            | 'ú'
            | 'ü'
            | 'Á'
            | 'À'
            | 'Â'
            | 'Ã'
            | 'É'
            | 'Ê'
            | 'Í'
            | 'Ó'
            | 'Ô'
            | 'Õ'
            | 'Ú'
            | 'Ü'
    )
}

fn classify_level(flesch: f64) -> ReadabilityLevel {
    if flesch >= 80.0 {
        ReadabilityLevel::VeryEasy
    } else if flesch >= 65.0 {
        ReadabilityLevel::Easy
    } else if flesch >= 50.0 {
        ReadabilityLevel::Standard
    } else if flesch >= 30.0 {
        ReadabilityLevel::Difficult
    } else {
        ReadabilityLevel::Academic
    }
}

pub(super) fn has_uniform_sentence_rhythm(word_counts: &[usize], min_run: usize) -> bool {
    if min_run == 0 || word_counts.len() < min_run {
        return false;
    }
    word_counts.windows(min_run).any(|win| {
        let min_len = win.iter().copied().min().unwrap_or(0);
        let max_len = win.iter().copied().max().unwrap_or(0);
        min_len >= 5 && max_len.saturating_sub(min_len) <= 1
    })
}
