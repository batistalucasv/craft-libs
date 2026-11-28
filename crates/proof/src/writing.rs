//! Rule-based writing checks across English, Portuguese, and Spanish:
//! balanced delimiters and quotation marks, bracket/punctuation spacing,
//! conservative contextual paronyms and homophones, deep grammar, clarity,
//! readability metrics, open writing quality scoring, and tone analysis.
//!
//! All checks operate directly on `&str` and [`ProofLang`] with conservative
//! guards so numbers, URLs, contractions, measurements, emoticons, list
//! markers, and inline code stay quiet.

mod clarity;
mod grammar_deep;
mod scoring;
mod tone;

use crate::{Issue, IssueKind, ProofLang};

pub use scoring::{ReadabilityLevel, ReadabilityMetrics, WritingScore};
pub use tone::{TextSpan, ToneProfile, ToneSignal, ToneSignalKind};

/// Complete deterministic writing analysis output for a text.
#[derive(Clone, Debug, PartialEq)]
pub struct WritingReport {
    /// Overall open writing quality score (`0..=100`).
    pub score: WritingScore,
    /// Open readability formulas and reading/speaking time estimates.
    pub readability: ReadabilityMetrics,
    /// Top dominant tone signals (up to 3) with evidence spans.
    pub tone: ToneProfile,
    /// Total word count in the analyzed text.
    pub words: usize,
    /// Total sentence count in the analyzed text.
    pub sentences: usize,
    /// Non-empty paragraph count in the analyzed text.
    pub paragraphs: usize,
    /// Total proofing issues detected (`issues.len()`).
    pub issue_count: usize,
    /// All proofing issues across paragraphs with document-relative UTF-8 byte offsets.
    pub issues: Vec<Issue>,
}

/// Runs deterministic spelling, grammar, clarity, readability, score, and tone
/// analysis on `text` for `lang`.
pub fn analyze_writing(text: &str, lang: ProofLang) -> WritingReport {
    let counts = scoring::measure_text(text);
    let readability = scoring::compute_readability(text, counts, lang);
    let tone = tone::analyze_tone(text);

    if text.trim().is_empty() {
        return WritingReport {
            score: WritingScore::default(),
            readability,
            tone,
            words: counts.words,
            sentences: counts.sentences,
            paragraphs: counts.paragraphs,
            issue_count: 0,
            issues: Vec::new(),
        };
    }

    let mut issues = Vec::new();
    for (para_start, para_text) in paragraph_slices(text) {
        let mut found = crate::check_text(para_text, &[(0..para_text.len(), lang)]);
        for issue in &mut found {
            issue.start = issue.start.saturating_add(para_start);
            issue.end = issue.end.saturating_add(para_start);
        }
        issues.extend(found);
    }

    let score = WritingScore::compute(&issues, counts.words);
    let issue_count = issues.len();

    WritingReport { score, readability, tone, words: counts.words, sentences: counts.sentences, paragraphs: counts.paragraphs, issue_count, issues }
}

