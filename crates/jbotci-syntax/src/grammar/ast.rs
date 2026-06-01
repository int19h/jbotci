//! Syntax AST behavior and parser-facing helpers.

pub use crate::tree::*;

#[allow(unused_imports)]
use bityzba::{data, ensures, invariant, new, requires};
use serde::Serialize;
use serde::ser::{SerializeSeq, Serializer};

impl<T> WithFreeModifiers<T> {
    #[requires(true)]
    #[ensures(true)]
    pub fn new(value: T, free_modifiers: Vec<FreeModifierSyntax>) -> Self {
        Self {
            value,
            free_modifiers,
        }
    }
}

impl<T: Serialize> Serialize for WithFreeModifiers<T> {
    #[requires(true)]
    #[ensures(true)]
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        if self.free_modifiers.is_empty() {
            return self.value.serialize(serializer);
        }
        let mut seq = serializer.serialize_seq(Some(1 + self.free_modifiers.len()))?;
        seq.serialize_element(&self.value)?;
        for free_modifier in &self.free_modifiers {
            seq.serialize_element(free_modifier)?;
        }
        seq.end()
    }
}

impl WithFreeModifiers<Token> {
    #[requires(true)]
    #[ensures(true)]
    pub fn extend_words_into(self, out: &mut Vec<Token>) {
        out.push(self.value);
        for free_modifier in self.free_modifiers {
            free_modifier.extend_words_into(out);
        }
    }

    #[requires(true)]
    #[ensures(true)]
    pub fn visit_words(&self, visitor: &mut impl FnMut(&Token)) {
        visitor(&self.value);
        for free_modifier in &self.free_modifiers {
            free_modifier.visit_words(visitor);
        }
    }

    #[requires(true)]
    #[ensures(ret >= 1)]
    pub fn word_count(&self) -> usize {
        1 + self
            .free_modifiers
            .iter()
            .map(FreeModifierSyntax::word_count)
            .sum::<usize>()
    }

    #[requires(true)]
    #[ensures(ret.is_some())]
    pub fn first_word(&self) -> Option<&Token> {
        Some(&self.value)
    }

    #[requires(true)]
    #[ensures(true)]
    pub fn words(self) -> Vec<Token> {
        let mut words = Vec::new();
        self.extend_words_into(&mut words);
        words
    }
}

impl WithFreeModifiers<Vec<Token>> {
    #[requires(true)]
    #[ensures(true)]
    pub fn extend_words_into(self, out: &mut Vec<Token>) {
        out.extend(self.value);
        for free_modifier in self.free_modifiers {
            free_modifier.extend_words_into(out);
        }
    }

    #[requires(true)]
    #[ensures(true)]
    pub fn visit_words(&self, visitor: &mut impl FnMut(&Token)) {
        for word in &self.value {
            visitor(word);
        }
        for free_modifier in &self.free_modifiers {
            free_modifier.visit_words(visitor);
        }
    }

    #[requires(true)]
    #[ensures(ret >= self.value.len())]
    pub fn word_count(&self) -> usize {
        self.value.len()
            + self
                .free_modifiers
                .iter()
                .map(FreeModifierSyntax::word_count)
                .sum::<usize>()
    }

    #[requires(true)]
    #[ensures(true)]
    pub fn first_word(&self) -> Option<&Token> {
        self.value.first().or_else(|| {
            self.free_modifiers
                .iter()
                .find_map(FreeModifierSyntax::first_word)
        })
    }

    #[requires(true)]
    #[ensures(true)]
    pub fn words(self) -> Vec<Token> {
        let mut words = Vec::new();
        self.extend_words_into(&mut words);
        words
    }
}

impl WithFreeModifiers<WordRun> {
    #[requires(true)]
    #[ensures(true)]
    pub fn extend_words_into(self, out: &mut Vec<Token>) {
        out.extend(self.value);
        for free_modifier in self.free_modifiers {
            free_modifier.extend_words_into(out);
        }
    }

    #[requires(true)]
    #[ensures(true)]
    pub fn visit_words(&self, visitor: &mut impl FnMut(&Token)) {
        for word in &self.value {
            visitor(word);
        }
        for free_modifier in &self.free_modifiers {
            free_modifier.visit_words(visitor);
        }
    }

    #[requires(true)]
    #[ensures(ret >= self.value.len())]
    pub fn word_count(&self) -> usize {
        self.value.len()
            + self
                .free_modifiers
                .iter()
                .map(FreeModifierSyntax::word_count)
                .sum::<usize>()
    }

    #[requires(true)]
    #[ensures(ret.is_some())]
    pub fn first_word(&self) -> Option<&Token> {
        Some(self.value.first())
    }

    #[requires(true)]
    #[ensures(true)]
    pub fn words(self) -> Vec<Token> {
        let mut words = Vec::new();
        self.extend_words_into(&mut words);
        words
    }
}

#[requires(true)]
#[ensures(true)]
fn visit_word_slice(words: &[Token], visitor: &mut impl FnMut(&Token)) {
    for word in words {
        visitor(word);
    }
}

impl StatementSyntax {
    #[requires(true)]
    #[ensures(true)]
    pub fn words(self) -> Vec<Token> {
        match self.into_data() {
            data!(StatementSyntax::Tuhe {
                tense_modal,
                tuhe,
                text,
                tuhu,
            }) => {
                let mut words = Vec::new();
                if let Some(tense_modal) = tense_modal {
                    words.extend(tense_modal.words());
                }
                words.extend(tuhe.words());
                words.extend(text.words());
                if let Some(tuhu) = tuhu {
                    words.extend(tuhu.words());
                }
                words
            }
            data!(StatementSyntax::Prenex {
                prenex_terms,
                zohu,
                inner_statement,
            }) => {
                let mut words = prenex_terms
                    .into_iter()
                    .flat_map(TermSyntax::words)
                    .collect::<Vec<_>>();
                words.extend(zohu.words());
                words.extend(inner_statement.words());
                words
            }
            data!(StatementSyntax::Predicate(predicate)) => predicate.words(),
            data!(StatementSyntax::Connected {
                i,
                connective,
                leading_statement,
                trailing_statement,
            }) => {
                let mut words = leading_statement.words();
                words.push(i);
                words.extend(connective.words());
                words.extend(trailing_statement.words());
                words
            }
            data!(StatementSyntax::PreIConnected {
                connective,
                i,
                leading_statement,
                trailing_statement,
            }) => {
                let mut words = leading_statement.words();
                words.extend(connective.words());
                words.push(i);
                words.extend(trailing_statement.words());
                words
            }
            data!(StatementSyntax::Iau {
                inner_statement,
                iau,
                reset_terms,
            }) => {
                let mut words = inner_statement.words();
                words.extend(iau.words());
                for term in reset_terms {
                    words.extend(term.words());
                }
                words
            }
            data!(StatementSyntax::ExperimentalPredicateContinuation {
                leading_statement,
                continuation,
            }) => {
                let mut words = leading_statement.words();
                words.extend(continuation.words());
                words
            }
            data!(StatementSyntax::Fragment(fragment)) => fragment.words(),
        }
    }
}

impl PredicateStatementContinuationSyntax {
    #[requires(true)]
    #[ensures(true)]
    pub fn words(self) -> Vec<Token> {
        let mut words = self.connective.words();
        if let Some(tense_modal) = self.tense_modal {
            words.extend(tense_modal.words());
        }
        match self.marker.into_data() {
            data!(PredicateStatementContinuationMarkerSyntax::Bo(bo)) => {
                words.extend(bo.words());
                words.extend(self.trailing_subsentence.words());
            }
            data!(PredicateStatementContinuationMarkerSyntax::Ke { ke, kehe }) => {
                words.extend(ke.words());
                words.extend(self.trailing_subsentence.words());
                if let Some(kehe) = kehe {
                    words.extend(kehe.words());
                }
            }
        }
        words
    }
}

impl TextSyntax {
    #[requires(true)]
    #[ensures(true)]
    pub fn words(self) -> Vec<Token> {
        let data!(TextSyntax {
            leading_nai,
            leading_cmevla,
            leading_indicators,
            leading_free_modifiers,
            leading_connective,
            paragraphs,
        }) = self.into_data();
        let mut words = leading_nai;
        words.extend(leading_cmevla);
        for indicator in leading_indicators {
            words.extend(indicator.words());
        }
        for free_modifier in leading_free_modifiers {
            words.extend(free_modifier.words());
        }
        if let Some(leading_connective) = leading_connective {
            words.extend(leading_connective.words());
        }
        for paragraph in paragraphs {
            words.extend(paragraph.words());
        }
        words
    }
}

impl ParagraphSyntax {
    #[requires(true)]
    #[ensures(true)]
    pub fn words(self) -> Vec<Token> {
        let data!(ParagraphSyntax {
            i,
            niho,
            free_modifiers,
            statements,
        }) = self.into_data();
        let mut words = i.into_iter().collect::<Vec<_>>();
        words.extend(niho);
        for free_modifier in free_modifiers {
            words.extend(free_modifier.words());
        }
        for paragraph_statement in statements {
            words.extend(paragraph_statement.words());
        }
        words
    }
}

impl ParagraphStatementSyntax {
    #[requires(true)]
    #[ensures(true)]
    pub fn words(self) -> Vec<Token> {
        let data!(ParagraphStatementSyntax {
            i,
            connective,
            free_modifiers,
            statement,
        }) = self.into_data();
        let mut words = i.into_iter().collect::<Vec<_>>();
        if let Some(connective) = connective {
            words.extend(connective.words());
        }
        for free_modifier in free_modifiers {
            words.extend(free_modifier.words());
        }
        if let Some(statement) = statement {
            words.extend(statement.words());
        }
        words
    }
}

impl FreeModifierSyntax {
    #[requires(true)]
    #[ensures(true)]
    pub fn words(self) -> Vec<Token> {
        match self.into_data() {
            data!(FreeModifierSyntax::Sei {
                sei,
                terms,
                cu,
                relation,
                sehu,
            }) => {
                let mut words = sei.words();
                for term in terms {
                    words.extend(term.words());
                }
                if let Some(cu) = cu {
                    words.extend(cu.words());
                }
                words.extend(relation.words());
                if let Some(sehu) = sehu {
                    words.extend(sehu.words());
                }
                words
            }
            data!(FreeModifierSyntax::To { to, text, toi }) => {
                let mut words = to.words();
                words.extend(text.words());
                if let Some(toi) = toi {
                    words.extend(toi.words());
                }
                words
            }
            data!(FreeModifierSyntax::Xi { xi, expression }) => {
                let mut words = xi.words();
                words.extend(expression.words());
                words
            }
            data!(FreeModifierSyntax::Mai { number, mai }) => {
                let mut words = number.into_vec();
                words.extend(mai.words());
                words
            }
            data!(FreeModifierSyntax::Soi {
                soi,
                leading_argument,
                trailing_argument,
                sehu,
            }) => {
                let mut words = soi.words();
                words.extend(leading_argument.words());
                if let Some(argument) = trailing_argument {
                    words.extend(argument.words());
                }
                if let Some(sehu) = sehu {
                    words.extend(sehu.words());
                }
                words
            }
            data!(FreeModifierSyntax::Vocative {
                vocative_markers,
                argument,
                dohu,
            }) => {
                let mut words = vocative_markers.words();
                if let Some(argument) = argument {
                    words.extend(argument.words());
                }
                if let Some(dohu) = dohu {
                    words.extend(dohu.words());
                }
                words
            }
            data!(FreeModifierSyntax::Replacement {
                lohai,
                old_words,
                sahai,
                new_words,
                lehai,
            }) => {
                let mut words = lohai.into_iter().collect::<Vec<_>>();
                words.extend(old_words);
                words.extend(sahai);
                words.extend(new_words);
                words.extend(lehai.words());
                words
            }
        }
    }
}

impl PredicateSyntax {
    #[requires(true)]
    #[ensures(true)]
    pub fn words(self) -> Vec<Token> {
        let data!(PredicateSyntax {
            leading_terms,
            cu,
            predicate_tail,
            free_modifiers,
        }) = self.into_data();
        let mut words = Vec::new();
        for term in leading_terms {
            words.extend(term.words());
        }
        if let Some(cu) = cu {
            words.extend(cu.words());
        }
        words.extend(predicate_tail.words());
        for free_modifier in free_modifiers {
            words.extend(free_modifier.words());
        }
        words
    }
}

impl PredicateTailSyntax {
    #[requires(true)]
    #[ensures(true)]
    pub fn words(self) -> Vec<Token> {
        let mut words = self.first.words();
        if let Some(ke_continuation) = self.ke_continuation {
            words.extend(ke_continuation.words());
        }
        words
    }
}

impl KePredicateTailSyntax {
    #[requires(true)]
    #[ensures(true)]
    pub fn words(self) -> Vec<Token> {
        let data!(KePredicateTailSyntax {
            connective,
            tense_modal,
            ke,
            predicate_tail,
            kehe,
            tail_terms,
            vau,
            free_modifiers,
        }) = self.into_data();
        let mut words = connective.words();
        if let Some(tense_modal) = tense_modal {
            words.extend(tense_modal.words());
        }
        words.extend(ke.words());
        words.extend(predicate_tail.words());
        if let Some(kehe) = kehe {
            words.extend(kehe.words());
        }
        for term in tail_terms {
            words.extend(term.words());
        }
        if let Some(vau) = vau {
            words.extend(vau.words());
        }
        for free_modifier in free_modifiers {
            words.extend(free_modifier.words());
        }
        words
    }
}

impl PredicateTail1Syntax {
    #[requires(true)]
    #[ensures(true)]
    pub fn words(self) -> Vec<Token> {
        let mut words = self.first.words();
        for continuation in self.continuations {
            words.extend(continuation.words());
        }
        words
    }
}

impl PredicateTailContinuationSyntax {
    #[requires(true)]
    #[ensures(true)]
    pub fn words(self) -> Vec<Token> {
        let data!(PredicateTailContinuationSyntax {
            connective,
            tense_modal,
            cu,
            predicate_tail,
            tail_terms,
            vau,
            free_modifiers,
        }) = self.into_data();
        let mut words = connective.words();
        if let Some(tense_modal) = tense_modal {
            words.extend(tense_modal.words());
        }
        if let Some(cu) = cu {
            words.extend(cu.words());
        }
        words.extend(predicate_tail.words());
        for term in tail_terms {
            words.extend(term.words());
        }
        if let Some(vau) = vau {
            words.extend(vau.words());
        }
        for free_modifier in free_modifiers {
            words.extend(free_modifier.words());
        }
        words
    }
}

impl PredicateTail2Syntax {
    #[requires(true)]
    #[ensures(true)]
    pub fn words(self) -> Vec<Token> {
        let mut words = self.first.words();
        if let Some(bo_continuation) = self.bo_continuation {
            words.extend(bo_continuation.words());
        }
        words
    }
}

