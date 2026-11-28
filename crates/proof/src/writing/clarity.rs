//! Clarity and conciseness checks: vague nominalizations, conservative passive voice,
//! long sentences, and monotonous sentence rhythm across English, Portuguese, and Spanish.

use crate::{Issue, IssueKind, ProofLang};

use super::scoring::has_uniform_sentence_rhythm;
use super::{FoldedText, LangGroup, has_word_boundaries, match_case, word_spans};

const LONG_SENTENCE_DEFAULT_WORDS: usize = 30;
const MONOTONOUS_MIN_RUN: usize = 3;

const EN_NOMINALIZATIONS: &[(&str, &str)] = &[
    ("make a decision", "decide"),
    ("take into consideration", "consider"),
    ("perform an analysis", "analyze"),
    ("conduct an investigation", "investigate"),
    ("give an explanation", "explain"),
];

const PT_NOMINALIZATIONS: &[(&str, &str)] = &[
    ("fazer uma análise", "analisar"),
    ("tomar uma decisão", "decidir"),
    ("realizar uma verificação", "verificar"),
    ("dar uma olhada", "verificar"),
    ("fazer uma revisão", "revisar"),
];

const EN_AUXILIARIES: &[&str] = &["is", "are", "was", "were", "be", "been", "being"];
const EN_IRREGULAR_PARTICIPLES: &[&str] = &[
    "written", "built", "made", "sent", "reviewed", "checked", "given", "taken", "shown", "found", "done", "seen", "known", "broken", "chosen",
    "spoken",
];
const EN_NOT_PARTICIPLES: &[&str] = &["bed", "red", "need", "speed", "seed", "feed", "shed", "sled", "bleed", "breed", "hundred", "tired", "excited"];

const PT_AUXILIARIES: &[&str] = &["é", "são", "foi", "foram", "era", "eram", "será", "serão", "sendo", "sido"];
const PT_IRREGULAR_PARTICIPLES: &[&str] =
    &["feito", "feita", "feitos", "feitas", "escrito", "escrita", "dito", "dita", "visto", "vista", "aberto", "aberta", "posto", "posta"];
const PT_NOT_PARTICIPLES: &[&str] = &[
    "estado",
    "lado",
    "resultado",
    "conteúdo",
    "sentido",
    "período",
    "rápido",
    "sólido",
    "válido",
    "líquido",
    "vida",
    "medida",
    "saída",
    "entrada",
    "partido",
    "marido",
];

pub(super) fn check(text: &str, folded: &FoldedText, group: LangGroup, lang: ProofLang, out: &mut Vec<Issue>) {
    match group {
        LangGroup::En => {
            check_nominalizations(text, folded, EN_NOMINALIZATIONS, false, lang, out);
            check_passive_voice(text, false, lang, out);
        }
        LangGroup::Pt => {
            check_nominalizations(text, folded, PT_NOMINALIZATIONS, true, lang, out);
            check_passive_voice(text, true, lang, out);
        }
        LangGroup::Es => {}
    }
    check_long_sentences(text, group, lang, out);
    check_monotonous_rhythm(text, group, lang, out);
}

fn check_nominalizations(text: &str, folded: &FoldedText, table: &[(&str, &str)], is_pt: bool, lang: ProofLang, out: &mut Vec<Issue>) {
    for &(phrase, replacement) in table {
        let mut cursor = 0usize;
        while let Some(rel) = folded.lower.get(cursor..).and_then(|s| s.find(phrase)) {
            let start = cursor + rel;
            let end = start + phrase.len();
            if has_word_boundaries(&folded.lower, start, end)
                && let Some((orig_start, orig_end)) = folded.map_range(start, end)
                && let Some(orig) = text.get(orig_start..orig_end)
            {
                let suggested = match_case(orig, replacement);
                let message = if is_pt {
                    format!("Prefira o verbo direto \"{suggested}\" em vez da nominalização vaga.")
                } else {
                    format!("Prefer the direct verb \"{suggested}\" over a vague nominalization.")
                };
                out.push(Issue { start: orig_start, end: orig_end, kind: IssueKind::Grammar, message, suggestions: vec![suggested], lang });
            }
            cursor = end;
        }
    }
}

fn check_passive_voice(text: &str, is_pt: bool, lang: ProofLang, out: &mut Vec<Issue>) {
    let spans = word_spans(text);
    for pair in spans.windows(2) {
        let [(a_start, a_end, aux), (p_start, p_end, part)] = pair else {
            continue;
        };
        if !gap_is_plain_spaces(text, *a_end, *p_start) {
            continue;
        }
        let aux_lower = aux.to_lowercase();
        let part_lower = part.to_lowercase();
        let matched = if is_pt {
            PT_AUXILIARIES.contains(&aux_lower.as_str()) && is_pt_participle(&part_lower)
        } else {
            EN_AUXILIARIES.contains(&aux_lower.as_str()) && is_en_participle(&part_lower)
        };
        if matched {
            let message = if is_pt {
                "Construção na voz passiva; considere reescrever na voz ativa para maior clareza."
            } else {
                "Passive voice construction; consider rewriting in active voice for directness."
            };
            out.push(Issue { start: *a_start, end: *p_end, kind: IssueKind::Grammar, message: message.to_owned(), suggestions: Vec::new(), lang });
        }
    }
}