fn paragraph_slices(text: &str) -> Vec<(usize, &str)> {
    let mut slices = Vec::new();
    let mut offset = 0usize;
    for raw_line in text.split_inclusive('\n') {
        let trimmed = raw_line.trim_end_matches(['\r', '\n']);
        if !trimmed.trim().is_empty() {
            slices.push((offset, trimmed));
        }
        offset = offset.saturating_add(raw_line.len());
    }
    if offset < text.len()
        && let Some(tail) = text.get(offset..)
        && !tail.trim().is_empty()
    {
        slices.push((offset, tail));
    }
    slices
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum LangGroup {
    En,
    Pt,
    Es,
}

fn lang_group(lang: ProofLang) -> Option<LangGroup> {
    if lang.is_english() {
        Some(LangGroup::En)
    } else if lang == ProofLang::Pt || lang.built_in() == ProofLang::Pt {
        Some(LangGroup::Pt)
    } else if matches!(lang, ProofLang::Es | ProofLang::EsMx | ProofLang::Es419 | ProofLang::EsUy) {
        Some(LangGroup::Es)
    } else {
        None
    }
}

struct ContextualPattern {
    needle: &'static str,
    target_sub: &'static str,
    replacement: &'static str,
    message: &'static str,
}

const EN_PATTERNS: &[ContextualPattern] = &[
    ContextualPattern {
        needle: "their is",
        target_sub: "their",
        replacement: "there",
        message: "Use \"there\" as the existential pronoun before \"is\".",
    },
    ContextualPattern {
        needle: "their are",
        target_sub: "their",
        replacement: "there",
        message: "Use \"there\" as the existential pronoun before \"are\".",
    },
    ContextualPattern {
        needle: "their was",
        target_sub: "their",
        replacement: "there",
        message: "Use \"there\" as the existential pronoun before \"was\".",
    },
    ContextualPattern {
        needle: "their were",
        target_sub: "their",
        replacement: "there",
        message: "Use \"there\" as the existential pronoun before \"were\".",
    },
    ContextualPattern {
        needle: "over their",
        target_sub: "their",
        replacement: "there",
        message: "Use \"there\" to indicate place (\"over there\").",
    },
    ContextualPattern { needle: "there own", target_sub: "there", replacement: "their", message: "Use possessive \"their\" before \"own\"." },
    ContextualPattern { needle: "there house", target_sub: "there", replacement: "their", message: "Use possessive \"their\" before \"house\"." },
    ContextualPattern { needle: "there team", target_sub: "there", replacement: "their", message: "Use possessive \"their\" before \"team\"." },
    ContextualPattern {
        needle: "there going to",
        target_sub: "there",
        replacement: "they're",
        message: "Use contraction \"they're\" (\"they are\") before \"going to\".",
    },
    ContextualPattern {
        needle: "your welcome",
        target_sub: "your",
        replacement: "you're",
        message: "Use contraction \"you're\" (\"you are\") in \"you're welcome\".",
    },
    ContextualPattern { needle: "you're name", target_sub: "you're", replacement: "your", message: "Use possessive \"your\" before \"name\"." },
    ContextualPattern { needle: "you're team", target_sub: "you're", replacement: "your", message: "Use possessive \"your\" before \"team\"." },
    ContextualPattern {
        needle: "it's own",
        target_sub: "it's",
        replacement: "its",
        message: "Use possessive \"its\" (without apostrophe) before \"own\".",
    },
    ContextualPattern {
        needle: "its been",
        target_sub: "its",
        replacement: "it's",
        message: "Use contraction \"it's\" (\"it has\") before \"been\".",
    },
    ContextualPattern { needle: "its a", target_sub: "its", replacement: "it's", message: "Use contraction \"it's\" (\"it is\") before \"a\"." },
    ContextualPattern { needle: "its an", target_sub: "its", replacement: "it's", message: "Use contraction \"it's\" (\"it is\") before \"an\"." },
    ContextualPattern { needle: "better then", target_sub: "then", replacement: "than", message: "Use \"than\" for comparisons after \"better\"." },
    ContextualPattern { needle: "more then", target_sub: "then", replacement: "than", message: "Use \"than\" for comparisons after \"more\"." },
    ContextualPattern { needle: "less then", target_sub: "then", replacement: "than", message: "Use \"than\" for comparisons after \"less\"." },
    ContextualPattern { needle: "rather then", target_sub: "then", replacement: "than", message: "Use \"than\" after \"rather\"." },
    ContextualPattern { needle: "other then", target_sub: "then", replacement: "than", message: "Use \"than\" after \"other\"." },
    ContextualPattern { needle: "greater then", target_sub: "then", replacement: "than", message: "Use \"than\" for comparisons after \"greater\"." },
    ContextualPattern { needle: "the affect", target_sub: "affect", replacement: "effect", message: "Use noun \"effect\" after \"the\"." },
    ContextualPattern { needle: "an affect", target_sub: "affect", replacement: "effect", message: "Use noun \"effect\" after \"an\"." },
    ContextualPattern { needle: "side affect", target_sub: "affect", replacement: "effect", message: "Use noun \"effect\" in \"side effect\"." },
    ContextualPattern {
        needle: "cause and affect",
        target_sub: "affect",
        replacement: "effect",
        message: "Use noun \"effect\" in \"cause and effect\".",
    },
];

const PT_PATTERNS: &[ContextualPattern] = &[
    ContextualPattern {
        needle: "cada vez mas",
        target_sub: "mas",
        replacement: "mais",
        message: "Use o advérbio de intensidade \"mais\" em \"cada vez mais\".",
    },
    ContextualPattern {
        needle: "muito mas",
        target_sub: "mas",
        replacement: "mais",
        message: "Use \"mais\" (intensidade ou quantidade) após \"muito\".",
    },
    ContextualPattern { needle: "bem mas", target_sub: "mas", replacement: "mais", message: "Use \"mais\" (intensidade) após \"bem\"." },
    ContextualPattern {
        needle: "o mas importante",
        target_sub: "mas",
        replacement: "mais",
        message: "Use \"mais\" antes do adjetivo \"importante\".",
    },
    ContextualPattern {
        needle: "mas rápido",
        target_sub: "mas",
        replacement: "mais",
        message: "Use \"mais\" para comparação ou intensidade antes de \"rápido\".",
    },
    ContextualPattern {
        needle: "mas fácil",
        target_sub: "mas",
        replacement: "mais",
        message: "Use \"mais\" para comparação ou intensidade antes de \"fácil\".",
    },
    ContextualPattern { needle: "mas tarde", target_sub: "mas", replacement: "mais", message: "Use \"mais\" na expressão \"mais tarde\"." },
    ContextualPattern { needle: "mas cedo", target_sub: "mas", replacement: "mais", message: "Use \"mais\" na expressão \"mais cedo\"." },
    ContextualPattern {
        needle: "mal humor",
        target_sub: "mal",
        replacement: "mau",
        message: "Use o adjetivo \"mau\" (oposto de \"bom\") antes de \"humor\".",
    },
    ContextualPattern {
        needle: "mal caráter",
        target_sub: "mal",
        replacement: "mau",
        message: "Use o adjetivo \"mau\" (oposto de \"bom\") antes de \"caráter\".",
    },
    ContextualPattern {
        needle: "mau feito",
        target_sub: "mau",
        replacement: "mal",
        message: "Use o advérbio \"mal\" (oposto de \"bem\") antes do particípio \"feito\".",
    },
    ContextualPattern {
        needle: "passou mau",
        target_sub: "mau",
        replacement: "mal",
        message: "Use o advérbio \"mal\" (oposto de \"bem\") com o verbo \"passar\".",
    },
    ContextualPattern {
        needle: "a anos",
        target_sub: "a",
        replacement: "há",
        message: "Use o verbo \"há\" para indicar tempo decorrido (\"há anos\").",
    },
    ContextualPattern {
        needle: "a meses",
        target_sub: "a",
        replacement: "há",
        message: "Use o verbo \"há\" para indicar tempo decorrido (\"há meses\").",
    },
    ContextualPattern {
        needle: "a dias",
        target_sub: "a",
        replacement: "há",
        message: "Use o verbo \"há\" para indicar tempo decorrido (\"há dias\").",
    },
    ContextualPattern {
        needle: "a muito tempo",
        target_sub: "a",
        replacement: "há",
        message: "Use \"há\" para indicar tempo transcorrido (\"há muito tempo\").",
    },
    ContextualPattern {
        needle: "daqui há", target_sub: "há", replacement: "a", message: "Para tempo futuro (\"daqui a\"), use a preposição \"a\"."
    },
    ContextualPattern {
        needle: "há partir de",
        target_sub: "há",
        replacement: "a",
        message: "Na locução \"a partir de\", use a preposição \"a\" sem \"h\".",
    },
    ContextualPattern {
        needle: "aonde fica",
        target_sub: "aonde",
        replacement: "onde",
        message: "Use \"onde\" com verbos estáticos como \"ficar\".",
    },
    ContextualPattern {
        needle: "aonde está",
        target_sub: "aonde",
        replacement: "onde",
        message: "Use \"onde\" com verbos estáticos como \"estar\".",
    },
    ContextualPattern {
        needle: "seção de cinema",
        target_sub: "seção",
        replacement: "sessão",
        message: "Use \"sessão\" para evento ou período de exibição.",
    },
    ContextualPattern {
        needle: "seção plenária", target_sub: "seção", replacement: "sessão", message: "Use \"sessão\" para reunião ou encontro."
    },
    ContextualPattern {
        needle: "sessão de livros",
        target_sub: "sessão",
        replacement: "seção",
        message: "Use \"seção\" para divisão ou departamento.",
    },
    ContextualPattern {
        needle: "sessão do artigo",
        target_sub: "sessão",
        replacement: "seção",
        message: "Use \"seção\" para parte ou subdivisão de um texto.",
    },
    ContextualPattern {
        needle: "sessão de direitos",
        target_sub: "sessão",
        replacement: "cessão",
        message: "Use \"cessão\" no sentido de ceder ou transferir direitos.",
    },
    ContextualPattern {
        needle: "seção de direitos",
        target_sub: "seção",
        replacement: "cessão",
        message: "Use \"cessão\" no sentido de ceder ou transferir direitos.",
    },
    ContextualPattern {
        needle: "o porque",
        target_sub: "porque",
        replacement: "porquê",
        message: "Substantivado após artigo (\"o porquê\"), escreve-se junto e com acento.",
    },
    ContextualPattern {
        needle: "um porque",
        target_sub: "porque",
        replacement: "porquê",
        message: "Substantivado após artigo (\"um porquê\"), escreve-se junto e com acento.",
    },
];

const ES_PATTERNS: &[ContextualPattern] = &[
    ContextualPattern {
        needle: "haber si",
        target_sub: "haber",
        replacement: "a ver",
        message: "Use la locución \"a ver\" (mirar o comprobar) antes de \"si\".",
    },
    ContextualPattern {
        needle: "haber qué",
        target_sub: "haber",
        replacement: "a ver",
        message: "Use \"a ver qué\" para expresar expectativa o comprobación.",
    },
    ContextualPattern { needle: "debe a ver", target_sub: "a ver", replacement: "haber", message: "Use el verbo \"haber\" tras \"debe\"." },
    ContextualPattern { needle: "puede a ver", target_sub: "a ver", replacement: "haber", message: "Use el verbo \"haber\" tras \"puede\"." },
    ContextualPattern {
        needle: "por hay",
        target_sub: "hay",
        replacement: "ahí",
        message: "Use el adverbio de lugar \"ahí\" en la expresión \"por ahí\".",
    },
    ContextualPattern { needle: "de hay", target_sub: "hay", replacement: "ahí", message: "Use el adverbio \"ahí\" en \"de ahí\"." },
    ContextualPattern {
        needle: "ahí que",
        target_sub: "ahí",
        replacement: "hay",
        message: "Use la forma verbal \"hay\" en la perífrasis \"hay que\".",
    },
    ContextualPattern { needle: "ahí muchos", target_sub: "ahí", replacement: "hay", message: "Use el verbo \"hay\" (haber) antes de \"muchos\"." },
    ContextualPattern { needle: "ahí muchas", target_sub: "ahí", replacement: "hay", message: "Use el verbo \"hay\" (haber) antes de \"muchas\"." },
    ContextualPattern {
        needle: "¡hay!",
        target_sub: "hay",
        replacement: "ay",
        message: "Use la interjección \"ay\" para expresar sorpresa o dolor.",
    },
    ContextualPattern {
        needle: "ha echo",
        target_sub: "echo",
        replacement: "hecho",
        message: "Use el participio \"hecho\" (del verbo \"hacer\") con el auxiliar \"ha\".",
    },
    ContextualPattern {
        needle: "he echo",
        target_sub: "echo",
        replacement: "hecho",
        message: "Use el participio \"hecho\" (del verbo \"hacer\") con el auxiliar \"he\".",
    },
    ContextualPattern {
        needle: "han echo",
        target_sub: "echo",
        replacement: "hecho",
        message: "Use el participio \"hecho\" (del verbo \"hacer\") con el auxiliar \"han\".",
    },
    ContextualPattern {
        needle: "hemos echo",
        target_sub: "echo",
        replacement: "hecho",
        message: "Use el participio \"hecho\" (del verbo \"hacer\") con \"hemos\".",
    },
    ContextualPattern { needle: "fue echo", target_sub: "echo", replacement: "hecho", message: "Use el participio \"hecho\" tras \"fue\"." },
    ContextualPattern { needle: "está echo", target_sub: "echo", replacement: "hecho", message: "Use el participio \"hecho\" tras \"está\"." },
    ContextualPattern {
        needle: "hecho de menos",
        target_sub: "hecho",
        replacement: "echo",
        message: "La expresión \"echar de menos\" usa \"echo\" (sin \"h\").",
    },
    ContextualPattern { needle: "una ves", target_sub: "ves", replacement: "vez", message: "Use el sustantivo \"vez\" en \"una vez\"." },
    ContextualPattern { needle: "otra ves", target_sub: "ves", replacement: "vez", message: "Use el sustantivo \"vez\" en \"otra vez\"." },
    ContextualPattern { needle: "cada ves", target_sub: "ves", replacement: "vez", message: "Use el sustantivo \"vez\" en \"cada vez\"." },
    ContextualPattern { needle: "tal ves", target_sub: "ves", replacement: "vez", message: "Use el sustantivo \"vez\" en \"tal vez\"." },
    ContextualPattern { needle: "en ves de", target_sub: "ves", replacement: "vez", message: "Use el sustantivo \"vez\" en \"en vez de\"." },
    ContextualPattern { needle: "si lo vez", target_sub: "vez", replacement: "ves", message: "Use el verbo \"ves\" (de \"ver\") tras \"si lo\"." },
    ContextualPattern { needle: "como vez", target_sub: "vez", replacement: "ves", message: "Use el verbo \"ves\" (de \"ver\") en \"como ves\"." },
    ContextualPattern {
        needle: "ojalá halla",
        target_sub: "halla",
        replacement: "haya",
        message: "Use \"haya\" (subjuntivo de \"haber\") después de \"ojalá\".",
    },
    ContextualPattern {
        needle: "espero que halla",
        target_sub: "halla",
        replacement: "haya",
        message: "Use \"haya\" (de \"haber\") en \"espero que haya\".",
    },
    ContextualPattern {
        needle: "aunque halla",
        target_sub: "halla",
        replacement: "haya",
        message: "Use \"haya\" (de \"haber\") después de \"aunque\".",
    },
    ContextualPattern {
        needle: "se haya en",
        target_sub: "haya",
        replacement: "halla",
        message: "Use \"halla\" (del verbo \"hallar\", encontrarse) en \"se halla en\".",
    },
    ContextualPattern {
        needle: "se haya ubicado",
        target_sub: "haya",
        replacement: "halla",
        message: "Use \"halla\" (del verbo \"hallar\") en \"se halla ubicado\".",
    },
    ContextualPattern {
        needle: "si no que",
        target_sub: "si no",
        replacement: "sino",
        message: "Use la conjunción adversativa \"sino\" en \"sino que\".",
    },
    ContextualPattern {
        needle: "sino vienes",
        target_sub: "sino",
        replacement: "si no",
        message: "Use \"si no\" (condicional + negación) antes de \"vienes\".",
    },
    ContextualPattern {
        needle: "sino llueve",
        target_sub: "sino",
        replacement: "si no",
        message: "Use \"si no\" (condicional + negación) antes de \"llueve\".",
    },
    ContextualPattern {
        needle: "sino puedes",
        target_sub: "sino",
        replacement: "si no",
        message: "Use \"si no\" (condicional + negación) antes de \"puedes\".",
    },
    ContextualPattern {
        needle: "¿porque",
        target_sub: "porque",
        replacement: "por qué",
        message: "En preguntas directas, use \"por qué\" separado y con tilde.",
    },
    ContextualPattern {
        needle: "no sé porque",
        target_sub: "porque",
        replacement: "por qué",
        message: "En interrogativas indirectas (\"no sé por qué\"), use \"por qué\".",
    },
    ContextualPattern {
        needle: "dime porque",
        target_sub: "porque",
        replacement: "por qué",
        message: "En interrogativas indirectas (\"dime por qué\"), use \"por qué\".",
    },
];

const EN_REDUNDANCIES: &[(&str, &str)] = &[
    ("due to the fact that", "because"),
    ("at this point in time", "now"),
    ("for the purpose of", "for"),
    ("each and every", "each"),
    ("in order to", "to"),
    ("very unique", "unique"),
];

const PT_REDUNDANCIES: &[(&str, &str)] = &[
    ("devido ao fato de que", "porque"),
    ("com o intuito de", "para"),
    ("entrar para dentro", "entrar"),
    ("sair para fora", "sair"),
    ("elo de ligação", "elo"),
    ("há anos atrás", "há anos"),
    ("subir para cima", "subir"),
    ("descer para baixo", "descer"),
];

const ES_REDUNDANCIES: &[(&str, &str)] =
    &[("subir para arriba", "subir"), ("bajar para abajo", "bajar"), ("entrar para adentro", "entrar"), ("salir para afuera", "salir")];

/// Runs conservative delimiter/quote, spacing, contextual paronym,
/// redundancy, deep grammar, and clarity checks for English, Portuguese, and Spanish text.
pub(crate) fn check(text: &str, lang: ProofLang) -> Vec<Issue> {
    let Some(group) = lang_group(lang) else {
        return Vec::new();
    };
    if text.is_empty() {
        return Vec::new();
    }
    let mut out = Vec::new();
    check_balanced_delimiters(text, group, lang, &mut out);
    check_balanced_straight_double_quotes(text, group, lang, &mut out);
    check_bracket_spacing(text, group, lang, &mut out);
    check_missing_space_after_comma_semicolon(text, group, lang, &mut out);

    let folded = FoldedText::new(text);
    check_contextual(text, &folded, group, lang, &mut out);
    check_redundancies(text, &folded, group, lang, &mut out);

    let mut deep = Vec::new();
    grammar_deep::check(text, &folded, group, lang, &mut deep);
    push_non_overlapping(&mut out, deep);

    let mut clear = Vec::new();
    clarity::check(text, &folded, group, lang, &mut clear);
    push_clarity(&mut out, clear);

    out.sort_by_key(|i| (i.start, i.end));
    out
}

fn push_non_overlapping(out: &mut Vec<Issue>, extra: Vec<Issue>) {
    for issue in extra {
        if !out.iter().any(|existing| existing.start < issue.end && issue.start < existing.end) {
            out.push(issue);
        }
    }
}

fn push_clarity(out: &mut Vec<Issue>, extra: Vec<Issue>) {
    for issue in extra {
        let exact_dup = out.iter().any(|existing| existing.start == issue.start && existing.end == issue.end);
        // Sentence- and paragraph-level cadence notes carry no replacements; avoid letting a short
        // word-level rule inside the sentence hide them, while still guarding exact duplicates and
        // overlapping word/phrase rules.
        let overlaps_word_rule = !issue.suggestions.is_empty() && out.iter().any(|existing| existing.start < issue.end && issue.start < existing.end);
        if !exact_dup && !overlaps_word_rule {
            out.push(issue);
        }
    }
}

/// Lowercased text paired with a map back to valid UTF-8 byte boundaries in the
/// original string so arbitrary Unicode input never breaks byte slicing.
pub(super) struct FoldedText {
    pub(super) lower: String,
    orig_bounds: Vec<usize>,
}

impl FoldedText {
    fn new(text: &str) -> Self {
        let mut lower = String::with_capacity(text.len());
        let mut orig_bounds = Vec::with_capacity(text.len() + 1);
        for (orig_start, ch) in text.char_indices() {
            let mut first = true;
            for lc in ch.to_lowercase() {
                let mut buf = [0u8; 4];
                let s = lc.encode_utf8(&mut buf);
                lower.push_str(s);
                for _ in 0..s.len() {
                    if first {
                        orig_bounds.push(orig_start);
                        first = false;
                    } else {
                        orig_bounds.push(usize::MAX);
                    }
                }
            }
        }
        orig_bounds.push(text.len());
        Self { lower, orig_bounds }
    }

    pub(super) fn map_range(&self, lower_start: usize, lower_end: usize) -> Option<(usize, usize)> {
        let orig_start = *self.orig_bounds.get(lower_start)?;
        let orig_end = *self.orig_bounds.get(lower_end)?;
        (orig_start != usize::MAX && orig_end != usize::MAX && orig_start <= orig_end).then_some((orig_start, orig_end))
    }
}

fn check_balanced_delimiters(text: &str, group: LangGroup, lang: ProofLang, out: &mut Vec<Issue>) {
    let mut stack: Vec<(char, usize)> = Vec::new();
    let mut in_backticks = false;

    for (idx, ch) in text.char_indices() {
        if ch == '`' {
            in_backticks = !in_backticks;
            continue;
        }
        if in_backticks || inside_url(text, idx) {
            continue;
        }
        if matches!(ch, '(' | ')') && is_emoticon_paren(text, idx) {
            continue;
        }
        if matches!(ch, '(' | '[' | '“' | '«') {
            stack.push((ch, idx));
            continue;
        }
        let expected_open = match ch {
            ')' => Some('('),
            ']' => Some('['),
            '”' => Some('“'),
            '»' => Some('«'),
            _ => None,
        };
        let Some(open) = expected_open else {
            continue;
        };
        if ch == ')' && is_list_marker_close(text, idx) {
            continue;
        }
        if let Some(pos) = stack.iter().rposition(|(c, _)| *c == open) {
            for (unclosed, u_idx) in stack.drain((pos + 1)..) {
                out.push(unclosed_issue(unclosed, u_idx, group, lang));
            }
            stack.pop();
        } else {
            out.push(unmatched_close_issue(ch, idx, group, lang));
        }
    }

    for (unclosed, idx) in stack {
        out.push(unclosed_issue(unclosed, idx, group, lang));
    }
}

fn is_emoticon_paren(text: &str, idx: usize) -> bool {
    let Some(before) = text.get(..idx) else {
        return false;
    };
    for prefix in [":-", ";-", ":", ";", "="] {
        if let Some(rest) = before.strip_suffix(prefix) {
            return rest.chars().next_back().is_none_or(|c| c.is_whitespace() || matches!(c, '(' | '['));
        }
    }
    false
}

fn is_list_marker_close(text: &str, close_idx: usize) -> bool {
    let Some(before) = text.get(..close_idx) else {
        return false;
    };
    let line_start = before.rfind('\n').map_or(0, |n| n + 1);
    let prefix = before.get(line_start..).unwrap_or("").trim_start();
    let len = prefix.len();
    (1..=3).contains(&len) && (prefix.chars().all(|c| c.is_ascii_digit()) || len == 1 && prefix.chars().all(|c| c.is_ascii_alphabetic()))
}

fn unclosed_issue(ch: char, idx: usize, group: LangGroup, lang: ProofLang) -> Issue {
    let message = match group {
        LangGroup::Pt => format!("Delimitador \"{ch}\" aberto sem fechamento correspondente."),
        LangGroup::Es => format!("Se abrió \"{ch}\" sin un delimitador de cierre correspondiente."),
        LangGroup::En => format!("Opening \"{ch}\" does not have a matching closing delimiter."),
    };
    Issue { start: idx, end: idx.saturating_add(ch.len_utf8()), kind: IssueKind::Grammar, message, suggestions: Vec::new(), lang }
}

fn unmatched_close_issue(ch: char, idx: usize, group: LangGroup, lang: ProofLang) -> Issue {
    let message = match group {
        LangGroup::Pt => format!("Delimitador \"{ch}\" sem abertura correspondente."),
        LangGroup::Es => format!("El delimitador \"{ch}\" aparece sin apertura correspondiente."),
        LangGroup::En => format!("Closing \"{ch}\" does not have a matching opening delimiter."),
    };
    Issue { start: idx, end: idx.saturating_add(ch.len_utf8()), kind: IssueKind::Grammar, message, suggestions: Vec::new(), lang }
}

fn check_balanced_straight_double_quotes(text: &str, group: LangGroup, lang: ProofLang, out: &mut Vec<Issue>) {
    let mut in_backticks = false;
    let mut positions = Vec::new();
    for (idx, ch) in text.char_indices() {
        if ch == '`' {
            in_backticks = !in_backticks;
            continue;
        }
        if in_backticks || ch != '"' || inside_url(text, idx) {
            continue;
        }
        let prev = text.get(..idx).and_then(|s| s.chars().next_back());
        let next = text.get(idx + 1..).and_then(|s| s.chars().next());
        let is_inch_mark = prev.is_some_and(|c| c.is_ascii_digit()) && !next.is_some_and(char::is_alphabetic);
        if !is_inch_mark {
            positions.push(idx);
        }
    }

    if positions.len() % 2 == 1
        && let Some(&last_idx) = positions.last()
    {
        let message = match group {
            LangGroup::Pt => "Há uma aspa dupla sem par correspondente neste parágrafo.".to_owned(),
            LangGroup::Es => "Falta una comilla doble de cierre o apertura en este párrafo.".to_owned(),
            LangGroup::En => "Unmatched straight double quotation mark in this paragraph.".to_owned(),
        };
        out.push(Issue { start: last_idx, end: last_idx.saturating_add(1), kind: IssueKind::Grammar, message, suggestions: Vec::new(), lang });
    }
}

fn check_bracket_spacing(text: &str, group: LangGroup, lang: ProofLang, out: &mut Vec<Issue>) {
    let mut in_backticks = false;
    let chars: Vec<(usize, char)> = text.char_indices().collect();
    for i in 0..chars.len() {
        let (idx, ch) = chars[i];
        if ch == '`' {
            in_backticks = !in_backticks;
            continue;
        }
        if in_backticks || inside_url(text, idx) {
            continue;
        }
        if matches!(ch, '(' | '[')
            && let Some(&(sp_idx, ' ')) = chars.get(i + 1)
            && let Some(&(_, after_sp)) = chars.get(i + 2)
            && !after_sp.is_whitespace()
        {
            let message = match group {
                LangGroup::Pt => format!("Remova o espaço extra logo após \"{ch}\"."),
                LangGroup::Es => format!("Elimine el espacio extra después de \"{ch}\"."),
                LangGroup::En => format!("Remove the extra space immediately after \"{ch}\"."),
            };
            out.push(Issue { start: idx, end: sp_idx.saturating_add(1), kind: IssueKind::Grammar, message, suggestions: vec![ch.to_string()], lang });
        }

        if matches!(ch, ')' | ']')
            && i >= 2
            && let (Some(&(sp_idx, ' ')), Some(&(_, before_sp))) = (chars.get(i - 1), chars.get(i - 2))
            && !before_sp.is_whitespace()
            && !matches!(before_sp, '(' | '[')
        {
            let message = match group {
                LangGroup::Pt => format!("Remova o espaço extra antes de \"{ch}\"."),
                LangGroup::Es => format!("Elimine el espacio extra antes de \"{ch}\"."),
                LangGroup::En => format!("Remove the extra space immediately before \"{ch}\"."),
            };
            out.push(Issue { start: sp_idx, end: idx.saturating_add(1), kind: IssueKind::Grammar, message, suggestions: vec![ch.to_string()], lang });
        }
    }
}

fn check_missing_space_after_comma_semicolon(text: &str, group: LangGroup, lang: ProofLang, out: &mut Vec<Issue>) {
    let mut in_backticks = false;
    let chars: Vec<(usize, char)> = text.char_indices().collect();
    for i in 1..chars.len().saturating_sub(1) {
        let (idx, ch) = chars[i];
        if ch == '`' {
            in_backticks = !in_backticks;
            continue;
        }
        if in_backticks || !matches!(ch, ',' | ';') {
            continue;
        }
        let prev = chars[i - 1].1;
        let next = chars[i + 1].1;
        if (prev.is_alphabetic() || matches!(prev, ')' | ']' | '”' | '»')) && next.is_alphabetic() && !inside_url(text, idx) {
            let message = match group {
                LangGroup::Pt => format!("Insira um espaço após \"{ch}\"."),
                LangGroup::Es => format!("Agregue un espacio después de \"{ch}\"."),
                LangGroup::En => format!("Add a space after \"{ch}\"."),
            };
            out.push(Issue { start: idx, end: idx.saturating_add(1), kind: IssueKind::Grammar, message, suggestions: vec![format!("{ch} ")], lang });
        }
    }
}

fn inside_url(text: &str, idx: usize) -> bool {
    let Some(before) = text.get(..idx) else {
        return false;
    };
    let token_start = before.rfind(char::is_whitespace).map_or(0, |p| p + 1);
    let after = text.get(idx..).unwrap_or("");
    let token_end = after.find(char::is_whitespace).map_or(text.len(), |p| idx + p);
    let token = text.get(token_start..token_end).unwrap_or("");
    token.contains("://") || token.starts_with("www.")
}

fn check_contextual(text: &str, folded: &FoldedText, group: LangGroup, lang: ProofLang, out: &mut Vec<Issue>) {
    let patterns = match group {
        LangGroup::En => EN_PATTERNS,
        LangGroup::Pt => PT_PATTERNS,
        LangGroup::Es => ES_PATTERNS,
    };
    for pattern in patterns {
        let mut cursor = 0usize;
        while let Some(rel) = folded.lower.get(cursor..).and_then(|s| s.find(pattern.needle)) {
            let start = cursor + rel;
            let end = start + pattern.needle.len();
            if has_word_boundaries(&folded.lower, start, end)
                && let Some(sub_rel) = pattern.needle.find(pattern.target_sub)
            {
                let sub_start = start + sub_rel;
                let sub_end = sub_start + pattern.target_sub.len();
                if let Some((orig_start, orig_end)) = folded.map_range(sub_start, sub_end)
                    && let Some(orig) = text.get(orig_start..orig_end)
                {
                    let suggestion = match_case(orig, pattern.replacement);
                    out.push(Issue {
                        start: orig_start,
                        end: orig_end,
                        kind: IssueKind::Grammar,
                        message: pattern.message.to_owned(),
                        suggestions: vec![suggestion],
                        lang,
                    });
                }
            }
            cursor = end;
        }
    }
    if group == LangGroup::Pt {
        check_pt_end_of_question(text, folded, lang, out);
    }
}

fn check_pt_end_of_question(text: &str, folded: &FoldedText, lang: ProofLang, out: &mut Vec<Issue>) {
    for (needle, target_sub) in [("porque?", "porque"), ("por que?", "por que")] {
        let mut cursor = 0usize;
        while let Some(rel) = folded.lower.get(cursor..).and_then(|s| s.find(needle)) {
            let start = cursor + rel;
            let target_end = start + target_sub.len();
            if left_boundary(&folded.lower, start)
                && let Some((orig_start, orig_end)) = folded.map_range(start, target_end)
                && let Some(orig) = text.get(orig_start..orig_end)
            {
                let suggestion = match_case(orig, "por quê");
                out.push(Issue {
                    start: orig_start,
                    end: orig_end,
                    kind: IssueKind::Grammar,
                    message: "Em fim de frase antes de ponto de interrogação, use \"por quê\".".to_owned(),
                    suggestions: vec![suggestion],
                    lang,
                });
            }
            cursor = start + needle.len();
        }
    }
}

fn check_redundancies(text: &str, folded: &FoldedText, group: LangGroup, lang: ProofLang, out: &mut Vec<Issue>) {
    let table = match group {
        LangGroup::En => EN_REDUNDANCIES,
        LangGroup::Pt => PT_REDUNDANCIES,
        LangGroup::Es => ES_REDUNDANCIES,
    };
    for &(phrase, replacement) in table {
        let mut cursor = 0usize;
        while let Some(rel) = folded.lower.get(cursor..).and_then(|s| s.find(phrase)) {
            let start = cursor + rel;
            let end = start + phrase.len();
            if has_word_boundaries(&folded.lower, start, end)
                && let Some((orig_start, orig_end)) = folded.map_range(start, end)
                && let Some(orig) = text.get(orig_start..orig_end)
            {
                let suggestion = match_case(orig, replacement);
                let message = match group {
                    LangGroup::Pt => format!("Expressão redundante; prefira \"{suggestion}\"."),
                    LangGroup::Es => format!("Expresión redundante; prefiera \"{suggestion}\"."),
                    LangGroup::En => format!("Wordy phrasing; consider \"{suggestion}\"."),
                };
                out.push(Issue { start: orig_start, end: orig_end, kind: IssueKind::Grammar, message, suggestions: vec![suggestion], lang });
            }
            cursor = end;
        }
    }
}

pub(super) fn left_boundary(text: &str, start: usize) -> bool {
    text.get(..start).and_then(|s| s.chars().next_back()).is_none_or(|c| !c.is_alphanumeric())
}

pub(super) fn has_word_boundaries(text: &str, start: usize, end: usize) -> bool {
    let left_ok = left_boundary(text, start);
    let right_ok = text.get(end..).and_then(|s| s.chars().next()).is_none_or(|c| !c.is_alphanumeric());
    left_ok && right_ok
}

pub(super) fn match_case(original: &str, replacement: &str) -> String {
    if original.chars().next().is_some_and(char::is_uppercase) {
        let mut chars = replacement.chars();
        match chars.next() {
            Some(first) => first.to_uppercase().chain(chars).collect(),
            None => String::new(),
        }
    } else {
        replacement.to_owned()
    }
}

pub(super) fn word_spans(text: &str) -> Vec<(usize, usize, &str)> {
    let mut spans = Vec::new();
    let mut start: Option<usize> = None;
    for (idx, ch) in text.char_indices() {
        let is_word = ch.is_alphanumeric() || ch == '-' || ch == '\'';
        match (start, is_word) {
            (None, true) => start = Some(idx),
            (Some(s), false) => {
                push_trimmed_word(text, s, idx, &mut spans);
                start = None;
            }
            _ => {}
        }
    }
    if let Some(s) = start {
        push_trimmed_word(text, s, text.len(), &mut spans);
    }
    spans
}

fn push_trimmed_word<'a>(text: &'a str, raw_start: usize, raw_end: usize, out: &mut Vec<(usize, usize, &'a str)>) {
    let Some(slice) = text.get(raw_start..raw_end) else {
        return;
    };
    let trimmed_start = slice.trim_start_matches(['-', '\'']);
    let lead = slice.len() - trimmed_start.len();
    let trimmed = trimmed_start.trim_end_matches(['-', '\'']);
    if trimmed.is_empty() || !trimmed.chars().any(char::is_alphabetic) {
        return;
    }
    let start = raw_start + lead;
    let end = start + trimmed.len();
    out.push((start, end, trimmed));
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{Issue, check_grammar_in, check_text};

    fn spans<'a>(text: &'a str, issues: &[Issue]) -> Vec<(&'a str, Vec<String>)> {
        issues.iter().map(|i| (&text[i.start..i.end], i.suggestions.clone())).collect()
    }

    #[test]
    fn delimiter_quote_and_spacing_checks_across_en_pt_es() {
        let en = check_grammar_in("Review the (unfinished note.", ProofLang::En);
        assert!(en.iter().any(|i| i.kind == IssueKind::Grammar && i.message.contains("matching closing")), "{en:?}");

        let pt = check_grammar_in("Ele disse “olá sem fechar.", ProofLang::Pt);
        assert!(pt.iter().any(|i| i.message.contains("fechamento")), "{pt:?}");

        let es = check_grammar_in("Cita «incompleta en español.", ProofLang::Es);
        assert!(es.iter().any(|i| i.message.contains("cierre")), "{es:?}");

        let q = check_grammar_in(r#"She wrote "open quote only."#, ProofLang::En);
        assert!(q.iter().any(|i| i.message.contains("quotation mark")), "{q:?}");

        let spaced = check_grammar_in("Check ( this item ).", ProofLang::En);
        assert_eq!(spans("Check ( this item ).", &spaced), [("( ", vec!["(".into()]), (" )", vec![")".into()])]);

        let comma = check_grammar_in("Olá,mundo;teste.", ProofLang::Pt);
        assert_eq!(spans("Olá,mundo;teste.", &comma), [(",", vec![", ".into()]), (";", vec!["; ".into()])]);
    }

    #[test]
    fn suppresses_false_positives_on_inches_emoticons_urls_numbers_lists_and_code() {
        for (sample, lang) in [
            ("Don't forget O'Brien's 27\" monitor (verified [ok]) :).", ProofLang::En),
            ("1) First item\na) Second item\nThe number is 1,000 and 3.14.", ProofLang::En),
            ("See https://example.com/a,b?x=(1 and `(unclosed` inline code.", ProofLang::En),
            ("Números como 10,5 ou 2,340 estão corretos «com aspas» ;).", ProofLang::Pt),
            ("El valor 1,500 cumple «con estilo» (sin errores) :-).", ProofLang::Es),
        ] {
            let found = check(sample, lang);
            assert!(found.is_empty(), "expected no false positives for {sample:?}, got {found:?}");
        }
    }

    #[test]
    fn contextual_paronyms_across_en_pt_es() {
        let en_text = "Their is a better then average team on it's own, and your welcome.";
        let en = check_grammar_in(en_text, ProofLang::En);
        assert_eq!(
            spans(en_text, &en),
            [("Their", vec!["There".into()]), ("then", vec!["than".into()]), ("it's", vec!["its".into()]), ("your", vec!["you're".into()]),]
        );

        let pt_text = "Ficou cada vez mas claro a anos onde estava, mas não sei o porque. Ele foi porque?";
        let pt = check_grammar_in(pt_text, ProofLang::Pt);
        assert_eq!(
            spans(pt_text, &pt),
            [("mas", vec!["mais".into()]), ("a", vec!["há".into()]), ("porque", vec!["porquê".into()]), ("porque", vec!["por quê".into()]),]
        );

        let es_text = "Haber si ha echo otra ves lo que dijo, sino puedes esperar y dime porque.";
        let es = check_grammar_in(es_text, ProofLang::Es);
        assert_eq!(
            spans(es_text, &es),
            [
                ("Haber", vec!["A ver".into()]),
                ("echo", vec!["hecho".into()]),
                ("ves", vec!["vez".into()]),
                ("sino", vec!["si no".into()]),
                ("porque", vec!["por qué".into()]),
            ]
        );
    }

    #[test]
    fn redundancies_en_and_pt() {
        let en_text = "In order to finish due to the fact that we need it.";
        let en = check_grammar_in(en_text, ProofLang::En);
        assert_eq!(spans(en_text, &en), [("In order to", vec!["To".into()]), ("due to the fact that", vec!["because".into()])]);

        let pt_text = "Vamos subir para cima para fortalecer o elo de ligação.";
        let pt = check_grammar_in(pt_text, ProofLang::Pt);
        assert_eq!(spans(pt_text, &pt), [("subir para cima", vec!["subir".into()]), ("elo de ligação", vec!["elo".into()])]);
    }

    #[test]
    fn preserves_dialect_and_avoids_overlap_with_builtin_rules() {
        let gb = check_text("Their is colour.", &[(0..16, ProofLang::EnGb)]);
        assert!(
            gb.iter().any(|i| i.kind == IssueKind::Grammar && i.lang == ProofLang::EnGb && &gb_slice("Their is colour.", i) == "Their"),
            "{gb:?}"
        );

        // Built-in space-before-punctuation (" ,") overlaps with ",b", so only one mark is emitted on that span.
        let pt = check_grammar_in("Olá ,mundo.", ProofLang::Pt);
        let comma_marks: Vec<_> = pt.iter().filter(|i| i.start <= 5 && i.end >= 4).collect();
        assert_eq!(comma_marks.len(), 1, "{pt:?}");
    }

    #[test]
    fn deep_grammar_checks_and_false_positive_guards() {
        let en = check_grammar_in("We finished early, however, the list of steps are missing less steps.", ProofLang::En);
        assert!(en.iter().any(|i| i.suggestions.contains(&"; however, ".into())), "{en:?}");
        assert!(en.iter().any(|i| i.suggestions == ["is"]), "{en:?}");
        assert!(en.iter().any(|i| i.suggestions == ["fewer"]), "{en:?}");
        assert!(check_grammar_in("However, we need less time.", ProofLang::En).is_empty());

        let pt = check_grammar_in("Houveram falhas à partir de hoje e fazem dois anos.", ProofLang::Pt);
        assert!(pt.iter().any(|i| i.suggestions == ["Houve"]), "{pt:?}");
        assert!(pt.iter().any(|i| i.suggestions == ["a partir"]), "{pt:?}");
        assert!(pt.iter().any(|i| i.suggestions == ["faz"]), "{pt:?}");
        assert!(check_grammar_in("Houveram por bem sair a partir de hoje.", ProofLang::Pt).is_empty());

        let es = check_grammar_in("Pienso de que hubieron muchos cambios.", ProofLang::Es);
        assert!(es.iter().any(|i| i.suggestions == ["que"]), "{es:?}");
        assert!(es.iter().any(|i| i.suggestions == ["hubo"]), "{es:?}");
        assert!(check_grammar_in("Pienso que ya hubieron visto la nota.", ProofLang::Es).is_empty());
    }

    #[test]
    fn clarity_nominalizations_passive_long_and_rhythm() {
        let en = check_grammar_in("We should make a decision after the report was reviewed.", ProofLang::En);
        assert!(en.iter().any(|i| i.suggestions == ["decide"]), "{en:?}");
        assert!(en.iter().any(|i| i.message.contains("Passive voice")), "{en:?}");
        assert!(check_grammar_in("The door is red and the bed was soft.", ProofLang::En).is_empty());

        let pt = check_grammar_in("Vamos fazer uma análise porque o texto foi escrito cedo.", ProofLang::Pt);
        assert!(pt.iter().any(|i| i.suggestions == ["analisar"]), "{pt:?}");
        assert!(pt.iter().any(|i| i.message.contains("voz passiva")), "{pt:?}");
        assert!(check_grammar_in("O projeto está em bom estado hoje.", ProofLang::Pt).is_empty());

        let long = "This sentence deliberately introduces more than thirty ordinary words in one continuous clause so that writers can easily notice when a single sentence carries far too much weight for comfortable reading.";
        let long_issues = check_grammar_in(long, ProofLang::En);
        assert!(long_issues.iter().any(|i| i.message.contains("suggested limit: 30")), "{long_issues:?}");

        let monotonous = "We build tools. We test code. We ship updates.";
        let rhythm_issues = check_grammar_in(monotonous, ProofLang::En);
        assert!(rhythm_issues.iter().any(|i| i.message.contains("same word")), "{rhythm_issues:?}");
    }

    #[test]
    fn analyze_writing_computes_score_readability_and_tones() {
        let empty = analyze_writing("", ProofLang::En);
        assert_eq!(empty.score.score, 100);
        assert_eq!(empty.issue_count, 0);
        assert!(empty.tone.dominant.is_empty());

        let sample = "Please review this urgent report immediately!\nThank you for your help, and therefore we must decide now.";
        let report = analyze_writing(sample, ProofLang::En);
        assert!(report.words >= 15);
        assert_eq!(report.paragraphs, 2);
        assert!(report.readability.flesch_reading_ease > 0.0);
        assert!(report.readability.reading_minutes > 0.0);
        assert!(report.readability.speaking_minutes > 0.0);
        assert!(!report.tone.dominant.is_empty() && report.tone.dominant.len() <= 3);
        for sig in &report.tone.dominant {
            assert!(!sig.evidence_spans.is_empty());
            for span in &sig.evidence_spans {
                assert!(span.slice(sample).is_some());
            }
        }
        assert_eq!(report.score.score, 100, "clean text scores 100: {report:?}");

        let flawed = analyze_writing("Their is less steps and we must make a decision.", ProofLang::En);
        assert!(flawed.issue_count >= 3, "{flawed:?}");
        assert!(flawed.score.score < 100);
        assert!(flawed.score.grammar_issues >= 3);
    }

    fn gb_slice(s: &str, i: &Issue) -> String {
        s[i.start..i.end].to_string()
    }
}
