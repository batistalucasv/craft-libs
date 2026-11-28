//! Deep rule-based grammar checks across English, Portuguese, and Spanish:
//! conjunctive-adverb comma splices, subject-verb agreement, countable quantifiers,
//! Portuguese crase & impersonal `haver`/`fazer`, and Spanish dequeísmo & impersonal `haber`.

use crate::{Issue, IssueKind, ProofLang};

use super::{FoldedText, LangGroup, has_word_boundaries, match_case, word_spans};

const EN_CONJUNCTIVE_ADVERBS: &[(&str, &str)] =
    &[("however", "However"), ("therefore", "Therefore"), ("moreover", "Moreover"), ("furthermore", "Furthermore")];

const EN_COUNTABLE_PLURALS: &[&str] = &[
    "apples",
    "books",
    "bugs",
    "changes",
    "clicks",
    "days",
    "errors",
    "files",
    "hours",
    "issues",
    "items",
    "lines",
    "minutes",
    "mistakes",
    "options",
    "pages",
    "people",
    "problems",
    "sentences",
    "steps",
    "tests",
    "users",
    "warnings",
    "words",
];

const PT_CRASE_RULES: &[(&str, &str, &str, &str)] = &[
    ("a medida que", "a medida que", "à medida que", "Locuções conjuntivas proporcionais femininas exigem crase (\"à medida que\")."),
    ("a proporção que", "a proporção que", "à proporção que", "Locuções proporcionais exigem crase (\"à proporção que\")."),
    ("à partir", "à partir", "a partir", "Não se usa crase antes de verbo no infinitivo (\"a partir\")."),
    ("dia à dia", "dia à dia", "dia a dia", "Expressões com palavras repetidas não levam crase (\"dia a dia\")."),
    ("frente à frente", "frente à frente", "frente a frente", "Expressões com palavras repetidas não levam crase (\"frente a frente\")."),
    ("lado à lado", "lado à lado", "lado a lado", "Expressões com palavras repetidas não levam crase (\"lado a lado\")."),
    ("cara à cara", "cara à cara", "cara a cara", "Expressões com palavras repetidas não levam crase (\"cara a cara\")."),
    ("gota à gota", "gota à gota", "gota a gota", "Expressões com palavras repetidas não levam crase (\"gota a gota\")."),
    ("a nível de", "a nível de", "em nível de", "Prefira \"em nível de\" em vez da locução \"a nível de\" na norma culta."),
];

const PT_TIME_UNITS: &[&str] = &["anos", "meses", "dias", "semanas", "horas", "minutos"];
const PT_COUNT_WORDS: &[&str] = &[
    "dois", "duas", "três", "quatro", "cinco", "seis", "sete", "oito", "nove", "dez", "muitos", "muitas", "vários", "várias", "alguns", "algumas",
    "poucos", "poucas",
];

const ES_DEQUEISMO_VERBS: &[&str] = &["pienso", "creo", "opino", "considero", "digo"];

pub(super) fn check(text: &str, folded: &FoldedText, group: LangGroup, lang: ProofLang, out: &mut Vec<Issue>) {
    match group {
        LangGroup::En => {
            check_en_comma_splices(text, folded, lang, out);
            check_en_agreement(text, lang, out);
            check_en_less_fewer(text, lang, out);
        }
        LangGroup::Pt => {
            check_pt_crase(text, folded, lang, out);
            check_pt_existential(text, folded, lang, out);
        }
        LangGroup::Es => {
            check_es_dequeismo(text, folded, lang, out);
            check_es_existential(text, folded, lang, out);
        }
    }
}

fn check_en_comma_splices(text: &str, folded: &FoldedText, lang: ProofLang, out: &mut Vec<Issue>) {
    for &(adv, cap_adv) in EN_CONJUNCTIVE_ADVERBS {
        let needle = format!(", {adv}, ");
        let mut cursor = 0usize;
        while let Some(rel) = folded.lower.get(cursor..).and_then(|s| s.find(&needle)) {
            let start = cursor + rel;
            let end = start + needle.len();
            if let Some((orig_start, orig_end)) = folded.map_range(start, end)
                && ends_with_alnum(text, orig_start)
                && starts_with_alpha(text, orig_end)
            {
                out.push(Issue {
                    start: orig_start,
                    end: orig_end,
                    kind: IssueKind::Grammar,
                    message: format!("Possible comma splice before \"{adv}\"; use \"; {adv}, \" or start a new sentence."),
                    suggestions: vec![format!("; {adv}, "), format!(". {cap_adv}, ")],
                    lang,
                });
            }
            cursor = end;
        }
    }
}

fn ends_with_alnum(text: &str, idx: usize) -> bool {
    text.get(..idx).and_then(|s| s.chars().next_back()).is_some_and(char::is_alphanumeric)
}