fn check_long_sentences(text: &str, group: LangGroup, lang: ProofLang, out: &mut Vec<Issue>) {
    for (start, end, sentence) in sentence_spans(text) {
        let word_count = word_spans(sentence).len();
        if word_count <= LONG_SENTENCE_DEFAULT_WORDS {
            continue;
        }
        let message = match group {
            LangGroup::Pt => format!("Esta frase tem {word_count} palavras (limite sugerido: {LONG_SENTENCE_DEFAULT_WORDS}); considere dividi-la."),
            LangGroup::Es => {
                format!("Esta oración tiene {word_count} palabras (límite sugerido: {LONG_SENTENCE_DEFAULT_WORDS}); considere dividirla.")
            }
            LangGroup::En => format!("This sentence has {word_count} words (suggested limit: {LONG_SENTENCE_DEFAULT_WORDS}); consider splitting it."),
        };
        out.push(Issue { start, end, kind: IssueKind::Grammar, message, suggestions: Vec::new(), lang });
    }
}

fn check_monotonous_rhythm(text: &str, group: LangGroup, lang: ProofLang, out: &mut Vec<Issue>) {
    let sentences = sentence_spans(text);
    if sentences.len() < MONOTONOUS_MIN_RUN {
        return;
    }
    let first_words: Vec<String> =
        sentences.iter().map(|(_, _, s)| word_spans(s).first().map(|(_, _, w)| w.to_lowercase()).unwrap_or_default()).collect();
    let repeated_opener = first_words.windows(MONOTONOUS_MIN_RUN).any(|win| {
        let first = &win[0];
        first.chars().count() >= 2 && win.iter().all(|w| w == first)
    });
    let word_counts: Vec<usize> = sentences.iter().map(|(_, _, s)| word_spans(s).len()).collect();
    let uniform_length = has_uniform_sentence_rhythm(&word_counts, MONOTONOUS_MIN_RUN);

    if (repeated_opener || uniform_length)
        && let (Some((first_start, _, _)), Some((_, last_end, _))) = (sentences.first(), sentences.last())
    {
        let message = rhythm_message(group, repeated_opener);
        out.push(Issue { start: *first_start, end: *last_end, kind: IssueKind::Grammar, message: message.to_owned(), suggestions: Vec::new(), lang });
    }
}

fn rhythm_message(group: LangGroup, repeated_opener: bool) -> &'static str {
    match (group, repeated_opener) {
        (LangGroup::Pt, true) => "Três ou mais frases seguidas começam com a mesma palavra; varie as aberturas.",
        (LangGroup::Pt, false) => "Sequência de frases com extensão muito uniforme; alterne frases curtas e médias.",
        (LangGroup::Es, true) => "Tres o más oraciones seguidas empiezan con la misma palabra; varíe las aperturas.",
        (LangGroup::Es, false) => "Secuencia de oraciones con extensión muy uniforme; alterne oraciones cortas y medias.",
        (LangGroup::En, true) => "Three or more consecutive sentences begin with the same word; vary sentence openings.",
        (LangGroup::En, false) => "Consecutive sentences have nearly identical lengths; vary cadence for better flow.",
    }
}

fn is_en_participle(lower: &str) -> bool {
    if EN_IRREGULAR_PARTICIPLES.contains(&lower) {
        return true;
    }
    lower.ends_with("ed") && lower.len() >= 4 && !EN_NOT_PARTICIPLES.contains(&lower)
}

fn is_pt_participle(lower: &str) -> bool {
    if PT_IRREGULAR_PARTICIPLES.contains(&lower) {
        return true;
    }
    let suffix_match = ["ado", "ada", "ados", "adas", "ido", "ida", "idos", "idas"].iter().any(|suf| lower.ends_with(suf));
    suffix_match && lower.chars().count() >= 5 && !PT_NOT_PARTICIPLES.contains(&lower)
}

fn sentence_spans(text: &str) -> Vec<(usize, usize, &str)> {
    let mut sentences = Vec::new();
    let mut start = 0usize;
    let mut chars = text.char_indices().peekable();
    while let Some((idx, ch)) = chars.next() {
        if !matches!(ch, '.' | '!' | '?' | '…') {
            continue;
        }
        let end = idx + ch.len_utf8();
        if chars.peek().is_none_or(|(_, next)| next.is_whitespace()) {
            push_trimmed_sentence(text, start, Some(end), &mut sentences);
            start = end;
        }
    }
    if start < text.len() {
        push_trimmed_sentence(text, start, None, &mut sentences);
    }
    sentences
}

fn push_trimmed_sentence<'a>(text: &'a str, start: usize, end: Option<usize>, out: &mut Vec<(usize, usize, &'a str)>) {
    let raw = match end {
        Some(e) => text.get(start..e),
        None => text.get(start..),
    };
    let Some(raw) = raw else {
        return;
    };
    let trimmed = raw.trim();
    if trimmed.is_empty() {
        return;
    }
    let lead = raw.len() - raw.trim_start().len();
    let s = start + lead;
    out.push((s, s + trimmed.len(), trimmed));
}

fn gap_is_plain_spaces(text: &str, left_end: usize, right_start: usize) -> bool {
    text.get(left_end..right_start).is_some_and(|gap| !gap.is_empty() && gap.chars().all(|c| c == ' '))
}