impl BoPredicateTailSyntax {
    #[requires(true)]
    #[ensures(true)]
    pub fn words(self) -> Vec<Token> {
        let data!(BoPredicateTailSyntax {
            connective,
            tense_modal,
            bo,
            cu,
            predicate_tail,
            tail_terms,
            vau,
            free_modifiers,
        }) = self.into_data();
        let mut words = connective.words();
        if let Some(tense_modal) = tense_modal {
            words.extend(tense_modal.words());
        }
        words.extend(bo.words());
        if let Some(cu) = cu {
            words.extend(cu.words());
        }
        words.extend(predicate_tail.words());
        for term in tail_terms {
            words.extend(term.words());
        }
        if let Some(vau) = vau {
            words.extend(vau.words());
        }
        for free_modifier in free_modifiers {
            words.extend(free_modifier.words());
        }
        words
    }
}

impl PredicateTail3Syntax {
    #[requires(true)]
    #[ensures(true)]
    pub fn words(self) -> Vec<Token> {
        match self.into_data() {
            data!(PredicateTail3Syntax::Relation {
                relation,
                terms,
                vau,
                free_modifiers,
            }) => {
                let mut words = relation.words();
                for term in terms {
                    words.extend(term.words());
                }
                if let Some(vau) = vau {
                    words.extend(vau.words());
                }
                for free_modifier in free_modifiers {
                    words.extend(free_modifier.words());
                }
                words
            }
            data!(PredicateTail3Syntax::GekSentence(gek_sentence)) => gek_sentence.words(),
        }
    }
}

impl GekSentenceSyntax {
    #[requires(true)]
    #[ensures(true)]
    pub fn words(self) -> Vec<Token> {
        match self.into_data() {
            data!(GekSentenceSyntax::Pair {
                gek,
                first,
                gik,
                second,
                gihi,
                tail_terms,
                vau,
                free_modifiers,
            }) => {
                let mut words = gek.words();
                words.extend(first.words());
                words.extend(gik.words());
                words.extend(second.words());
                if let Some(gihi) = gihi {
                    words.push(gihi);
                }
                for term in tail_terms {
                    words.extend(term.words());
                }
                if let Some(vau) = vau {
                    words.extend(vau.words());
                }
                for free_modifier in free_modifiers {
                    words.extend(free_modifier.words());
                }
                words
            }
            data!(GekSentenceSyntax::Ke {
                tense_modal,
                ke,
                inner,
                kehe,
            }) => {
                let mut words = Vec::new();
                if let Some(tense_modal) = tense_modal {
                    words.extend(tense_modal.words());
                }
                words.extend(ke.words());
                words.extend(inner.words());
                if let Some(kehe) = kehe {
                    words.extend(kehe.words());
                }
                words
            }
            data!(GekSentenceSyntax::Na { na, inner }) => {
                let mut words = na.words();
                words.extend(inner.words());
                words
            }
        }
    }
}

impl SubsentenceSyntax {
    #[requires(true)]
    #[ensures(true)]
    pub fn visit_words(&self, visitor: &mut impl FnMut(&Token)) {
        match self.as_data() {
            data!(SubsentenceSyntax::Plain(predicate)) => predicate.visit_words(visitor),
            data!(SubsentenceSyntax::Prenex {
                prenex_terms,
                zohu,
                inner_subsentence,
            }) => {
                for term in prenex_terms {
                    term.visit_words(visitor);
                }
                zohu.visit_words(visitor);
                inner_subsentence.visit_words(visitor);
            }
        }
    }
}

impl FragmentSyntax {
    #[requires(true)]
    #[ensures(true)]
    pub fn visit_words(&self, visitor: &mut impl FnMut(&Token)) {
        match self.as_data() {
            data!(FragmentSyntax::Ek(connective)) | data!(FragmentSyntax::Gihek(connective)) => {
                connective.visit_words(visitor);
            }
            data!(FragmentSyntax::Other(words)) => words.visit_words(visitor),
            data!(FragmentSyntax::Ijek { i, connective }) => {
                visitor(i);
                connective.visit_words(visitor);
            }
            data!(FragmentSyntax::Prenex { terms, zohu }) => {
                for term in terms {
                    term.visit_words(visitor);
                }
                zohu.visit_words(visitor);
            }
            data!(FragmentSyntax::BeLink {
                be,
                fa,
                first_argument,
                bei_links,
                beho,
            }) => {
                be.visit_words(visitor);
                if let Some(fa) = fa {
                    fa.visit_words(visitor);
                }
                if let Some(first_argument) = first_argument {
                    first_argument.visit_words(visitor);
                }
                for bei_link in bei_links {
                    bei_link.visit_words(visitor);
                }
                if let Some(beho) = beho {
                    beho.visit_words(visitor);
                }
            }
            data!(FragmentSyntax::BeiLink(bei_only_links)) => {
                for bei_link in bei_only_links {
                    bei_link.visit_words(visitor);
                }
            }
            data!(FragmentSyntax::RelativeClause(relative_clauses)) => {
                for relative_clause in relative_clauses {
                    relative_clause.visit_words(visitor);
                }
            }
            data!(FragmentSyntax::MathExpression(math_expression)) => {
                math_expression.visit_words(visitor)
            }
            data!(FragmentSyntax::Term { terms, vau }) => {
                for term in terms {
                    term.visit_words(visitor);
                }
                if let Some(vau) = vau {
                    vau.visit_words(visitor);
                }
            }
            data!(FragmentSyntax::Relation(relation)) => relation.visit_words(visitor),
        }
    }
}

impl TermSyntax {
    #[requires(true)]
    #[ensures(true)]
    pub fn visit_words(&self, visitor: &mut impl FnMut(&Token)) {
        match self.as_data() {
            data!(TermSyntax::NuhiTermset {
                nuhi,
                termset,
                nuhu,
            }) => {
                nuhi.visit_words(visitor);
                for term in termset {
                    term.visit_words(visitor);
                }
                if let Some(nuhu) = nuhu {
                    nuhu.visit_words(visitor);
                }
            }
            data!(TermSyntax::GekNuhiTermset {
                m_nuhi,
                gek,
                terms,
                nuhu,
                gik,
                gik_terms,
                gihi,
                gik_nuhu,
            }) => {
                if let Some(nuhi) = m_nuhi {
                    nuhi.visit_words(visitor);
                }
                gek.visit_words(visitor);
                for term in terms {
                    term.visit_words(visitor);
                }
                if let Some(nuhu) = nuhu {
                    nuhu.visit_words(visitor);
                }
                gik.visit_words(visitor);
                for term in gik_terms {
                    term.visit_words(visitor);
                }
                if let Some(gihi) = gihi {
                    visitor(gihi);
                }
                if let Some(nuhu) = gik_nuhu {
                    nuhu.visit_words(visitor);
                }
            }
            data!(TermSyntax::Cehe {
                leading_terms,
                cehe,
                trailing_terms,
            }) => {
                for term in leading_terms {
                    term.visit_words(visitor);
                }
                cehe.visit_words(visitor);
                for term in trailing_terms {
                    term.visit_words(visitor);
                }
            }
            data!(TermSyntax::Pehe {
                leading_terms,
                pehe,
                connective,
                trailing_terms,
            }) => {
                for term in leading_terms {
                    term.visit_words(visitor);
                }
                pehe.visit_words(visitor);
                connective.visit_words(visitor);
                for term in trailing_terms {
                    term.visit_words(visitor);
                }
            }
            data!(TermSyntax::Argument(argument)) => argument.visit_words(visitor),
            data!(TermSyntax::Fa { fa, argument, ku }) => {
                fa.visit_words(visitor);
                argument.visit_words(visitor);
                if let Some(ku) = ku {
                    ku.visit_words(visitor);
                }
            }
            data!(TermSyntax::NaKu { na, na_ku }) => {
                visitor(na);
                na_ku.visit_words(visitor);
            }
            data!(TermSyntax::BareNa(na)) => na.visit_words(visitor),
            data!(TermSyntax::NoihaAdverbial {
                noiha,
                tail_elements,
                relation,
                relative_clauses,
                fehu,
            }) => {
                noiha.visit_words(visitor);
                for tail_element in tail_elements {
                    tail_element.visit_words(visitor);
                }
                if let Some(relation) = relation {
                    relation.visit_words(visitor);
                }
                for relative_clause in relative_clauses {
                    relative_clause.visit_words(visitor);
                }
                if let Some(fehu) = fehu {
                    fehu.visit_words(visitor);
                }
            }
            data!(TermSyntax::PoihaBrigahi {
                poiha,
                tail_elements,
                relation,
                relative_clauses,
                brigahi_ku,
            }) => {
                poiha.visit_words(visitor);
                for tail_element in tail_elements {
                    tail_element.visit_words(visitor);
                }
                if let Some(relation) = relation {
                    relation.visit_words(visitor);
                }
                for relative_clause in relative_clauses {
                    relative_clause.visit_words(visitor);
                }
                brigahi_ku.visit_words(visitor);
            }
            data!(TermSyntax::FihoiAdverbial {
                fihoi,
                subsentence,
                fihau,
            }) => {
                fihoi.visit_words(visitor);
                subsentence.visit_words(visitor);
                if let Some(fihau) = fihau {
                    fihau.visit_words(visitor);
                }
            }
            data!(TermSyntax::SoiAdverbial {
                soi,
                subsentence,
                sehu,
            }) => {
                soi.visit_words(visitor);
                subsentence.visit_words(visitor);
                if let Some(sehu) = sehu {
                    sehu.visit_words(visitor);
                }
            }
            data!(TermSyntax::JaiTagged { jai, tag, argument }) => {
                jai.visit_words(visitor);
                if let Some(tag) = tag {
                    tag.visit_words(visitor);
                }
                argument.visit_words(visitor);
            }
            data!(TermSyntax::Tagged {
                tense_modal,
                argument,
            }) => {
                if let Some(tense_modal) = tense_modal {
                    tense_modal.visit_words(visitor);
                }
                argument.visit_words(visitor);
            }
            data!(TermSyntax::Connected {
                leading_terms,
                connective,
                trailing_terms,
            }) => {
                for term in leading_terms {
                    term.visit_words(visitor);
                }
                connective.visit_words(visitor);
                for term in trailing_terms {
                    term.visit_words(visitor);
                }
            }
            data!(TermSyntax::BoConnected {
                leading_terms,
                bo_connective,
                tense_modal,
                bo,
                trailing_term,
            }) => {
                for term in leading_terms {
                    term.visit_words(visitor);
                }
                if let Some(connective) = bo_connective {
                    connective.visit_words(visitor);
                }
                if let Some(tense_modal) = tense_modal {
                    tense_modal.visit_words(visitor);
                }
                bo.visit_words(visitor);
                trailing_term.visit_words(visitor);
            }
        }
    }
}

impl ArgumentTagSyntax {
    #[requires(true)]
    #[ensures(true)]
    pub fn visit_words(&self, visitor: &mut impl FnMut(&Token)) {
        match self.as_data() {
            data!(ArgumentTagSyntax::TenseModal(tense_modal)) => tense_modal.visit_words(visitor),
            data!(ArgumentTagSyntax::Fa(fa)) => fa.visit_words(visitor),
        }
    }
}

impl MathExpressionSyntax {
    #[requires(true)]
    #[ensures(true)]
    pub fn visit_words(&self, visitor: &mut impl FnMut(&Token)) {
        match self.as_data() {
            data!(MathExpressionSyntax::Number(quantifier)) => quantifier.visit_words(visitor),
            data!(MathExpressionSyntax::Letter { letter, boi }) => {
                letter.visit_words(visitor);
                if let Some(boi) = boi {
                    boi.visit_words(visitor);
                }
            }
            data!(MathExpressionSyntax::Vei {
                vei,
                inner_expression,
                veho,
            }) => {
                vei.visit_words(visitor);
                inner_expression.visit_words(visitor);
                if let Some(veho) = veho {
                    veho.visit_words(visitor);
                }
            }
            data!(MathExpressionSyntax::Gek {
                gek,
                left_expression,
                gik,
                right_expression,
            }) => {
                gek.visit_words(visitor);
                left_expression.visit_words(visitor);
                gik.visit_words(visitor);
                right_expression.visit_words(visitor);
            }
            data!(MathExpressionSyntax::Forethought {
                peho,
                operator,
                operands,
                kuhe,
            }) => {
                if let Some(peho) = peho {
                    peho.visit_words(visitor);
                }
                operator.visit_words(visitor);
                for operand in operands {
                    operand.visit_words(visitor);
                }
                if let Some(kuhe) = kuhe {
                    kuhe.visit_words(visitor);
                }
            }
            data!(MathExpressionSyntax::ReversePolish {
                fuha,
                operands,
                operators,
            }) => {
                fuha.visit_words(visitor);
                for operand in operands {
                    operand.visit_words(visitor);
                }
                for operator in operators {
                    operator.visit_words(visitor);
                }
            }
            data!(MathExpressionSyntax::Nihe {
                nihe,
                relation,
                tehu,
            }) => {
                nihe.visit_words(visitor);
                relation.visit_words(visitor);
                if let Some(tehu) = tehu {
                    tehu.visit_words(visitor);
                }
            }
            data!(MathExpressionSyntax::Mohe {
                mohe,
                argument,
                tehu,
            }) => {
                mohe.visit_words(visitor);
                argument.visit_words(visitor);
                if let Some(tehu) = tehu {
                    tehu.visit_words(visitor);
                }
            }
            data!(MathExpressionSyntax::Johi {
                johi,
                expressions,
                tehu,
            }) => {
                johi.visit_words(visitor);
                for expression in expressions {
                    expression.visit_words(visitor);
                }
                if let Some(tehu) = tehu {
                    tehu.visit_words(visitor);
                }
            }
            data!(MathExpressionSyntax::Lahe {
                markers,
                inner_expression,
                luhu,
            }) => {
                markers.visit_words(visitor);
                inner_expression.visit_words(visitor);
                if let Some(luhu) = luhu {
                    luhu.visit_words(visitor);
                }
            }
            data!(MathExpressionSyntax::Connected {
                left_expression,
                connective,
                right_expression,
            }) => {
                left_expression.visit_words(visitor);
                connective.visit_words(visitor);
                right_expression.visit_words(visitor);
            }
            data!(MathExpressionSyntax::Binary {
                operator,
                left_expression,
                right_expression,
            }) => {
                left_expression.visit_words(visitor);
                operator.visit_words(visitor);
                right_expression.visit_words(visitor);
            }
            data!(MathExpressionSyntax::Bihe {
                left_expression,
                bihe,
                operator,
                right_expression,
            }) => {
                left_expression.visit_words(visitor);
                bihe.visit_words(visitor);
                operator.visit_words(visitor);
                right_expression.visit_words(visitor);
            }
        }
    }
}