fn starts_with_alpha(text: &str, idx: usize) -> bool {
    text.get(idx..).and_then(|s| s.chars().next()).is_some_and(char::is_alphabetic)
}

fn check_en_agreement(text: &str, lang: ProofLang, out: &mut Vec<Issue>) {
    let spans = word_spans(text);
    for idx in 0..spans.len() {
        let Some((prefix_words, message)) = en_agreement_prefix(&spans, idx) else {
            continue;
        };
        let search_start = idx + prefix_words;
        let search_end = (search_start + 4).min(spans.len());
        for &(a_start, a_end, orig) in spans.get(search_start..search_end).unwrap_or(&[]) {
            let lower_cand = orig.to_lowercase();
            if matches!(lower_cand.as_str(), "and" | "who" | "that" | "which" | "is") {
                break;
            }
            if lower_cand == "are" {
                out.push(Issue {
                    start: a_start,
                    end: a_end,
                    kind: IssueKind::Grammar,
                    message: message.to_owned(),
                    suggestions: vec![match_case(orig, "is")],
                    lang,
                });
                break;
            }
        }
    }
}

fn en_agreement_prefix(spans: &[(usize, usize, &str)], idx: usize) -> Option<(usize, &'static str)> {
    let head = spans.get(idx)?.2.to_lowercase();
    if head == "the"
        && spans.get(idx + 1).is_some_and(|w| w.2.eq_ignore_ascii_case("list"))
        && spans.get(idx + 2).is_some_and(|w| w.2.eq_ignore_ascii_case("of"))
    {
        return Some((3, "Use singular verb \"is\" with singular subject \"the list\"."));
    }
    if head == "each"
        && spans.get(idx + 1).is_some_and(|w| w.2.eq_ignore_ascii_case("of"))
        && spans.get(idx + 2).is_some_and(|w| w.2.eq_ignore_ascii_case("the"))
    {
        return Some((3, "Use singular verb \"is\" after singular pronoun \"each\"."));
    }
    if head == "neither" && spans.get(idx + 1).is_some_and(|w| w.2.eq_ignore_ascii_case("of")) {
        return Some((2, "In formal prose, \"neither of ...\" takes singular \"is\"."));
    }
    None
}

fn check_en_less_fewer(text: &str, lang: ProofLang, out: &mut Vec<Issue>) {
    let spans = word_spans(text);
    for pair in spans.windows(2) {
        let [(l_start, l_end, less_word), (n_start, _, noun_word)] = pair else {
            continue;
        };
        if !less_word.eq_ignore_ascii_case("less") {
            continue;
        }
        let gap_ok = text.get(*l_end..*n_start).is_some_and(|g| !g.is_empty() && g.chars().all(|c| c == ' '));
        if !gap_ok {
            continue;
        }
        let noun_lower = noun_word.to_lowercase();
        if EN_COUNTABLE_PLURALS.contains(&noun_lower.as_str()) {
            out.push(Issue {
                start: *l_start,
                end: *l_end,
                kind: IssueKind::Grammar,
                message: format!("Use \"fewer\" instead of \"less\" before countable noun \"{noun_word}\"."),
                suggestions: vec![match_case(less_word, "fewer")],
                lang,
            });
        }
    }
}

fn check_pt_crase(text: &str, folded: &FoldedText, lang: ProofLang, out: &mut Vec<Issue>) {
    for &(needle, target_sub, replacement, explanation) in PT_CRASE_RULES {
        let mut cursor = 0usize;
        while let Some(rel) = folded.lower.get(cursor..).and_then(|s| s.find(needle)) {
            let start = cursor + rel;
            let end = start + needle.len();
            if has_word_boundaries(&folded.lower, start, end)
                && let Some(sub_rel) = needle.find(target_sub)
            {
                let t_start = start + sub_rel;
                let t_end = t_start + target_sub.len();
                if let Some((orig_start, orig_end)) = folded.map_range(t_start, t_end)
                    && let Some(orig) = text.get(orig_start..orig_end)
                {
                    out.push(Issue {
                        start: orig_start,
                        end: orig_end,
                        kind: IssueKind::Grammar,
                        message: explanation.to_owned(),
                        suggestions: vec![match_case(orig, replacement)],
                        lang,
                    });
                }
            }
            cursor = end;
        }
    }
}

fn check_pt_existential(text: &str, folded: &FoldedText, lang: ProofLang, out: &mut Vec<Issue>) {
    let mut cursor = 0usize;
    while let Some(rel) = folded.lower.get(cursor..).and_then(|s| s.find("houveram")) {
        let start = cursor + rel;
        let end = start + "houveram".len();
        let tail_ok = folded.lower.get(end..).is_some_and(|rest| !rest.trim_start().starts_with("por bem"));
        if has_word_boundaries(&folded.lower, start, end)
            && tail_ok
            && let Some((orig_start, orig_end)) = folded.map_range(start, end)
            && let Some(orig) = text.get(orig_start..orig_end)
        {
            out.push(Issue {
                start: orig_start,
                end: orig_end,
                kind: IssueKind::Grammar,
                message: "No sentido de existir, o verbo \"haver\" é impessoal e fica no singular (\"houve\").".to_owned(),
                suggestions: vec![match_case(orig, "houve")],
                lang,
            });
        }
        cursor = end;
    }

    let tokens = raw_space_tokens(text);
    for win in tokens.windows(3) {
        let [(f_start, f_end, verb), (_, _, mid), (_, _, unit)] = win else {
            continue;
        };
        if !verb.eq_ignore_ascii_case("fazem") {
            continue;
        }
        let mid_lower = mid.to_lowercase();
        let unit_lower = unit.trim_end_matches(['.', ',', ';', '!', '?']).to_lowercase();
        let is_count = mid.chars().all(|c| c.is_ascii_digit()) || PT_COUNT_WORDS.contains(&mid_lower.as_str());
        if is_count && PT_TIME_UNITS.contains(&unit_lower.as_str()) {
            out.push(Issue {
                start: *f_start,
                end: *f_end,
                kind: IssueKind::Grammar,
                message: "Indicando tempo decorrido, o verbo \"fazer\" é impessoal e permanece no singular (\"faz\").".to_owned(),
                suggestions: vec![match_case(verb, "faz")],
                lang,
            });
        }
    }
}

fn check_es_dequeismo(text: &str, folded: &FoldedText, lang: ProofLang, out: &mut Vec<Issue>) {
    for &verb in ES_DEQUEISMO_VERBS {
        let needle = format!("{verb} de que");
        let mut cursor = 0usize;
        while let Some(rel) = folded.lower.get(cursor..).and_then(|s| s.find(&needle)) {
            let start = cursor + rel;
            let end = start + needle.len();
            if has_word_boundaries(&folded.lower, start, end) {
                let de_que_start = start + verb.len() + 1;
                if let Some((orig_start, orig_end)) = folded.map_range(de_que_start, end)
                    && let Some(orig) = text.get(orig_start..orig_end)
                {
                    out.push(Issue {
                        start: orig_start,
                        end: orig_end,
                        kind: IssueKind::Grammar,
                        message: format!("El verbo \"{verb}\" introduce subordinadas con \"que\", sin la preposición \"de\" (\"{verb} que\")."),
                        suggestions: vec![match_case(orig, "que")],
                        lang,
                    });
                }
            }
            cursor = end;
        }
    }
}

fn check_es_existential(text: &str, folded: &FoldedText, lang: ProofLang, out: &mut Vec<Issue>) {
    let mut cursor = 0usize;
    while let Some(rel) = folded.lower.get(cursor..).and_then(|s| s.find("hubieron")) {
        let start = cursor + rel;
        let end = start + "hubieron".len();
        let next_word = folded
            .lower
            .get(end..)
            .and_then(|rest| rest.split_whitespace().next())
            .map(|w| w.trim_end_matches(|c: char| !c.is_alphabetic()))
            .unwrap_or("");
        if has_word_boundaries(&folded.lower, start, end)
            && !looks_spanish_participle(next_word)
            && let Some((orig_start, orig_end)) = folded.map_range(start, end)
            && let Some(orig) = text.get(orig_start..orig_end)
        {
            out.push(Issue {
                start: orig_start,
                end: orig_end,
                kind: IssueKind::Grammar,
                message: "Con valor existencial, \"haber\" es impersonal y se usa en singular (\"hubo\").".to_owned(),
                suggestions: vec![match_case(orig, "hubo")],
                lang,
            });
        }
        cursor = end;
    }
}

fn looks_spanish_participle(word: &str) -> bool {
    word.ends_with("ado") || word.ends_with("ido") || matches!(word, "hecho" | "visto" | "escrito" | "dicho" | "puesto")
}

fn raw_space_tokens(text: &str) -> Vec<(usize, usize, &str)> {
    let mut tokens = Vec::new();
    let mut start = None;
    for (idx, ch) in text.char_indices() {
        if ch.is_whitespace() {
            if let Some(s) = start.take()
                && let Some(slice) = text.get(s..idx)
            {
                tokens.push((s, idx, slice));
            }
        } else if start.is_none() {
            start = Some(idx);
        }
    }
    if let Some(s) = start
        && let Some(slice) = text.get(s..)
    {
        tokens.push((s, text.len(), slice));
    }
    tokens
}