impl ArgumentSyntax {
    #[requires(true)]
    #[ensures(true)]
    pub fn visit_words(&self, visitor: &mut impl FnMut(&Token)) {
        match self.as_data() {
            data!(ArgumentSyntax::Quote(quote)) => quote.visit_words(visitor),
            data!(ArgumentSyntax::MathExpression {
                li,
                expression,
                loho,
            }) => {
                li.visit_words(visitor);
                expression.visit_words(visitor);
                if let Some(loho) = loho {
                    loho.visit_words(visitor);
                }
            }
            data!(ArgumentSyntax::Letter { letter, boi }) => {
                letter.visit_words(visitor);
                if let Some(boi) = boi {
                    boi.visit_words(visitor);
                }
            }
            data!(ArgumentSyntax::Quantified {
                quantifier,
                inner_argument,
            }) => {
                quantifier.visit_words(visitor);
                inner_argument.visit_words(visitor);
            }
            data!(ArgumentSyntax::RelativeClause {
                base_argument,
                vuho,
                relative_clauses,
            }) => {
                base_argument.visit_words(visitor);
                if let Some(vuho) = vuho {
                    vuho.visit_words(visitor);
                }
                for relative_clause in relative_clauses {
                    relative_clause.visit_words(visitor);
                }
            }
            data!(ArgumentSyntax::Vuho {
                base_argument,
                vuho_marker,
                relative_clauses,
                connected_argument,
            }) => {
                base_argument.visit_words(visitor);
                vuho_marker.visit_words(visitor);
                for relative_clause in relative_clauses {
                    relative_clause.visit_words(visitor);
                }
                if let Some(connected_argument) = connected_argument {
                    connected_argument.connective.visit_words(visitor);
                    connected_argument.argument.visit_words(visitor);
                }
            }
            data!(ArgumentSyntax::BridiDescription {
                lohoi,
                subsentence,
                kuhau,
            }) => {
                lohoi.visit_words(visitor);
                subsentence.visit_words(visitor);
                if let Some(kuhau) = kuhau {
                    kuhau.visit_words(visitor);
                }
            }
            data!(ArgumentSyntax::NaKu { na, ku }) => {
                visitor(na);
                ku.visit_words(visitor);
            }
            data!(ArgumentSyntax::Tagged {
                tag,
                inner_argument,
            }) => {
                tag.visit_words(visitor);
                inner_argument.visit_words(visitor);
            }
            data!(ArgumentSyntax::NaheBo {
                nahe,
                bo,
                inner_argument,
                luhu,
            }) => {
                visitor(nahe);
                bo.visit_words(visitor);
                inner_argument.visit_words(visitor);
                if let Some(luhu) = luhu {
                    luhu.visit_words(visitor);
                }
            }
            data!(ArgumentSyntax::Nahe {
                nahe,
                inner_argument,
                luhu,
            }) => {
                nahe.visit_words(visitor);
                inner_argument.visit_words(visitor);
                if let Some(luhu) = luhu {
                    luhu.visit_words(visitor);
                }
            }
            data!(ArgumentSyntax::TermWrapped {
                wrapper,
                wrapper_bo,
                inner_term,
                luhu,
                ..
            }) => {
                wrapper.visit_words(visitor);
                if let Some(wrapper_bo) = wrapper_bo {
                    wrapper_bo.visit_words(visitor);
                }
                inner_term.visit_words(visitor);
                if let Some(luhu) = luhu {
                    luhu.visit_words(visitor);
                }
            }
            data!(ArgumentSyntax::Koha(koha)) => koha.visit_words(visitor),
            data!(ArgumentSyntax::Zohe {
                tag,
                maybe_ku,
                free_modifiers,
            }) => {
                if let Some(tag) = tag {
                    tag.visit_words(visitor);
                }
                if let Some(ku) = maybe_ku {
                    ku.visit_words(visitor);
                }
                for free_modifier in free_modifiers {
                    free_modifier.visit_words(visitor);
                }
            }
            data!(ArgumentSyntax::Lahe {
                lahe,
                relative_clauses,
                inner_argument,
                luhu,
            }) => {
                lahe.visit_words(visitor);
                for relative_clause in relative_clauses {
                    relative_clause.visit_words(visitor);
                }
                inner_argument.visit_words(visitor);
                if let Some(luhu) = luhu {
                    luhu.visit_words(visitor);
                }
            }
            data!(ArgumentSyntax::Connected {
                leading_argument,
                connective,
                trailing_argument,
            }) => {
                leading_argument.visit_words(visitor);
                connective.visit_words(visitor);
                trailing_argument.visit_words(visitor);
            }
            data!(ArgumentSyntax::Ke {
                ke,
                inner_argument,
                kehe,
            }) => {
                ke.visit_words(visitor);
                inner_argument.visit_words(visitor);
                if let Some(kehe) = kehe {
                    kehe.visit_words(visitor);
                }
            }
            data!(ArgumentSyntax::Bo {
                leading_argument,
                bo_connective,
                bo_tense_modal,
                bo,
                trailing_argument,
            }) => {
                leading_argument.visit_words(visitor);
                if let Some(connective) = bo_connective {
                    connective.visit_words(visitor);
                }
                if let Some(tense_modal) = bo_tense_modal {
                    tense_modal.visit_words(visitor);
                }
                bo.visit_words(visitor);
                trailing_argument.visit_words(visitor);
            }
            data!(ArgumentSyntax::Gek {
                gek,
                leading_argument,
                gik,
                trailing_argument,
                gihi,
            }) => {
                gek.visit_words(visitor);
                leading_argument.visit_words(visitor);
                gik.visit_words(visitor);
                trailing_argument.visit_words(visitor);
                if let Some(gihi) = gihi {
                    visitor(gihi);
                }
            }
            data!(ArgumentSyntax::Descriptor(descriptor)) => descriptor.visit_words(visitor),
            data!(ArgumentSyntax::ConnectedDescriptor(connected_descriptor)) => {
                connected_descriptor.visit_words(visitor);
            }
            data!(ArgumentSyntax::Name { la, names }) => {
                la.visit_words(visitor);
                names.visit_words(visitor);
            }
            data!(ArgumentSyntax::Cmevla(cmevla)) => cmevla.visit_words(visitor),
            data!(ArgumentSyntax::RelationVocative {
                leading_relative_clauses,
                relation,
                trailing_relative_clauses,
            }) => {
                for relative_clause in leading_relative_clauses {
                    relative_clause.visit_words(visitor);
                }
                relation.visit_words(visitor);
                for relative_clause in trailing_relative_clauses {
                    relative_clause.visit_words(visitor);
                }
            }
        }
    }
}

impl GoiRelativeClauseSyntax {
    #[requires(true)]
    #[ensures(true)]
    pub fn visit_words(&self, visitor: &mut impl FnMut(&Token)) {
        self.goi.visit_words(visitor);
        self.argument.visit_words(visitor);
        if let Some(gehu) = &self.gehu {
            gehu.visit_words(visitor);
        }
    }
}

impl SelbriRelativeClauseSyntax {
    #[requires(true)]
    #[ensures(true)]
    pub fn visit_words(&self, visitor: &mut impl FnMut(&Token)) {
        self.nohoi.visit_words(visitor);
        self.relation.visit_words(visitor);
        if let Some(kuhoi) = &self.kuhoi {
            kuhoi.visit_words(visitor);
        }
    }
}

impl RelativeClauseSyntax {
    #[requires(true)]
    #[ensures(true)]
    pub fn visit_words(&self, visitor: &mut impl FnMut(&Token)) {
        match self.as_data() {
            data!(RelativeClauseSyntax::Goi(relative_clause)) => {
                relative_clause.visit_words(visitor)
            }
            data!(RelativeClauseSyntax::Noi {
                noi,
                subsentence,
                kuho,
            })
            | data!(RelativeClauseSyntax::Poi {
                poi: noi,
                subsentence,
                kuho,
            }) => {
                noi.visit_words(visitor);
                subsentence.visit_words(visitor);
                if let Some(kuho) = kuho {
                    kuho.visit_words(visitor);
                }
            }
            data!(RelativeClauseSyntax::Zihe { zihe, inner }) => {
                zihe.visit_words(visitor);
                inner.visit_words(visitor);
            }
            data!(RelativeClauseSyntax::Connected { connective, inner }) => {
                connective.visit_words(visitor);
                inner.visit_words(visitor);
            }
        }
    }
}

impl QuoteSyntax {
    #[requires(true)]
    #[ensures(true)]
    pub fn visit_words(&self, visitor: &mut impl FnMut(&Token)) {
        match self.as_data() {
            data!(QuoteSyntax::Lu { lu, text, lihu }) => {
                lu.visit_words(visitor);
                text.visit_words(visitor);
                if let Some(lihu) = lihu {
                    lihu.visit_words(visitor);
                }
            }
            data!(QuoteSyntax::Zo(zo)) | data!(QuoteSyntax::Zoi(zo)) => zo.visit_words(visitor),
            data!(QuoteSyntax::ZohOi(zohoi)) => zohoi.visit_words(visitor),
            data!(QuoteSyntax::Lohu(lohu)) => lohu.visit_words(visitor),
        }
    }
}

impl DescriptorSyntax {
    #[requires(true)]
    #[ensures(true)]
    pub fn visit_words(&self, visitor: &mut impl FnMut(&Token)) {
        if let Some(quantifier) = &self.outer_quantifier {
            quantifier.visit_words(visitor);
        }
        if let Some(descriptor) = &self.descriptor {
            descriptor.visit_words(visitor);
        }
        for element in &self.tail_elements {
            element.visit_words(visitor);
        }
        if let Some(relation) = &self.relation {
            relation.visit_words(visitor);
        }
        for relative_clause in &self.relative_clauses {
            relative_clause.visit_words(visitor);
        }
        if let Some(ku) = &self.ku {
            ku.visit_words(visitor);
        }
    }
}

impl DescriptorHeadSyntax {
    #[requires(true)]
    #[ensures(true)]
    pub fn visit_words(&self, visitor: &mut impl FnMut(&Token)) {
        self.descriptor.visit_words(visitor);
    }
}

impl ConnectedDescriptorSyntax {
    #[requires(true)]
    #[ensures(true)]
    pub fn visit_words(&self, visitor: &mut impl FnMut(&Token)) {
        self.leading_descriptor_head.visit_words(visitor);
        self.connective.visit_words(visitor);
        self.trailing_descriptor_head.visit_words(visitor);
        for element in &self.tail_elements {
            element.visit_words(visitor);
        }
        if let Some(relation) = &self.relation {
            relation.visit_words(visitor);
        }
        for relative_clause in &self.relative_clauses {
            relative_clause.visit_words(visitor);
        }
        if let Some(ku) = &self.ku {
            ku.visit_words(visitor);
        }
    }
}

#[invariant(true)]
#[derive(Debug)]
pub struct ConnectiveSyntaxParts {
    pub kind: ConnectiveKind,
    pub se: Option<Token>,
    pub nahe: Option<Token>,
    pub na: Option<Token>,
    pub cmavo: WithFreeModifiers<Vec<Token>>,
    pub nai: Option<WithFreeModifiers<Token>>,
}

impl ConnectiveSyntax {
    #[requires(true)]
    #[ensures(true)]
    pub fn new(
        kind: ConnectiveKind,
        se: Option<Token>,
        nahe: Option<Token>,
        na: Option<Token>,
        cmavo: WithFreeModifiers<Vec<Token>>,
        nai: Option<WithFreeModifiers<Token>>,
    ) -> Self {
        match kind {
            ConnectiveKind::Afterthought => new!(ConnectiveSyntax::Afterthought {
                se,
                nahe,
                na,
                cmavo: Box::new(cmavo),
                nai: nai.map(Box::new),
            }),
            ConnectiveKind::Relation => new!(ConnectiveSyntax::Relation {
                se,
                nahe,
                na,
                cmavo: Box::new(cmavo),
                nai: nai.map(Box::new),
            }),
            ConnectiveKind::PredicateTail => new!(ConnectiveSyntax::PredicateTail {
                se,
                nahe,
                na,
                cmavo: Box::new(cmavo),
                nai: nai.map(Box::new),
            }),
            ConnectiveKind::Forethought => new!(ConnectiveSyntax::Forethought {
                se,
                nahe,
                na,
                cmavo: Box::new(cmavo),
                nai: nai.map(Box::new),
            }),
            ConnectiveKind::NonLogical => new!(ConnectiveSyntax::NonLogical {
                se,
                nahe,
                na,
                cmavo: Box::new(cmavo),
                nai: nai.map(Box::new),
            }),
            ConnectiveKind::Interval => new!(ConnectiveSyntax::Interval {
                se,
                nahe,
                na,
                cmavo: Box::new(cmavo),
                nai: nai.map(Box::new),
            }),
        }
    }

    #[requires(true)]
    #[ensures(true)]
    pub fn kind(&self) -> ConnectiveKind {
        match self.as_data() {
            data!(ConnectiveSyntax::Afterthought { .. }) => ConnectiveKind::Afterthought,
            data!(ConnectiveSyntax::Relation { .. }) => ConnectiveKind::Relation,
            data!(ConnectiveSyntax::PredicateTail { .. }) => ConnectiveKind::PredicateTail,
            data!(ConnectiveSyntax::Forethought { .. }) => ConnectiveKind::Forethought,
            data!(ConnectiveSyntax::NonLogical { .. }) => ConnectiveKind::NonLogical,
            data!(ConnectiveSyntax::Interval { .. }) => ConnectiveKind::Interval,
        }
    }

    #[requires(true)]
    #[ensures(true)]
    pub fn cmavo(&self) -> &WithFreeModifiers<Vec<Token>> {
        match self.as_data() {
            data!(ConnectiveSyntax::Afterthought { cmavo, .. })
            | data!(ConnectiveSyntax::Relation { cmavo, .. })
            | data!(ConnectiveSyntax::PredicateTail { cmavo, .. })
            | data!(ConnectiveSyntax::Forethought { cmavo, .. })
            | data!(ConnectiveSyntax::NonLogical { cmavo, .. })
            | data!(ConnectiveSyntax::Interval { cmavo, .. }) => cmavo.as_ref(),
        }
    }

    #[requires(true)]
    #[ensures(true)]
    pub fn into_parts(self) -> ConnectiveSyntaxParts {
        match self.into_data() {
            data!(ConnectiveSyntax::Afterthought {
                se,
                nahe,
                na,
                cmavo,
                nai,
            }) => ConnectiveSyntaxParts {
                kind: ConnectiveKind::Afterthought,
                se,
                nahe,
                na,
                cmavo: *cmavo,
                nai: nai.map(|nai| *nai),
            },
            data!(ConnectiveSyntax::Relation {
                se,
                nahe,
                na,
                cmavo,
                nai,
            }) => ConnectiveSyntaxParts {
                kind: ConnectiveKind::Relation,
                se,
                nahe,
                na,
                cmavo: *cmavo,
                nai: nai.map(|nai| *nai),
            },
            data!(ConnectiveSyntax::PredicateTail {
                se,
                nahe,
                na,
                cmavo,
                nai,
            }) => ConnectiveSyntaxParts {
                kind: ConnectiveKind::PredicateTail,
                se,
                nahe,
                na,
                cmavo: *cmavo,
                nai: nai.map(|nai| *nai),
            },
            data!(ConnectiveSyntax::Forethought {
                se,
                nahe,
                na,
                cmavo,
                nai,
            }) => ConnectiveSyntaxParts {
                kind: ConnectiveKind::Forethought,
                se,
                nahe,
                na,
                cmavo: *cmavo,
                nai: nai.map(|nai| *nai),
            },
            data!(ConnectiveSyntax::NonLogical {
                se,
                nahe,
                na,
                cmavo,
                nai,
            }) => ConnectiveSyntaxParts {
                kind: ConnectiveKind::NonLogical,
                se,
                nahe,
                na,
                cmavo: *cmavo,
                nai: nai.map(|nai| *nai),
            },
            data!(ConnectiveSyntax::Interval {
                se,
                nahe,
                na,
                cmavo,
                nai,
            }) => ConnectiveSyntaxParts {
                kind: ConnectiveKind::Interval,
                se,
                nahe,
                na,
                cmavo: *cmavo,
                nai: nai.map(|nai| *nai),
            },
        }
    }

    #[requires(true)]
    #[ensures(true)]
    pub fn visit_words(&self, visitor: &mut impl FnMut(&Token)) {
        let (se, nahe, na, cmavo, nai) = match self.as_data() {
            data!(ConnectiveSyntax::Afterthought {
                se,
                nahe,
                na,
                cmavo,
                nai,
            })
            | data!(ConnectiveSyntax::Relation {
                se,
                nahe,
                na,
                cmavo,
                nai,
            })
            | data!(ConnectiveSyntax::PredicateTail {
                se,
                nahe,
                na,
                cmavo,
                nai,
            })
            | data!(ConnectiveSyntax::Forethought {
                se,
                nahe,
                na,
                cmavo,
                nai,
            })
            | data!(ConnectiveSyntax::NonLogical {
                se,
                nahe,
                na,
                cmavo,
                nai,
            })
            | data!(ConnectiveSyntax::Interval {
                se,
                nahe,
                na,
                cmavo,
                nai,
            }) => (se, nahe, na, cmavo, nai),
        };
        if let Some(se) = se {
            visitor(se);
        }
        if let Some(nahe) = nahe {
            visitor(nahe);
        }
        if let Some(na) = na {
            visitor(na);
        }
        cmavo.visit_words(visitor);
        if let Some(nai) = nai {
            nai.visit_words(visitor);
        }
    }
}

impl BeiLinkSyntax {
    #[requires(true)]
    #[ensures(true)]
    pub fn visit_words(&self, visitor: &mut impl FnMut(&Token)) {
        self.bei.visit_words(visitor);
        if let Some(fa) = &self.fa {
            fa.visit_words(visitor);
        }
        if let Some(argument) = &self.argument {
            argument.visit_words(visitor);
        }
    }
}

impl ArgumentTailElementSyntax {
    #[requires(true)]
    #[ensures(true)]
    pub fn visit_words(&self, visitor: &mut impl FnMut(&Token)) {
        match self.as_data() {
            data!(ArgumentTailElementSyntax::Argument(argument)) => argument.visit_words(visitor),
            data!(ArgumentTailElementSyntax::RelativeClauses(relative_clauses)) => {
                for relative_clause in relative_clauses {
                    relative_clause.visit_words(visitor);
                }
            }
            data!(ArgumentTailElementSyntax::Quantifier(quantifier)) => {
                quantifier.visit_words(visitor);
            }
        }
    }
}

impl QuantifierSyntax {
    #[requires(true)]
    #[ensures(true)]
    pub fn visit_words(&self, visitor: &mut impl FnMut(&Token)) {
        match self.as_data() {
            data!(QuantifierSyntax::Number { number, boi }) => {
                number.visit_words(visitor);
                if let Some(boi) = boi {
                    boi.visit_words(visitor);
                }
            }
            data!(QuantifierSyntax::Vei {
                vei,
                math_expression,
                veho,
            }) => {
                vei.visit_words(visitor);
                math_expression.visit_words(visitor);
                if let Some(veho) = veho {
                    veho.visit_words(visitor);
                }
            }
        }
    }
}

impl MathOperatorSyntax {
    #[requires(true)]
    #[ensures(true)]
    pub fn visit_words(&self, visitor: &mut impl FnMut(&Token)) {
        match self.as_data() {
            data!(MathOperatorSyntax::Vuhu(vuhu)) => vuhu.visit_words(visitor),
            data!(MathOperatorSyntax::Maho {
                maho,
                math_expression,
                tehu,
            }) => {
                maho.visit_words(visitor);
                math_expression.visit_words(visitor);
                if let Some(tehu) = tehu {
                    tehu.visit_words(visitor);
                }
            }
            data!(MathOperatorSyntax::Se { se, inner_operator }) => {
                se.visit_words(visitor);
                inner_operator.visit_words(visitor);
            }
            data!(MathOperatorSyntax::Nahe {
                nahe,
                inner_operator,
            }) => {
                nahe.visit_words(visitor);
                inner_operator.visit_words(visitor);
            }
            data!(MathOperatorSyntax::Nahu {
                nahu,
                relation,
                tehu,
            }) => {
                nahu.visit_words(visitor);
                relation.visit_words(visitor);
                if let Some(tehu) = tehu {
                    tehu.visit_words(visitor);
                }
            }
            data!(MathOperatorSyntax::Ke {
                ke,
                inner_operator,
                kehe,
            }) => {
                ke.visit_words(visitor);
                inner_operator.visit_words(visitor);
                if let Some(kehe) = kehe {
                    kehe.visit_words(visitor);
                }
            }
            data!(MathOperatorSyntax::Bo {
                left_operator,
                connective,
                bo,
                right_operator,
            }) => {
                left_operator.visit_words(visitor);
                connective.visit_words(visitor);
                bo.visit_words(visitor);
                right_operator.visit_words(visitor);
            }
            data!(MathOperatorSyntax::Connected {
                left_operator,
                connective,
                right_operator,
            }) => {
                left_operator.visit_words(visitor);
                connective.visit_words(visitor);
                right_operator.visit_words(visitor);
            }
        }
    }
}

impl RelationSyntax {
    #[requires(true)]
    #[ensures(true)]
    pub fn visit_words(&self, visitor: &mut impl FnMut(&Token)) {
        match self.as_data() {
            data!(RelationSyntax::Connected {
                connective,
                leading_relation,
                trailing_relation,
            }) => {
                leading_relation.visit_words(visitor);
                connective.visit_words(visitor);
                trailing_relation.visit_words(visitor);
            }
            data!(RelationSyntax::Co {
                leading_relation,
                co,
                trailing_relation,
            }) => {
                leading_relation.visit_words(visitor);
                co.visit_words(visitor);
                trailing_relation.visit_words(visitor);
            }
            data!(RelationSyntax::Bo {
                leading_relation,
                bo_connective,
                bo_tense_modal,
                bo,
                trailing_relation,
            }) => {
                leading_relation.visit_words(visitor);
                if let Some(connective) = bo_connective {
                    connective.visit_words(visitor);
                }
                if let Some(tense_modal) = bo_tense_modal {
                    tense_modal.visit_words(visitor);
                }
                bo.visit_words(visitor);
                trailing_relation.visit_words(visitor);
            }
            data!(RelationSyntax::Na { na, inner_relation }) => {
                na.visit_words(visitor);
                inner_relation.visit_words(visitor);
            }
            data!(RelationSyntax::Base(word)) => visitor(word),
            data!(RelationSyntax::Se { se, inner_relation }) => {
                se.visit_words(visitor);
                inner_relation.visit_words(visitor);
            }
            data!(RelationSyntax::Ke {
                ke,
                relation,
                kehe,
                ..
            }) => {
                ke.visit_words(visitor);
                relation.visit_words(visitor);
                if let Some(kehe) = kehe {
                    kehe.visit_words(visitor);
                }
            }
            data!(RelationSyntax::TenseModal {
                tense_modal,
                inner_relation,
            }) => {
                tense_modal.visit_words(visitor);
                inner_relation.visit_words(visitor);
            }
            data!(RelationSyntax::Guha {
                guhek,
                leading_predicate,
                gik,
                trailing_predicate,
                gihi,
            }) => {
                guhek.visit_words(visitor);
                leading_predicate.visit_words(visitor);
                gik.visit_words(visitor);
                trailing_predicate.visit_words(visitor);
                if let Some(gihi) = gihi {
                    visitor(gihi);
                }
            }
            data!(RelationSyntax::Abstraction(abstraction)) => abstraction.visit_words(visitor),
            data!(RelationSyntax::Compound(units)) => {
                for unit in units.iter() {
                    unit.visit_words(visitor);
                }
            }
        }
    }

    #[requires(true)]
    #[ensures(true)]
    pub fn word_count(&self) -> usize {
        let mut count = 0;
        self.visit_words(&mut |_| count += 1);
        count
    }
}

impl RelationUnitSyntax {
    #[requires(true)]
    #[ensures(true)]
    pub fn visit_words(&self, visitor: &mut impl FnMut(&Token)) {
        match self.as_data() {
            data!(RelationUnitSyntax::Word(word)) => word.visit_words(visitor),
            data!(RelationUnitSyntax::Goha { goha, raho }) => {
                goha.visit_words(visitor);
                if let Some(raho) = raho {
                    raho.visit_words(visitor);
                }
            }
            data!(RelationUnitSyntax::Se { se, inner_unit }) => {
                se.visit_words(visitor);
                inner_unit.visit_words(visitor);
            }
            data!(RelationUnitSyntax::Ke {
                ke,
                relation,
                kehe,
                ..
            }) => {
                ke.visit_words(visitor);
                relation.visit_words(visitor);
                if let Some(kehe) = kehe {
                    kehe.visit_words(visitor);
                }
            }
            data!(RelationUnitSyntax::Nahe { nahe, inner_unit }) => {
                nahe.visit_words(visitor);
                inner_unit.visit_words(visitor);
            }
            data!(RelationUnitSyntax::Bo {
                leading_unit,
                bo_connective,
                bo_tense_modal,
                bo,
                trailing_unit,
            }) => {
                leading_unit.visit_words(visitor);
                if let Some(connective) = bo_connective {
                    connective.visit_words(visitor);
                }
                if let Some(tense_modal) = bo_tense_modal {
                    tense_modal.visit_words(visitor);
                }
                bo.visit_words(visitor);
                trailing_unit.visit_words(visitor);
            }
            data!(RelationUnitSyntax::Connected {
                leading_unit,
                connective,
                trailing_unit,
            }) => {
                leading_unit.visit_words(visitor);
                connective.visit_words(visitor);
                trailing_unit.visit_words(visitor);
            }
            data!(RelationUnitSyntax::SelbriRelativeClause {
                base,
                selbri_relative_clauses,
            }) => {
                base.visit_words(visitor);
                for selbri_relative_clause in selbri_relative_clauses {
                    selbri_relative_clause.visit_words(visitor);
                }
            }
            data!(RelationUnitSyntax::Wrapped(relation)) => relation.visit_words(visitor),
            data!(RelationUnitSyntax::Jai {
                jai,
                tense_modal,
                inner_unit,
            }) => {
                jai.visit_words(visitor);
                if let Some(tense_modal) = tense_modal {
                    tense_modal.visit_words(visitor);
                }
                inner_unit.visit_words(visitor);
            }
            data!(RelationUnitSyntax::Be {
                base,
                be,
                fa,
                first_argument,
                bei_links,
                beho,
            }) => {
                base.visit_words(visitor);
                be.visit_words(visitor);
                if let Some(fa) = fa {
                    fa.visit_words(visitor);
                }
                if let Some(first_argument) = first_argument {
                    first_argument.visit_words(visitor);
                }
                for bei_link in bei_links {
                    bei_link.visit_words(visitor);
                }
                if let Some(beho) = beho {
                    beho.visit_words(visitor);
                }
            }
            data!(RelationUnitSyntax::PreposedBe {
                be,
                fa,
                first_argument,
                bei_links,
                beho,
                base,
            }) => {
                be.visit_words(visitor);
                if let Some(fa) = fa {
                    fa.visit_words(visitor);
                }
                if let Some(first_argument) = first_argument {
                    first_argument.visit_words(visitor);
                }
                for bei_link in bei_links {
                    bei_link.visit_words(visitor);
                }
                if let Some(beho) = beho {
                    beho.visit_words(visitor);
                }
                base.visit_words(visitor);
            }
            data!(RelationUnitSyntax::Abstraction(abstraction)) => abstraction.visit_words(visitor),
            data!(RelationUnitSyntax::Me {
                me,
                argument,
                mehu,
                moi_marker,
            }) => {
                me.visit_words(visitor);
                argument.visit_words(visitor);
                if let Some(mehu) = mehu {
                    mehu.visit_words(visitor);
                }
                if let Some(moi_marker) = moi_marker {
                    moi_marker.visit_words(visitor);
                }
            }
            data!(RelationUnitSyntax::Mehoi(mehoi)) => mehoi.visit_words(visitor),
            data!(RelationUnitSyntax::Gohoi(gohoi)) => gohoi.visit_words(visitor),
            data!(RelationUnitSyntax::Muhoi(muhoi)) => muhoi.visit_words(visitor),
            data!(RelationUnitSyntax::Luhei { luhei, text, liau }) => {
                luhei.visit_words(visitor);
                text.visit_words(visitor);
                if let Some(liau) = liau {
                    liau.visit_words(visitor);
                }
            }
            data!(RelationUnitSyntax::Moi { number, moi }) => {
                visit_word_slice(number, visitor);
                moi.visit_words(visitor);
            }
            data!(RelationUnitSyntax::Nuha {
                nuha,
                math_operator,
            }) => {
                nuha.visit_words(visitor);
                math_operator.visit_words(visitor);
            }
            data!(RelationUnitSyntax::Xohi { xohi, tag }) => {
                xohi.visit_words(visitor);
                tag.visit_words(visitor);
            }
            data!(RelationUnitSyntax::Cei { base, assignments }) => {
                base.visit_words(visitor);
                for assignment in assignments {
                    assignment.cei.visit_words(visitor);
                    assignment.relation_unit.visit_words(visitor);
                }
            }
        }
    }
}

impl AbstractionSyntax {
    #[requires(true)]
    #[ensures(true)]
    pub fn visit_words(&self, visitor: &mut impl FnMut(&Token)) {
        self.nu.visit_words(visitor);
        if let Some(nai) = &self.nai {
            nai.visit_words(visitor);
        }
        for additional_nu in &self.additional_nu {
            additional_nu.visit_words(visitor);
        }
        self.subsentence.visit_words(visitor);
        if let Some(kei) = &self.kei {
            kei.visit_words(visitor);
        }
    }
}

impl AdditionalNuSyntax {
    #[requires(true)]
    #[ensures(true)]
    pub fn visit_words(&self, visitor: &mut impl FnMut(&Token)) {
        self.connective.visit_words(visitor);
        self.nu.visit_words(visitor);
        if let Some(nai) = &self.nai {
            nai.visit_words(visitor);
        }
    }
}

impl CompositeTenseModalPartSyntax {
    #[requires(true)]
    #[ensures(true)]
    pub fn extend_leaf_words_into(self, out: &mut Vec<Token>) {
        match self.into_data() {
            data!(CompositeTenseModalPartSyntax::Word(word)) => out.push(word),
            data!(CompositeTenseModalPartSyntax::Fiho(fiho)) => {
                let data!(FihoModalSyntax {
                    nahe: _,
                    fiho,
                    relation,
                    fehu,
                }) = fiho.into_data();
                out.push(fiho.value);
                out.extend(relation.words());
                if let Some(fehu) = fehu {
                    out.push(fehu.value);
                }
            }
        }
    }

    #[requires(true)]
    #[ensures(true)]
    pub fn visit_words(&self, visitor: &mut impl FnMut(&Token)) {
        match self.as_data() {
            data!(CompositeTenseModalPartSyntax::Word(word)) => visitor(word),
            data!(CompositeTenseModalPartSyntax::Fiho(fiho)) => {
                fiho.fiho.visit_words(visitor);
                fiho.relation.visit_words(visitor);
                if let Some(fehu) = &fiho.fehu {
                    fehu.visit_words(visitor);
                }
            }
        }
    }

    #[requires(true)]
    #[ensures(true)]
    pub fn words(self) -> Vec<Token> {
        match self.into_data() {
            data!(CompositeTenseModalPartSyntax::Word(word)) => vec![word],
            data!(CompositeTenseModalPartSyntax::Fiho(fiho)) => {
                let data!(FihoModalSyntax {
                    nahe: _,
                    fiho,
                    relation,
                    fehu,
                }) = fiho.into_data();
                let mut words = vec![fiho.value];
                words.extend(relation.words());
                if let Some(fehu) = fehu {
                    words.push(fehu.value);
                }
                words
            }
        }
    }
}

impl TenseModalSyntax {
    #[requires(true)]
    #[ensures(true)]
    pub fn extend_words_into(self, out: &mut Vec<Token>) {
        let (leaves, free_modifiers) = self.leaf_words_and_free_modifiers();
        out.extend(leaves);
        for free_modifier in free_modifiers {
            free_modifier.extend_words_into(out);
        }
    }

    #[requires(true)]
    #[ensures(true)]
    pub fn visit_words(&self, visitor: &mut impl FnMut(&Token)) {
        match self.as_data() {
            data!(TenseModalSyntax::Composite { parts }) => {
                for part in &parts.value {
                    part.visit_words(visitor);
                }
                for free_modifier in &parts.free_modifiers {
                    free_modifier.visit_words(visitor);
                }
            }
            data!(TenseModalSyntax::Pu(word))
            | data!(TenseModalSyntax::TimeInterval(word))
            | data!(TenseModalSyntax::SpaceDistance(word))
            | data!(TenseModalSyntax::SpaceDirection(word))
            | data!(TenseModalSyntax::Caha(word)) => word.visit_words(visitor),
            data!(TenseModalSyntax::PuDistance { pu, distance }) => {
                visitor(pu);
                distance.visit_words(visitor);
            }
            data!(TenseModalSyntax::PuCaha { pu, caha }) => {
                visitor(pu);
                caha.visit_words(visitor);
            }
            data!(TenseModalSyntax::SpaceMovement {
                mohi,
                direction,
                distance,
            }) => {
                visitor(mohi);
                direction.visit_words(visitor);
                if let Some(distance) = distance {
                    distance.visit_words(visitor);
                }
            }
            data!(TenseModalSyntax::Simple {
                nahe,
                se,
                bai,
                nai,
                ki,
            }) => {
                if let Some(nahe) = nahe {
                    nahe.visit_words(visitor);
                }
                if let Some(se) = se {
                    se.visit_words(visitor);
                }
                bai.visit_words(visitor);
                if let Some(nai) = nai {
                    nai.visit_words(visitor);
                }
                if let Some(ki) = ki {
                    ki.visit_words(visitor);
                }
            }
            data!(TenseModalSyntax::Ki(ki)) => ki.visit_words(visitor),
            data!(TenseModalSyntax::Fiho {
                fiho,
                relation,
                fehu,
            }) => {
                fiho.visit_words(visitor);
                relation.visit_words(visitor);
                if let Some(fehu) = fehu {
                    fehu.visit_words(visitor);
                }
            }
            data!(TenseModalSyntax::Zaho(words)) => words.visit_words(visitor),
            data!(TenseModalSyntax::Interval {
                number,
                roi_or_tahe,
                nai,
            }) => {
                if let Some(number) = number {
                    visit_word_slice(number, visitor);
                }
                roi_or_tahe.visit_words(visitor);
                if let Some(nai) = nai {
                    nai.visit_words(visitor);
                }
            }
        }
    }

    #[requires(true)]
    #[ensures(true)]
    pub fn word_count(&self) -> usize {
        let mut count = 0;
        self.visit_words(&mut |_| count += 1);
        count
    }
}

impl SubsentenceSyntax {
    #[requires(true)]
    #[ensures(true)]
    pub fn words(self) -> Vec<Token> {
        match self.into_data() {
            data!(SubsentenceSyntax::Plain(predicate)) => predicate.words(),
            data!(SubsentenceSyntax::Prenex {
                prenex_terms,
                zohu,
                inner_subsentence,
            }) => {
                let mut words = prenex_terms
                    .into_iter()
                    .flat_map(TermSyntax::words)
                    .collect::<Vec<_>>();
                words.extend(zohu.words());
                words.extend(inner_subsentence.words());
                words
            }
        }
    }
}

impl FragmentSyntax {
    #[requires(true)]
    #[ensures(true)]
    pub fn words(self) -> Vec<Token> {
        match self.into_data() {
            data!(FragmentSyntax::Ek(connective)) | data!(FragmentSyntax::Gihek(connective)) => {
                connective.words()
            }
            data!(FragmentSyntax::Other(words)) => words.words(),
            data!(FragmentSyntax::Ijek { i, connective }) => {
                let mut words = vec![i];
                words.extend(connective.words());
                words
            }
            data!(FragmentSyntax::Prenex { terms, zohu }) => {
                let mut words = terms
                    .into_iter()
                    .flat_map(TermSyntax::words)
                    .collect::<Vec<_>>();
                words.extend(zohu.words());
                words
            }
            data!(FragmentSyntax::BeLink {
                be,
                fa,
                first_argument,
                bei_links,
                beho,
            }) => {
                let mut words = be.words();
                if let Some(fa) = fa {
                    words.extend(fa.words());
                }
                if let Some(first_argument) = first_argument {
                    words.extend(first_argument.words());
                }
                words.extend(bei_links.into_iter().flat_map(BeiLinkSyntax::words));
                if let Some(beho) = beho {
                    words.extend(beho.words());
                }
                words
            }
            data!(FragmentSyntax::BeiLink(bei_only_links)) => bei_only_links
                .into_iter()
                .flat_map(BeiLinkSyntax::words)
                .collect(),
            data!(FragmentSyntax::RelativeClause(relative_clauses)) => relative_clauses
                .into_iter()
                .flat_map(RelativeClauseSyntax::words)
                .collect(),
            data!(FragmentSyntax::MathExpression(math_expression)) => math_expression.words(),
            data!(FragmentSyntax::Term { terms, vau }) => {
                let mut words = Vec::new();
                for term in terms {
                    words.extend(term.words());
                }
                if let Some(vau) = vau {
                    words.extend(vau.words());
                }
                words
            }
            data!(FragmentSyntax::Relation(relation)) => relation.words(),
        }
    }
}

impl TermSyntax {
    #[requires(true)]
    #[ensures(true)]
    pub fn words(self) -> Vec<Token> {
        match self.into_data() {
            data!(TermSyntax::NuhiTermset {
                nuhi,
                termset,
                nuhu,
            }) => {
                let mut words = nuhi.words();
                for term in termset {
                    words.extend(term.words());
                }
                if let Some(nuhu) = nuhu {
                    words.extend(nuhu.words());
                }
                words
            }
            data!(TermSyntax::GekNuhiTermset {
                m_nuhi,
                gek,
                terms,
                nuhu,
                gik,
                gik_terms,
                gihi,
                gik_nuhu,
            }) => {
                let mut words = Vec::new();
                if let Some(nuhi) = m_nuhi {
                    words.extend(nuhi.words());
                }
                words.extend(gek.words());
                for term in terms {
                    words.extend(term.words());
                }
                if let Some(nuhu) = nuhu {
                    words.extend(nuhu.words());
                }
                words.extend(gik.words());
                for term in gik_terms {
                    words.extend(term.words());
                }
                if let Some(gihi) = gihi {
                    words.push(gihi);
                }
                if let Some(nuhu) = gik_nuhu {
                    words.extend(nuhu.words());
                }
                words
            }
            data!(TermSyntax::Cehe {
                leading_terms,
                cehe,
                trailing_terms,
            }) => {
                let mut words = Vec::new();
                for term in leading_terms {
                    words.extend(term.words());
                }
                words.extend(cehe.words());
                for term in trailing_terms {
                    words.extend(term.words());
                }
                words
            }
            data!(TermSyntax::Pehe {
                leading_terms,
                pehe,
                connective,
                trailing_terms,
            }) => {
                let mut words = Vec::new();
                for term in leading_terms {
                    words.extend(term.words());
                }
                words.extend(pehe.words());
                words.extend(connective.words());
                for term in trailing_terms {
                    words.extend(term.words());
                }
                words
            }
            data!(TermSyntax::Argument(argument)) => argument.words(),
            data!(TermSyntax::Fa { fa, argument, ku }) => {
                let mut words = fa.words();
                words.extend(argument.words());
                if let Some(ku) = ku {
                    words.extend(ku.words());
                }
                words
            }
            data!(TermSyntax::NaKu { na, na_ku }) => {
                let mut words = vec![na];
                words.extend(na_ku.words());
                words
            }
            data!(TermSyntax::BareNa(na)) => na.words(),
            data!(TermSyntax::NoihaAdverbial {
                noiha,
                tail_elements,
                relation,
                relative_clauses,
                fehu,
            }) => {
                let mut words = noiha.words();
                for tail_element in tail_elements {
                    words.extend(tail_element.words());
                }
                if let Some(relation) = relation {
                    words.extend(relation.words());
                }
                for relative_clause in relative_clauses {
                    words.extend(relative_clause.words());
                }
                if let Some(fehu) = fehu {
                    words.extend(fehu.words());
                }
                words
            }
            data!(TermSyntax::PoihaBrigahi {
                poiha,
                tail_elements,
                relation,
                relative_clauses,
                brigahi_ku,
            }) => {
                let mut words = poiha.words();
                for tail_element in tail_elements {
                    words.extend(tail_element.words());
                }
                if let Some(relation) = relation {
                    words.extend(relation.words());
                }
                for relative_clause in relative_clauses {
                    words.extend(relative_clause.words());
                }
                words.extend(brigahi_ku.words());
                words
            }
            data!(TermSyntax::FihoiAdverbial {
                fihoi,
                subsentence,
                fihau,
            }) => {
                let mut words = fihoi.words();
                words.extend((*subsentence).words());
                if let Some(fihau) = fihau {
                    words.extend(fihau.words());
                }
                words
            }
            data!(TermSyntax::SoiAdverbial {
                soi,
                subsentence,
                sehu,
            }) => {
                let mut words = soi.words();
                words.extend((*subsentence).words());
                if let Some(sehu) = sehu {
                    words.extend(sehu.words());
                }
                words
            }
            data!(TermSyntax::JaiTagged { jai, tag, argument }) => {
                let mut words = jai.words();
                if let Some(tag) = tag {
                    words.extend(tag.words());
                }
                words.extend(argument.words());
                words
            }
            data!(TermSyntax::Tagged {
                tense_modal,
                argument,
            }) => {
                let mut words = tense_modal
                    .into_iter()
                    .flat_map(|tense_modal| tense_modal.words())
                    .collect::<Vec<_>>();
                words.extend(argument.words());
                words
            }
            data!(TermSyntax::Connected {
                leading_terms,
                connective,
                trailing_terms,
            }) => {
                let mut words = Vec::new();
                for term in leading_terms {
                    words.extend(term.words());
                }
                words.extend(connective.words());
                for term in trailing_terms {
                    words.extend(term.words());
                }
                words
            }
            data!(TermSyntax::BoConnected {
                leading_terms,
                bo_connective,
                tense_modal,
                bo,
                trailing_term,
            }) => {
                let mut words = Vec::new();
                for term in leading_terms {
                    words.extend(term.words());
                }
                if let Some(connective) = bo_connective {
                    words.extend(connective.words());
                }
                if let Some(tense_modal) = tense_modal {
                    words.extend(tense_modal.words());
                }
                words.extend(bo.words());
                words.extend(trailing_term.words());
                words
            }
        }
    }
}

impl ArgumentTagSyntax {
    #[requires(true)]
    #[ensures(true)]
    pub fn words(self) -> Vec<Token> {
        match self.into_data() {
            data!(ArgumentTagSyntax::TenseModal(tense_modal)) => tense_modal.words(),
            data!(ArgumentTagSyntax::Fa(fa)) => fa.words(),
        }
    }
}

impl MathExpressionSyntax {
    #[requires(true)]
    #[ensures(true)]
    pub fn words(self) -> Vec<Token> {
        match self.into_data() {
            data!(MathExpressionSyntax::Number(quantifier)) => quantifier.words(),
            data!(MathExpressionSyntax::Letter { letter, boi }) => {
                let mut words = letter.words();
                if let Some(boi) = boi {
                    words.extend(boi.words());
                }
                words
            }
            data!(MathExpressionSyntax::Vei {
                vei,
                inner_expression,
                veho,
            }) => {
                let mut words = vei.words();
                words.extend(inner_expression.words());
                if let Some(veho) = veho {
                    words.extend(veho.words());
                }
                words
            }
            data!(MathExpressionSyntax::Gek {
                gek,
                left_expression,
                gik,
                right_expression,
            }) => {
                let mut words = gek.words();
                words.extend(left_expression.words());
                words.extend(gik.words());
                words.extend(right_expression.words());
                words
            }
            data!(MathExpressionSyntax::Forethought {
                peho,
                operator,
                operands,
                kuhe,
            }) => {
                let mut words = Vec::new();
                if let Some(peho) = peho {
                    words.extend(peho.words());
                }
                words.extend(operator.words());
                for operand in operands {
                    words.extend(operand.words());
                }
                if let Some(kuhe) = kuhe {
                    words.extend(kuhe.words());
                }
                words
            }
            data!(MathExpressionSyntax::ReversePolish {
                fuha,
                operands,
                operators,
            }) => {
                let mut words = fuha.words();
                for operand in operands {
                    words.extend(operand.words());
                }
                for operator in operators {
                    words.extend(operator.words());
                }
                words
            }
            data!(MathExpressionSyntax::Nihe {
                nihe,
                relation,
                tehu,
            }) => {
                let mut words = nihe.words();
                words.extend(relation.words());
                if let Some(tehu) = tehu {
                    words.extend(tehu.words());
                }
                words
            }
            data!(MathExpressionSyntax::Mohe {
                mohe,
                argument,
                tehu,
            }) => {
                let mut words = mohe.words();
                words.extend(argument.words());
                if let Some(tehu) = tehu {
                    words.extend(tehu.words());
                }
                words
            }
            data!(MathExpressionSyntax::Johi {
                johi,
                expressions,
                tehu,
            }) => {
                let mut words = johi.words();
                for expression in expressions {
                    words.extend(expression.words());
                }
                if let Some(tehu) = tehu {
                    words.extend(tehu.words());
                }
                words
            }
            data!(MathExpressionSyntax::Lahe {
                markers,
                inner_expression,
                luhu,
            }) => {
                let mut words = markers.words();
                words.extend(inner_expression.words());
                if let Some(luhu) = luhu {
                    words.extend(luhu.words());
                }
                words
            }
            data!(MathExpressionSyntax::Connected {
                left_expression,
                connective,
                right_expression,
            }) => {
                let mut words = left_expression.words();
                words.extend(connective.words());
                words.extend(right_expression.words());
                words
            }
            data!(MathExpressionSyntax::Binary {
                operator,
                left_expression,
                right_expression,
            }) => {
                let mut words = left_expression.words();
                words.extend(operator.words());
                words.extend(right_expression.words());
                words
            }
            data!(MathExpressionSyntax::Bihe {
                left_expression,
                bihe,
                operator,
                right_expression,
            }) => {
                let mut words = left_expression.words();
                words.extend(bihe.words());
                words.extend(operator.words());
                words.extend(right_expression.words());
                words
            }
        }
    }
}

impl ArgumentSyntax {
    #[requires(true)]
    #[ensures(true)]
    pub fn words(self) -> Vec<Token> {
        match self.into_data() {
            data!(ArgumentSyntax::Quote(quote)) => quote.words(),
            data!(ArgumentSyntax::MathExpression {
                li,
                expression,
                loho,
            }) => {
                let mut words = li.words();
                words.extend(expression.words());
                if let Some(loho) = loho {
                    words.extend(loho.words());
                }
                words
            }
            data!(ArgumentSyntax::Letter { letter, boi }) => {
                let mut words = letter.words();
                if let Some(boi) = boi {
                    words.extend(boi.words());
                }
                words
            }
            data!(ArgumentSyntax::Quantified {
                quantifier,
                inner_argument,
            }) => {
                let mut words = quantifier.words();
                words.extend(inner_argument.words());
                words
            }
            data!(ArgumentSyntax::RelativeClause {
                base_argument,
                vuho,
                relative_clauses,
            }) => {
                let mut words = base_argument.words();
                if let Some(vuho) = vuho {
                    words.extend(vuho.words());
                }
                for relative_clause in relative_clauses {
                    words.extend(relative_clause.words());
                }
                words
            }
            data!(ArgumentSyntax::Vuho {
                base_argument,
                vuho_marker,
                relative_clauses,
                connected_argument,
            }) => {
                let mut words = base_argument.words();
                words.extend(vuho_marker.words());
                for relative_clause in relative_clauses {
                    words.extend(relative_clause.words());
                }
                if let Some(connected_argument) = connected_argument {
                    words.extend(connected_argument.connective.words());
                    words.extend(connected_argument.argument.words());
                }
                words
            }
            data!(ArgumentSyntax::BridiDescription {
                lohoi,
                subsentence,
                kuhau,
            }) => {
                let mut words = lohoi.words();
                words.extend(subsentence.words());
                if let Some(kuhau) = kuhau {
                    words.extend(kuhau.words());
                }
                words
            }
            data!(ArgumentSyntax::NaKu { na, ku }) => {
                let mut words = vec![na];
                words.extend(ku.words());
                words
            }
            data!(ArgumentSyntax::Tagged {
                tag,
                inner_argument,
            }) => {
                let mut words = tag.words();
                words.extend(inner_argument.words());
                words
            }
            data!(ArgumentSyntax::NaheBo {
                nahe,
                bo,
                inner_argument,
                luhu,
            }) => {
                let mut words = vec![nahe];
                words.extend(bo.words());
                words.extend(inner_argument.words());
                if let Some(luhu) = luhu {
                    words.extend(luhu.words());
                }
                words
            }
            data!(ArgumentSyntax::Nahe {
                nahe,
                inner_argument,
                luhu,
            }) => {
                let mut words = nahe.words();
                words.extend(inner_argument.words());
                if let Some(luhu) = luhu {
                    words.extend(luhu.words());
                }
                words
            }
            data!(ArgumentSyntax::TermWrapped {
                wrapper,
                wrapper_bo,
                inner_term,
                luhu,
                ..
            }) => {
                let mut words = wrapper.words();
                if let Some(wrapper_bo) = wrapper_bo {
                    words.extend(wrapper_bo.words());
                }
                words.extend(inner_term.words());
                if let Some(luhu) = luhu {
                    words.extend(luhu.words());
                }
                words
            }
            data!(ArgumentSyntax::Koha(koha)) => koha.words(),
            data!(ArgumentSyntax::Zohe {
                tag,
                maybe_ku,
                free_modifiers,
            }) => {
                let mut words = tag
                    .into_iter()
                    .flat_map(|tag| tag.words())
                    .collect::<Vec<_>>();
                if let Some(ku) = maybe_ku {
                    words.extend(ku.words());
                }
                for free_modifier in free_modifiers {
                    words.extend(free_modifier.words());
                }
                words
            }
            data!(ArgumentSyntax::Lahe {
                lahe,
                relative_clauses,
                inner_argument,
                luhu,
            }) => {
                let mut words = lahe.words();
                for relative_clause in relative_clauses {
                    words.extend(relative_clause.words());
                }
                words.extend(inner_argument.words());
                if let Some(luhu) = luhu {
                    words.extend(luhu.words());
                }
                words
            }
            data!(ArgumentSyntax::Connected {
                leading_argument,
                connective,
                trailing_argument,
            }) => {
                let mut words = leading_argument.words();
                words.extend(connective.words());
                words.extend(trailing_argument.words());
                words
            }
            data!(ArgumentSyntax::Ke {
                ke,
                inner_argument,
                kehe,
            }) => {
                let mut words = ke.words();
                words.extend(inner_argument.words());
                if let Some(kehe) = kehe {
                    words.extend(kehe.words());
                }
                words
            }
            data!(ArgumentSyntax::Bo {
                leading_argument,
                bo_connective,
                bo_tense_modal,
                bo,
                trailing_argument,
            }) => {
                let mut words = leading_argument.words();
                if let Some(connective) = bo_connective {
                    words.extend(connective.words());
                }
                if let Some(tense_modal) = bo_tense_modal {
                    words.extend(tense_modal.words());
                }
                words.extend(bo.words());
                words.extend(trailing_argument.words());
                words
            }
            data!(ArgumentSyntax::Gek {
                gek,
                leading_argument,
                gik,
                trailing_argument,
                gihi,
            }) => {
                let mut words = gek.words();
                words.extend(leading_argument.words());
                words.extend(gik.words());
                words.extend(trailing_argument.words());
                if let Some(gihi) = gihi {
                    words.push(gihi);
                }
                words
            }
            data!(ArgumentSyntax::Descriptor(descriptor)) => descriptor.words(),
            data!(ArgumentSyntax::ConnectedDescriptor(connected_descriptor)) => {
                connected_descriptor.words()
            }
            data!(ArgumentSyntax::Name { la, names }) => {
                let mut words = la.words();
                words.extend(names.words());
                words
            }
            data!(ArgumentSyntax::Cmevla(cmevla)) => cmevla.words(),
            data!(ArgumentSyntax::RelationVocative {
                leading_relative_clauses,
                relation,
                trailing_relative_clauses,
            }) => {
                let mut words = leading_relative_clauses
                    .into_iter()
                    .flat_map(RelativeClauseSyntax::words)
                    .collect::<Vec<_>>();
                words.extend(relation.words());
                words.extend(
                    trailing_relative_clauses
                        .into_iter()
                        .flat_map(RelativeClauseSyntax::words),
                );
                words
            }
        }
    }
}

impl GoiRelativeClauseSyntax {
    #[requires(true)]
    #[ensures(true)]
    pub fn words(self) -> Vec<Token> {
        let data!(GoiRelativeClauseSyntax {
            goi,
            argument,
            gehu,
        }) = self.into_data();
        let mut words = goi.words();
        words.extend(argument.words());
        if let Some(gehu) = gehu {
            words.extend(gehu.words());
        }
        words
    }
}

impl SelbriRelativeClauseSyntax {
    #[requires(true)]
    #[ensures(true)]
    pub fn words(self) -> Vec<Token> {
        let data!(SelbriRelativeClauseSyntax {
            nohoi,
            relation,
            kuhoi,
        }) = self.into_data();
        let mut words = nohoi.words();
        words.extend(relation.words());
        if let Some(kuhoi) = kuhoi {
            words.extend(kuhoi.words());
        }
        words
    }
}

impl RelativeClauseSyntax {
    #[requires(true)]
    #[ensures(true)]
    pub fn words(self) -> Vec<Token> {
        match self.into_data() {
            data!(RelativeClauseSyntax::Goi(relative_clause)) => relative_clause.words(),
            data!(RelativeClauseSyntax::Noi {
                noi,
                subsentence,
                kuho,
            }) => {
                let mut words = noi.words();
                words.extend(subsentence.words());
                if let Some(kuho) = kuho {
                    words.extend(kuho.words());
                }
                words
            }
            data!(RelativeClauseSyntax::Poi {
                poi,
                subsentence,
                kuho,
            }) => {
                let mut words = poi.words();
                words.extend(subsentence.words());
                if let Some(kuho) = kuho {
                    words.extend(kuho.words());
                }
                words
            }
            data!(RelativeClauseSyntax::Zihe { zihe, inner }) => {
                let mut words = zihe.words();
                words.extend(inner.words());
                words
            }
            data!(RelativeClauseSyntax::Connected { connective, inner }) => {
                let mut words = connective.words();
                words.extend(inner.words());
                words
            }
        }
    }
}

impl QuoteSyntax {
    #[requires(true)]
    #[ensures(true)]
    pub fn words(self) -> Vec<Token> {
        match self.into_data() {
            data!(QuoteSyntax::Lu { lu, text, lihu }) => {
                let mut words = lu.words();
                words.extend(text.words());
                if let Some(lihu) = lihu {
                    words.extend(lihu.words());
                }
                words
            }
            data!(QuoteSyntax::Zo(zo)) | data!(QuoteSyntax::Zoi(zo)) => zo.words(),
            data!(QuoteSyntax::ZohOi(zohoi)) => zohoi.words(),
            data!(QuoteSyntax::Lohu(lohu)) => lohu.words(),
        }
    }
}

impl DescriptorSyntax {
    #[requires(true)]
    #[ensures(true)]
    pub fn words(self) -> Vec<Token> {
        let data!(DescriptorSyntax {
            outer_quantifier,
            descriptor,
            tail_elements,
            relation,
            relative_clauses,
            ku,
        }) = self.into_data();
        let mut words = outer_quantifier
            .into_iter()
            .flat_map(|quantifier| quantifier.words())
            .collect::<Vec<_>>();
        if let Some(descriptor) = descriptor {
            words.extend(descriptor.words());
        }
        for element in tail_elements {
            words.extend(element.words());
        }
        if let Some(relation) = relation {
            words.extend(relation.words());
        }
        for relative_clause in relative_clauses {
            words.extend(relative_clause.words());
        }
        if let Some(ku) = ku {
            words.extend(ku.words());
        }
        words
    }
}

impl DescriptorHeadSyntax {
    #[requires(true)]
    #[ensures(true)]
    pub fn words(self) -> Vec<Token> {
        let data!(DescriptorHeadSyntax { descriptor }) = self.into_data();
        descriptor.words()
    }
}

impl ConnectedDescriptorSyntax {
    #[requires(true)]
    #[ensures(true)]
    pub fn words(self) -> Vec<Token> {
        let data!(ConnectedDescriptorSyntax {
            leading_descriptor_head,
            connective,
            trailing_descriptor_head,
            tail_elements,
            relation,
            relative_clauses,
            ku,
        }) = self.into_data();
        let mut words = leading_descriptor_head.words();
        words.extend(connective.words());
        words.extend(trailing_descriptor_head.words());
        for element in tail_elements {
            words.extend(element.words());
        }
        if let Some(relation) = relation {
            words.extend(relation.words());
        }
        for relative_clause in relative_clauses {
            words.extend(relative_clause.words());
        }
        if let Some(ku) = ku {
            words.extend(ku.words());
        }
        words
    }
}

impl ConnectiveSyntax {
    #[requires(true)]
    #[ensures(true)]
    pub fn words(self) -> Vec<Token> {
        let ConnectiveSyntaxParts {
            kind: _,
            se,
            nahe,
            na,
            cmavo,
            nai,
        } = self.into_parts();
        let mut words = Vec::new();
        if let Some(se) = se {
            words.push(se);
        }
        if let Some(nahe) = nahe {
            words.push(nahe);
        }
        if let Some(na) = na {
            words.push(na);
        }
        cmavo.extend_words_into(&mut words);
        if let Some(nai) = nai {
            nai.extend_words_into(&mut words);
        }
        words
    }
}

impl BeiLinkSyntax {
    #[requires(true)]
    #[ensures(true)]
    pub fn words(self) -> Vec<Token> {
        let data!(BeiLinkSyntax { bei, fa, argument }) = self.into_data();
        let mut words = bei.words();
        if let Some(fa) = fa {
            words.extend(fa.words());
        }
        if let Some(argument) = argument {
            words.extend(argument.words());
        }
        words
    }
}

impl ArgumentTailElementSyntax {
    #[requires(true)]
    #[ensures(true)]
    pub fn words(self) -> Vec<Token> {
        match self.into_data() {
            data!(ArgumentTailElementSyntax::Argument(argument)) => argument.words(),
            data!(ArgumentTailElementSyntax::RelativeClauses(relative_clauses)) => relative_clauses
                .into_iter()
                .flat_map(RelativeClauseSyntax::words)
                .collect(),
            data!(ArgumentTailElementSyntax::Quantifier(quantifier)) => quantifier.words(),
        }
    }
}

impl QuantifierSyntax {
    #[requires(true)]
    #[ensures(true)]
    pub fn words(self) -> Vec<Token> {
        match self.into_data() {
            data!(QuantifierSyntax::Number { number, boi }) => {
                let mut words = number.words();
                if let Some(boi) = boi {
                    words.extend(boi.words());
                }
                words
            }
            data!(QuantifierSyntax::Vei {
                vei,
                math_expression,
                veho,
            }) => {
                let mut words = vei.words();
                words.extend(math_expression.words());
                if let Some(veho) = veho {
                    words.extend(veho.words());
                }
                words
            }
        }
    }
}

impl MathOperatorSyntax {
    #[requires(true)]
    #[ensures(true)]
    pub fn words(self) -> Vec<Token> {
        match self.into_data() {
            data!(MathOperatorSyntax::Vuhu(vuhu)) => vuhu.words(),
            data!(MathOperatorSyntax::Maho {
                maho,
                math_expression,
                tehu,
            }) => {
                let mut words = maho.words();
                words.extend(math_expression.words());
                if let Some(tehu) = tehu {
                    words.extend(tehu.words());
                }
                words
            }
            data!(MathOperatorSyntax::Se { se, inner_operator }) => {
                let mut words = se.words();
                words.extend(inner_operator.words());
                words
            }
            data!(MathOperatorSyntax::Nahe {
                nahe,
                inner_operator,
            }) => {
                let mut words = nahe.words();
                words.extend(inner_operator.words());
                words
            }
            data!(MathOperatorSyntax::Nahu {
                nahu,
                relation,
                tehu,
            }) => {
                let mut words = nahu.words();
                words.extend(relation.words());
                if let Some(tehu) = tehu {
                    words.extend(tehu.words());
                }
                words
            }
            data!(MathOperatorSyntax::Ke {
                ke,
                inner_operator,
                kehe,
            }) => {
                let mut words = ke.words();
                words.extend(inner_operator.words());
                if let Some(kehe) = kehe {
                    words.extend(kehe.words());
                }
                words
            }
            data!(MathOperatorSyntax::Bo {
                left_operator,
                connective,
                bo,
                right_operator,
            }) => {
                let mut words = left_operator.words();
                words.extend(connective.words());
                words.extend(bo.words());
                words.extend(right_operator.words());
                words
            }
            data!(MathOperatorSyntax::Connected {
                left_operator,
                connective,
                right_operator,
            }) => {
                let mut words = left_operator.words();
                words.extend(connective.words());
                words.extend(right_operator.words());
                words
            }
        }
    }
}

impl RelationSyntax {
    #[requires(true)]
    #[ensures(true)]
    pub fn words(self) -> Vec<Token> {
        match self.into_data() {
            data!(RelationSyntax::Connected {
                connective,
                leading_relation,
                trailing_relation,
            }) => {
                let mut words = leading_relation.words();
                words.extend(connective.words());
                words.extend(trailing_relation.words());
                words
            }
            data!(RelationSyntax::Co {
                leading_relation,
                co,
                trailing_relation,
            }) => {
                let mut words = leading_relation.words();
                words.extend(co.words());
                words.extend(trailing_relation.words());
                words
            }
            data!(RelationSyntax::Bo {
                leading_relation,
                bo_connective,
                bo_tense_modal,
                bo,
                trailing_relation,
            }) => {
                let mut words = leading_relation.words();
                if let Some(connective) = bo_connective {
                    words.extend(connective.words());
                }
                if let Some(tense_modal) = bo_tense_modal {
                    words.extend(tense_modal.words());
                }
                words.extend(bo.words());
                words.extend(trailing_relation.words());
                words
            }
            data!(RelationSyntax::Na { na, inner_relation }) => {
                let mut words = na.words();
                words.extend(inner_relation.words());
                words
            }
            data!(RelationSyntax::Base(word)) => vec![word],
            data!(RelationSyntax::Se { se, inner_relation }) => {
                let mut words = se.words();
                words.extend(inner_relation.words());
                words
            }
            data!(RelationSyntax::Ke {
                ke,
                relation,
                kehe,
                ..
            }) => {
                let mut words = ke.words();
                words.extend(relation.words());
                if let Some(kehe) = kehe {
                    words.extend(kehe.words());
                }
                words
            }
            data!(RelationSyntax::TenseModal {
                tense_modal,
                inner_relation,
            }) => {
                let mut words = tense_modal.words();
                words.extend(inner_relation.words());
                words
            }
            data!(RelationSyntax::Guha {
                guhek,
                leading_predicate,
                gik,
                trailing_predicate,
                gihi,
            }) => {
                let mut words = guhek.words();
                words.extend(leading_predicate.words());
                words.extend(gik.words());
                words.extend(trailing_predicate.words());
                if let Some(gihi) = gihi {
                    words.push(gihi);
                }
                words
            }
            data!(RelationSyntax::Abstraction(abstraction)) => abstraction.words(),
            data!(RelationSyntax::Compound(units)) => (*units)
                .into_vec()
                .into_iter()
                .flat_map(RelationUnitSyntax::words)
                .collect(),
        }
    }
}

impl RelationUnitSyntax {
    #[requires(true)]
    #[ensures(true)]
    pub fn words(self) -> Vec<Token> {
        match self.into_data() {
            data!(RelationUnitSyntax::Word(word)) => word.words(),
            data!(RelationUnitSyntax::Goha { goha, raho }) => {
                let mut words = goha.words();
                if let Some(raho) = raho {
                    words.extend(raho.words());
                }
                words
            }
            data!(RelationUnitSyntax::Se { se, inner_unit }) => {
                let mut words = se.words();
                words.extend(inner_unit.words());
                words
            }
            data!(RelationUnitSyntax::Ke {
                ke,
                relation,
                kehe,
                ..
            }) => {
                let mut words = ke.words();
                words.extend(relation.words());
                if let Some(kehe) = kehe {
                    words.extend(kehe.words());
                }
                words
            }
            data!(RelationUnitSyntax::Nahe { nahe, inner_unit }) => {
                let mut words = nahe.words();
                words.extend(inner_unit.words());
                words
            }
            data!(RelationUnitSyntax::Bo {
                leading_unit,
                bo_connective,
                bo_tense_modal,
                bo,
                trailing_unit,
            }) => {
                let mut words = leading_unit.words();
                if let Some(connective) = bo_connective {
                    words.extend(connective.words());
                }
                if let Some(tense_modal) = bo_tense_modal {
                    words.extend(tense_modal.words());
                }
                words.extend(bo.words());
                words.extend(trailing_unit.words());
                words
            }
            data!(RelationUnitSyntax::Connected {
                leading_unit,
                connective,
                trailing_unit,
            }) => {
                let mut words = leading_unit.words();
                words.extend(connective.words());
                words.extend(trailing_unit.words());
                words
            }
            data!(RelationUnitSyntax::SelbriRelativeClause {
                base,
                selbri_relative_clauses,
            }) => {
                let mut words = base.words();
                for selbri_relative_clause in selbri_relative_clauses {
                    words.extend(selbri_relative_clause.words());
                }
                words
            }
            data!(RelationUnitSyntax::Wrapped(relation)) => relation.words(),
            data!(RelationUnitSyntax::Jai {
                jai,
                tense_modal,
                inner_unit,
            }) => {
                let mut words = jai.words();
                if let Some(tense_modal) = tense_modal {
                    words.extend(tense_modal.words());
                }
                words.extend(inner_unit.words());
                words
            }
            data!(RelationUnitSyntax::Be {
                base,
                be,
                fa,
                first_argument,
                bei_links,
                beho,
            }) => {
                let mut words = base.words();
                words.extend(be.words());
                if let Some(fa) = fa {
                    words.extend(fa.words());
                }
                if let Some(first_argument) = first_argument {
                    words.extend(first_argument.words());
                }
                words.extend(bei_links.into_iter().flat_map(BeiLinkSyntax::words));
                if let Some(beho) = beho {
                    words.extend(beho.words());
                }
                words
            }
            data!(RelationUnitSyntax::PreposedBe {
                be,
                fa,
                first_argument,
                bei_links,
                beho,
                base,
            }) => {
                let mut words = be.words();
                if let Some(fa) = fa {
                    words.extend(fa.words());
                }
                if let Some(first_argument) = first_argument {
                    words.extend(first_argument.words());
                }
                words.extend(bei_links.into_iter().flat_map(BeiLinkSyntax::words));
                if let Some(beho) = beho {
                    words.extend(beho.words());
                }
                words.extend(base.words());
                words
            }
            data!(RelationUnitSyntax::Abstraction(abstraction)) => abstraction.words(),
            data!(RelationUnitSyntax::Me {
                me,
                argument,
                mehu,
                moi_marker,
            }) => {
                let mut words = me.words();
                words.extend(argument.words());
                if let Some(mehu) = mehu {
                    words.extend(mehu.words());
                }
                if let Some(moi_marker) = moi_marker {
                    words.extend(moi_marker.words());
                }
                words
            }
            data!(RelationUnitSyntax::Mehoi(mehoi)) => mehoi.words(),
            data!(RelationUnitSyntax::Gohoi(gohoi)) => gohoi.words(),
            data!(RelationUnitSyntax::Muhoi(muhoi)) => muhoi.words(),
            data!(RelationUnitSyntax::Luhei { luhei, text, liau }) => {
                let mut words = luhei.words();
                words.extend(text.words());
                if let Some(liau) = liau {
                    words.extend(liau.words());
                }
                words
            }
            data!(RelationUnitSyntax::Moi { number, moi }) => {
                let mut words = number.into_vec();
                words.extend(moi.words());
                words
            }
            data!(RelationUnitSyntax::Nuha {
                nuha,
                math_operator,
            }) => {
                let mut words = nuha.words();
                words.extend(math_operator.words());
                words
            }
            data!(RelationUnitSyntax::Xohi { xohi, tag }) => {
                let mut words = xohi.words();
                words.extend(tag.words());
                words
            }
            data!(RelationUnitSyntax::Cei { base, assignments }) => {
                let mut words = base.words();
                for assignment in assignments {
                    let data!(CeiAssignmentSyntax { cei, relation_unit }) = assignment.into_data();
                    words.extend(cei.words());
                    words.extend(relation_unit.words());
                }
                words
            }
        }
    }
}

impl AbstractionSyntax {
    #[requires(true)]
    #[ensures(true)]
    pub fn words(self) -> Vec<Token> {
        let data!(AbstractionSyntax {
            nu,
            nai,
            additional_nu,
            subsentence,
            kei,
        }) = self.into_data();
        let mut words = nu.words();
        if let Some(nai) = nai {
            words.extend(nai.words());
        }
        for additional_nu in additional_nu {
            words.extend(additional_nu.words());
        }
        words.extend((*subsentence).words());
        if let Some(kei) = kei {
            words.extend(kei.words());
        }
        words
    }
}

impl AdditionalNuSyntax {
    #[requires(true)]
    #[ensures(true)]
    pub fn words(self) -> Vec<Token> {
        let data!(AdditionalNuSyntax {
            connective,
            nu,
            nai,
        }) = self.into_data();
        let mut words = connective.words();
        words.extend(nu.words());
        if let Some(nai) = nai {
            words.extend(nai.words());
        }
        words
    }
}

impl TenseModalSyntax {
    #[requires(true)]
    #[ensures(true)]
    pub fn free_modifier_count(&self) -> usize {
        match self.as_data() {
            data!(TenseModalSyntax::Composite { parts }) => parts.free_modifiers.len(),
            data!(TenseModalSyntax::Pu(word))
            | data!(TenseModalSyntax::TimeInterval(word))
            | data!(TenseModalSyntax::SpaceDistance(word))
            | data!(TenseModalSyntax::SpaceDirection(word))
            | data!(TenseModalSyntax::Caha(word)) => word.free_modifiers.len(),
            data!(TenseModalSyntax::PuDistance { distance, .. }) => distance.free_modifiers.len(),
            data!(TenseModalSyntax::PuCaha { caha, .. }) => caha.free_modifiers.len(),
            data!(TenseModalSyntax::SpaceMovement {
                direction,
                distance,
                ..
            }) => distance
                .as_ref()
                .map_or(direction.free_modifiers.len(), |distance| {
                    distance.free_modifiers.len()
                }),
            data!(TenseModalSyntax::Simple {
                nahe,
                se,
                bai,
                nai,
                ki,
            }) => {
                if let Some(ki) = ki {
                    ki.free_modifiers.len()
                } else if let Some(nai) = nai {
                    nai.free_modifiers.len()
                } else if !bai.free_modifiers.is_empty() {
                    bai.free_modifiers.len()
                } else if let Some(se) = se {
                    se.free_modifiers.len()
                } else if let Some(nahe) = nahe {
                    nahe.free_modifiers.len()
                } else {
                    bai.free_modifiers.len()
                }
            }
            data!(TenseModalSyntax::Ki(ki)) => ki.free_modifiers.len(),
            data!(TenseModalSyntax::Fiho { fiho, fehu, .. }) => fehu
                .as_ref()
                .map_or(fiho.free_modifiers.len(), |fehu| fehu.free_modifiers.len()),
            data!(TenseModalSyntax::Zaho(words)) => words.free_modifiers.len(),
            data!(TenseModalSyntax::Interval {
                roi_or_tahe,
                nai,
                ..
            }) => nai
                .as_ref()
                .map_or(roi_or_tahe.free_modifiers.len(), |nai| {
                    nai.free_modifiers.len()
                }),
        }
    }

    #[requires(true)]
    #[ensures(ret.1.len() == old(self.free_modifier_count()))]
    pub fn leaf_words_and_free_modifiers(self) -> (Vec<Token>, Vec<FreeModifierSyntax>) {
        match self.into_data() {
            data!(TenseModalSyntax::Composite { parts }) => {
                let mut words = Vec::new();
                for part in parts.value {
                    part.extend_leaf_words_into(&mut words);
                }
                (words, parts.free_modifiers)
            }
            data!(TenseModalSyntax::Pu(word)) | data!(TenseModalSyntax::Caha(word)) => {
                (vec![word.value], word.free_modifiers)
            }
            data!(TenseModalSyntax::PuDistance { pu, distance }) => {
                (vec![pu, distance.value], distance.free_modifiers)
            }
            data!(TenseModalSyntax::TimeInterval(word)) => (vec![word.value], word.free_modifiers),
            data!(TenseModalSyntax::PuCaha { pu, caha }) => {
                (vec![pu, caha.value], caha.free_modifiers)
            }
            data!(TenseModalSyntax::SpaceDistance(word)) => (vec![word.value], word.free_modifiers),
            data!(TenseModalSyntax::SpaceDirection(word)) => {
                (vec![word.value], word.free_modifiers)
            }
            data!(TenseModalSyntax::SpaceMovement {
                mohi,
                direction,
                distance,
            }) => {
                let mut words = vec![mohi, direction.value];
                let mut free_modifiers = direction.free_modifiers;
                if let Some(distance) = distance {
                    words.push(distance.value);
                    free_modifiers = distance.free_modifiers;
                }
                (words, free_modifiers)
            }
            data!(TenseModalSyntax::Simple {
                nahe,
                se,
                bai,
                nai,
                ki,
            }) => {
                let mut words = Vec::new();
                let nahe_is_some = nahe.is_some();
                let nahe_free_modifiers = if let Some(nahe) = nahe {
                    words.push(nahe.value);
                    nahe.free_modifiers
                } else {
                    Vec::new()
                };
                let se_is_some = se.is_some();
                let se_free_modifiers = if let Some(se) = se {
                    words.push(se.value);
                    se.free_modifiers
                } else {
                    Vec::new()
                };
                words.push(bai.value);
                let bai_free_modifiers = bai.free_modifiers;
                let nai_is_some = nai.is_some();
                let nai_free_modifiers = if let Some(nai) = nai {
                    words.push(nai.value);
                    nai.free_modifiers
                } else {
                    Vec::new()
                };
                let ki_is_some = ki.is_some();
                let ki_free_modifiers = if let Some(ki) = ki {
                    words.push(ki.value);
                    ki.free_modifiers
                } else {
                    Vec::new()
                };
                let free_modifiers = if ki_is_some {
                    ki_free_modifiers
                } else if nai_is_some {
                    nai_free_modifiers
                } else if !bai_free_modifiers.is_empty() {
                    bai_free_modifiers
                } else if se_is_some {
                    se_free_modifiers
                } else if nahe_is_some {
                    nahe_free_modifiers
                } else {
                    bai_free_modifiers
                };
                (words, free_modifiers)
            }
            data!(TenseModalSyntax::Ki(ki)) => (vec![ki.value], ki.free_modifiers),
            data!(TenseModalSyntax::Fiho {
                fiho,
                relation,
                fehu,
            }) => {
                let mut words = vec![fiho.value];
                let mut free_modifiers = fiho.free_modifiers;
                words.extend((*relation).words());
                if let Some(fehu) = fehu {
                    words.push(fehu.value);
                    free_modifiers = fehu.free_modifiers;
                }
                (words, free_modifiers)
            }
            data!(TenseModalSyntax::Zaho(words)) => (words.value, words.free_modifiers),
            data!(TenseModalSyntax::Interval {
                number,
                roi_or_tahe,
                nai,
            }) => {
                let mut words = number.map_or_else(Vec::new, WordRun::into_vec);
                words.push(roi_or_tahe.value);
                let mut free_modifiers = roi_or_tahe.free_modifiers;
                if let Some(nai) = nai {
                    words.push(nai.value);
                    free_modifiers = nai.free_modifiers;
                }
                (words, free_modifiers)
            }
        }
    }

    #[requires(true)]
    #[ensures(true)]
    pub fn leaf_words(self) -> Vec<Token> {
        self.leaf_words_and_free_modifiers().0
    }

    #[requires(true)]
    #[ensures(true)]
    pub fn words(self) -> Vec<Token> {
        let mut words = Vec::new();
        self.extend_words_into(&mut words);
        words
    }

    #[requires(true)]
    #[ensures(true)]
    pub fn free_modifiers(self) -> Vec<FreeModifierSyntax> {
        self.leaf_words_and_free_modifiers().1
    }
}

impl TextSyntax {
    #[requires(true)]
    #[ensures(true)]
    pub fn visit_words(&self, visitor: &mut impl FnMut(&Token)) {
        visit_word_slice(&self.leading_nai, visitor);
        visit_word_slice(&self.leading_cmevla, visitor);
        for indicator in &self.leading_indicators {
            indicator.visit_words(visitor);
        }
        for free_modifier in &self.leading_free_modifiers {
            free_modifier.visit_words(visitor);
        }
        if let Some(leading_connective) = &self.leading_connective {
            leading_connective.visit_words(visitor);
        }
        for paragraph in &self.paragraphs {
            paragraph.visit_words(visitor);
        }
    }

    #[requires(true)]
    #[ensures(true)]
    pub fn word_count(&self) -> usize {
        let mut count = 0;
        self.visit_words(&mut |_| count += 1);
        count
    }
}

impl ParagraphSyntax {
    #[requires(true)]
    #[ensures(true)]
    pub fn visit_words(&self, visitor: &mut impl FnMut(&Token)) {
        if let Some(i) = &self.i {
            visitor(i);
        }
        visit_word_slice(&self.niho, visitor);
        for free_modifier in &self.free_modifiers {
            free_modifier.visit_words(visitor);
        }
        for paragraph_statement in &self.statements {
            paragraph_statement.visit_words(visitor);
        }
    }
}

impl ParagraphStatementSyntax {
    #[requires(true)]
    #[ensures(true)]
    pub fn visit_words(&self, visitor: &mut impl FnMut(&Token)) {
        if let Some(i) = &self.i {
            visitor(i);
        }
        if let Some(connective) = &self.connective {
            connective.visit_words(visitor);
        }
        for free_modifier in &self.free_modifiers {
            free_modifier.visit_words(visitor);
        }
        if let Some(statement) = &self.statement {
            statement.visit_words(visitor);
        }
    }
}

impl StatementSyntax {
    #[requires(true)]
    #[ensures(true)]
    pub fn visit_words(&self, visitor: &mut impl FnMut(&Token)) {
        match self.as_data() {
            data!(StatementSyntax::Tuhe {
                tense_modal,
                tuhe,
                text,
                tuhu,
            }) => {
                if let Some(tense_modal) = tense_modal {
                    tense_modal.visit_words(visitor);
                }
                tuhe.visit_words(visitor);
                text.visit_words(visitor);
                if let Some(tuhu) = tuhu {
                    tuhu.visit_words(visitor);
                }
            }
            data!(StatementSyntax::Prenex {
                prenex_terms,
                zohu,
                inner_statement,
            }) => {
                for term in prenex_terms {
                    term.visit_words(visitor);
                }
                zohu.visit_words(visitor);
                inner_statement.visit_words(visitor);
            }
            data!(StatementSyntax::Predicate(predicate)) => predicate.visit_words(visitor),
            data!(StatementSyntax::Connected {
                i,
                connective,
                leading_statement,
                trailing_statement,
            }) => {
                leading_statement.visit_words(visitor);
                visitor(i);
                connective.visit_words(visitor);
                trailing_statement.visit_words(visitor);
            }
            data!(StatementSyntax::PreIConnected {
                connective,
                i,
                leading_statement,
                trailing_statement,
            }) => {
                leading_statement.visit_words(visitor);
                connective.visit_words(visitor);
                visitor(i);
                trailing_statement.visit_words(visitor);
            }
            data!(StatementSyntax::Iau {
                inner_statement,
                iau,
                reset_terms,
            }) => {
                inner_statement.visit_words(visitor);
                iau.visit_words(visitor);
                for term in reset_terms {
                    term.visit_words(visitor);
                }
            }
            data!(StatementSyntax::ExperimentalPredicateContinuation {
                leading_statement,
                continuation,
            }) => {
                leading_statement.visit_words(visitor);
                continuation.visit_words(visitor);
            }
            data!(StatementSyntax::Fragment(fragment)) => fragment.visit_words(visitor),
        }
    }

    #[requires(true)]
    #[ensures(true)]
    pub fn word_count(&self) -> usize {
        let mut count = 0;
        self.visit_words(&mut |_| count += 1);
        count
    }
}

impl PredicateStatementContinuationSyntax {
    #[requires(true)]
    #[ensures(true)]
    pub fn visit_words(&self, visitor: &mut impl FnMut(&Token)) {
        self.connective.visit_words(visitor);
        if let Some(tense_modal) = &self.tense_modal {
            tense_modal.visit_words(visitor);
        }
        match self.marker.as_data() {
            data!(PredicateStatementContinuationMarkerSyntax::Bo(bo)) => {
                bo.visit_words(visitor);
                self.trailing_subsentence.visit_words(visitor);
            }
            data!(PredicateStatementContinuationMarkerSyntax::Ke { ke, kehe }) => {
                ke.visit_words(visitor);
                self.trailing_subsentence.visit_words(visitor);
                if let Some(kehe) = kehe {
                    kehe.visit_words(visitor);
                }
            }
        }
    }
}

impl FreeModifierSyntax {
    #[requires(true)]
    #[ensures(true)]
    pub fn extend_words_into(self, out: &mut Vec<Token>) {
        out.extend(self.words());
    }

    #[requires(true)]
    #[ensures(true)]
    pub fn visit_words(&self, visitor: &mut impl FnMut(&Token)) {
        match self.as_data() {
            data!(FreeModifierSyntax::Sei {
                sei,
                terms,
                cu,
                relation,
                sehu,
            }) => {
                sei.visit_words(visitor);
                for term in terms {
                    term.visit_words(visitor);
                }
                if let Some(cu) = cu {
                    cu.visit_words(visitor);
                }
                relation.visit_words(visitor);
                if let Some(sehu) = sehu {
                    sehu.visit_words(visitor);
                }
            }
            data!(FreeModifierSyntax::To { to, text, toi }) => {
                to.visit_words(visitor);
                text.visit_words(visitor);
                if let Some(toi) = toi {
                    toi.visit_words(visitor);
                }
            }
            data!(FreeModifierSyntax::Xi { xi, expression }) => {
                xi.visit_words(visitor);
                expression.visit_words(visitor);
            }
            data!(FreeModifierSyntax::Mai { number, mai }) => {
                visit_word_slice(number, visitor);
                mai.visit_words(visitor);
            }
            data!(FreeModifierSyntax::Soi {
                soi,
                leading_argument,
                trailing_argument,
                sehu,
            }) => {
                soi.visit_words(visitor);
                leading_argument.visit_words(visitor);
                if let Some(argument) = trailing_argument {
                    argument.visit_words(visitor);
                }
                if let Some(sehu) = sehu {
                    sehu.visit_words(visitor);
                }
            }
            data!(FreeModifierSyntax::Vocative {
                vocative_markers,
                argument,
                dohu,
            }) => {
                vocative_markers.visit_words(visitor);
                if let Some(argument) = argument {
                    argument.visit_words(visitor);
                }
                if let Some(dohu) = dohu {
                    dohu.visit_words(visitor);
                }
            }
            data!(FreeModifierSyntax::Replacement {
                lohai,
                old_words,
                sahai,
                new_words,
                lehai,
            }) => {
                if let Some(lohai) = lohai {
                    visitor(lohai);
                }
                visit_word_slice(old_words, visitor);
                if let Some(sahai) = sahai {
                    visitor(sahai);
                }
                visit_word_slice(new_words, visitor);
                lehai.visit_words(visitor);
            }
        }
    }

    #[requires(true)]
    #[ensures(true)]
    pub fn word_count(&self) -> usize {
        let mut count = 0;
        self.visit_words(&mut |_| count += 1);
        count
    }

    #[requires(true)]
    #[ensures(true)]
    pub fn first_word(&self) -> Option<&Token> {
        match self.as_data() {
            data!(FreeModifierSyntax::Sei { sei, .. }) => sei.first_word(),
            data!(FreeModifierSyntax::To { to, .. }) => to.first_word(),
            data!(FreeModifierSyntax::Xi { xi, .. }) => xi.first_word(),
            data!(FreeModifierSyntax::Mai { number, .. }) => Some(number.first()),
            data!(FreeModifierSyntax::Soi { soi, .. }) => soi.first_word(),
            data!(FreeModifierSyntax::Vocative {
                vocative_markers,
                ..
            }) => vocative_markers.first_word(),
            data!(FreeModifierSyntax::Replacement {
                lohai,
                old_words,
                sahai,
                new_words,
                lehai,
            }) => lohai
                .as_ref()
                .or_else(|| old_words.first())
                .or(sahai.as_ref())
                .or_else(|| new_words.first())
                .or_else(|| lehai.first_word()),
        }
    }
}

impl PredicateSyntax {
    #[requires(true)]
    #[ensures(true)]
    pub fn visit_words(&self, visitor: &mut impl FnMut(&Token)) {
        for term in &self.leading_terms {
            term.visit_words(visitor);
        }
        if let Some(cu) = &self.cu {
            cu.visit_words(visitor);
        }
        self.predicate_tail.visit_words(visitor);
        for free_modifier in &self.free_modifiers {
            free_modifier.visit_words(visitor);
        }
    }
}

impl PredicateTailSyntax {
    #[requires(true)]
    #[ensures(true)]
    pub fn visit_words(&self, visitor: &mut impl FnMut(&Token)) {
        self.first.visit_words(visitor);
        if let Some(ke_continuation) = &self.ke_continuation {
            ke_continuation.visit_words(visitor);
        }
    }
}

impl KePredicateTailSyntax {
    #[requires(true)]
    #[ensures(true)]
    pub fn visit_words(&self, visitor: &mut impl FnMut(&Token)) {
        self.connective.visit_words(visitor);
        if let Some(tense_modal) = &self.tense_modal {
            tense_modal.visit_words(visitor);
        }
        self.ke.visit_words(visitor);
        self.predicate_tail.visit_words(visitor);
        if let Some(kehe) = &self.kehe {
            kehe.visit_words(visitor);
        }
        for term in &self.tail_terms {
            term.visit_words(visitor);
        }
        if let Some(vau) = &self.vau {
            vau.visit_words(visitor);
        }
        for free_modifier in &self.free_modifiers {
            free_modifier.visit_words(visitor);
        }
    }
}

impl PredicateTail1Syntax {
    #[requires(true)]
    #[ensures(true)]
    pub fn visit_words(&self, visitor: &mut impl FnMut(&Token)) {
        self.first.visit_words(visitor);
        for continuation in &self.continuations {
            continuation.visit_words(visitor);
        }
    }
}

impl PredicateTailContinuationSyntax {
    #[requires(true)]
    #[ensures(true)]
    pub fn visit_words(&self, visitor: &mut impl FnMut(&Token)) {
        self.connective.visit_words(visitor);
        if let Some(tense_modal) = &self.tense_modal {
            tense_modal.visit_words(visitor);
        }
        if let Some(cu) = &self.cu {
            cu.visit_words(visitor);
        }
        self.predicate_tail.visit_words(visitor);
        for term in &self.tail_terms {
            term.visit_words(visitor);
        }
        if let Some(vau) = &self.vau {
            vau.visit_words(visitor);
        }
        for free_modifier in &self.free_modifiers {
            free_modifier.visit_words(visitor);
        }
    }
}

impl PredicateTail2Syntax {
    #[requires(true)]
    #[ensures(true)]
    pub fn visit_words(&self, visitor: &mut impl FnMut(&Token)) {
        self.first.visit_words(visitor);
        if let Some(bo_continuation) = &self.bo_continuation {
            bo_continuation.visit_words(visitor);
        }
    }
}

impl BoPredicateTailSyntax {
    #[requires(true)]
    #[ensures(true)]
    pub fn visit_words(&self, visitor: &mut impl FnMut(&Token)) {
        self.connective.visit_words(visitor);
        if let Some(tense_modal) = &self.tense_modal {
            tense_modal.visit_words(visitor);
        }
        self.bo.visit_words(visitor);
        if let Some(cu) = &self.cu {
            cu.visit_words(visitor);
        }
        self.predicate_tail.visit_words(visitor);
        for term in &self.tail_terms {
            term.visit_words(visitor);
        }
        if let Some(vau) = &self.vau {
            vau.visit_words(visitor);
        }
        for free_modifier in &self.free_modifiers {
            free_modifier.visit_words(visitor);
        }
    }
}

impl PredicateTail3Syntax {
    #[requires(true)]
    #[ensures(true)]
    pub fn visit_words(&self, visitor: &mut impl FnMut(&Token)) {
        match self.as_data() {
            data!(PredicateTail3Syntax::Relation {
                relation,
                terms,
                vau,
                free_modifiers,
            }) => {
                relation.visit_words(visitor);
                for term in terms {
                    term.visit_words(visitor);
                }
                if let Some(vau) = vau {
                    vau.visit_words(visitor);
                }
                for free_modifier in free_modifiers {
                    free_modifier.visit_words(visitor);
                }
            }
            data!(PredicateTail3Syntax::GekSentence(gek_sentence)) => {
                gek_sentence.visit_words(visitor)
            }
        }
    }
}

impl GekSentenceSyntax {
    #[requires(true)]
    #[ensures(true)]
    pub fn visit_words(&self, visitor: &mut impl FnMut(&Token)) {
        match self.as_data() {
            data!(GekSentenceSyntax::Pair {
                gek,
                first,
                gik,
                second,
                gihi,
                tail_terms,
                vau,
                free_modifiers,
            }) => {
                gek.visit_words(visitor);
                first.visit_words(visitor);
                gik.visit_words(visitor);
                second.visit_words(visitor);
                if let Some(gihi) = gihi {
                    visitor(gihi);
                }
                for term in tail_terms {
                    term.visit_words(visitor);
                }
                if let Some(vau) = vau {
                    vau.visit_words(visitor);
                }
                for free_modifier in free_modifiers {
                    free_modifier.visit_words(visitor);
                }
            }
            data!(GekSentenceSyntax::Ke {
                tense_modal,
                ke,
                inner,
                kehe,
            }) => {
                if let Some(tense_modal) = tense_modal {
                    tense_modal.visit_words(visitor);
                }
                ke.visit_words(visitor);
                inner.visit_words(visitor);
                if let Some(kehe) = kehe {
                    kehe.visit_words(visitor);
                }
            }
            data!(GekSentenceSyntax::Na { na, inner }) => {
                na.visit_words(visitor);
                inner.visit_words(visitor);
            }
        }
    }
}
