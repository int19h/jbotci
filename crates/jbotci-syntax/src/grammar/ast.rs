//! Syntax AST behavior and parser-facing helpers.

pub use crate::tree::*;

use std::sync::Arc;

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
            data!(StatementSyntax::TextGroup {
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
            data!(StatementSyntax::Bridi(bridi)) => bridi.words(),
            data!(StatementSyntax::StatementConnection {
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
            data!(StatementSyntax::PreposedIStatementConnection {
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
            data!(StatementSyntax::ExperimentalBridiContinuation {
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

impl BridiStatementContinuationSyntax {
    #[requires(true)]
    #[ensures(true)]
    pub fn words(self) -> Vec<Token> {
        let mut words = self.connective.words();
        if let Some(tense_modal) = self.tense_modal {
            words.extend(tense_modal.words());
        }
        match self.marker.into_data() {
            data!(BridiStatementContinuationMarkerSyntax::BoGrouped(bo)) => {
                words.extend(bo.words());
                words.extend(self.trailing_subbridi.words());
            }
            data!(BridiStatementContinuationMarkerSyntax::KeGrouped { ke, kehe }) => {
                words.extend(ke.words());
                words.extend(self.trailing_subbridi.words());
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
            data!(FreeModifierSyntax::MetalinguisticBridi {
                sei,
                terms,
                cu,
                selbri,
                sehu,
            }) => {
                let mut words = sei.words();
                for term in terms {
                    words.extend(term.words());
                }
                if let Some(cu) = cu {
                    words.extend(cu.words());
                }
                words.extend(selbri.words());
                if let Some(sehu) = sehu {
                    words.extend(sehu.words());
                }
                words
            }
            data!(FreeModifierSyntax::ParentheticalText { to, text, toi }) => {
                let mut words = to.words();
                words.extend(text.words());
                if let Some(toi) = toi {
                    words.extend(toi.words());
                }
                words
            }
            data!(FreeModifierSyntax::Subscript { xi, expression }) => {
                let mut words = xi.words();
                words.extend(expression.words());
                words
            }
            data!(FreeModifierSyntax::UtteranceOrdinal { number, mai }) => {
                let mut words = number.into_vec();
                words.extend(mai.words());
                words
            }
            data!(FreeModifierSyntax::ReciprocalSumti {
                soi,
                leading_sumti,
                trailing_sumti,
                sehu,
            }) => {
                let mut words = soi.words();
                words.extend(leading_sumti.words());
                if let Some(sumti) = trailing_sumti {
                    words.extend(sumti.words());
                }
                if let Some(sehu) = sehu {
                    words.extend(sehu.words());
                }
                words
            }
            data!(FreeModifierSyntax::Vocative {
                vocative_markers,
                sumti,
                dohu,
            }) => {
                let mut words = vocative_markers.words();
                if let Some(sumti) = sumti {
                    words.extend(sumti.words());
                }
                if let Some(dohu) = dohu {
                    words.extend(dohu.words());
                }
                words
            }
            data!(FreeModifierSyntax::TextReplacement {
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

impl BridiSyntax {
    #[requires(true)]
    #[ensures(true)]
    pub fn words(self) -> Vec<Token> {
        let data!(BridiSyntax {
            leading_terms,
            cu,
            bridi_tail,
            free_modifiers,
        }) = self.into_data();
        let mut words = Vec::new();
        for term in leading_terms {
            words.extend(term.words());
        }
        if let Some(cu) = cu {
            words.extend(unwrap_or_clone_arc(cu).words());
        }
        words.extend(bridi_tail.words());
        for free_modifier in free_modifiers {
            words.extend(free_modifier.words());
        }
        words
    }
}

impl BridiTailSyntax {
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

impl GroupedBridiTailConnectionSyntax {
    #[requires(true)]
    #[ensures(true)]
    pub fn words(self) -> Vec<Token> {
        let data!(GroupedBridiTailConnectionSyntax {
            connective,
            tense_modal,
            ke,
            bridi_tail,
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
        words.extend(bridi_tail.words());
        if let Some(kehe) = kehe {
            words.extend(unwrap_or_clone_arc(kehe).words());
        }
        for term in tail_terms {
            words.extend(term.words());
        }
        if let Some(vau) = vau {
            words.extend(unwrap_or_clone_arc(vau).words());
        }
        for free_modifier in free_modifiers {
            words.extend(free_modifier.words());
        }
        words
    }
}

impl AfterthoughtBridiTailSyntax {
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

impl BridiTailConnectionSyntax {
    #[requires(true)]
    #[ensures(true)]
    pub fn words(self) -> Vec<Token> {
        let data!(BridiTailConnectionSyntax {
            connective,
            tense_modal,
            cu,
            bridi_tail,
            tail_terms,
            vau,
            free_modifiers,
        }) = self.into_data();
        let mut words = connective.words();
        if let Some(tense_modal) = tense_modal {
            words.extend(tense_modal.words());
        }
        if let Some(cu) = cu {
            words.extend(unwrap_or_clone_arc(cu).words());
        }
        words.extend(bridi_tail.words());
        for term in tail_terms {
            words.extend(term.words());
        }
        if let Some(vau) = vau {
            words.extend(unwrap_or_clone_arc(vau).words());
        }
        for free_modifier in free_modifiers {
            words.extend(free_modifier.words());
        }
        words
    }
}

impl BoGroupedBridiTailSyntax {
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

impl BoundBridiTailConnectionSyntax {
    #[requires(true)]
    #[ensures(true)]
    pub fn words(self) -> Vec<Token> {
        let data!(BoundBridiTailConnectionSyntax {
            connective,
            tense_modal,
            bo,
            cu,
            bridi_tail,
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
            words.extend(unwrap_or_clone_arc(cu).words());
        }
        words.extend(bridi_tail.words());
        for term in tail_terms {
            words.extend(term.words());
        }
        if let Some(vau) = vau {
            words.extend(unwrap_or_clone_arc(vau).words());
        }
        for free_modifier in free_modifiers {
            words.extend(free_modifier.words());
        }
        words
    }
}

impl SimpleBridiTailSyntax {
    #[requires(true)]
    #[ensures(true)]
    pub fn words(self) -> Vec<Token> {
        match self.into_data() {
            data!(SimpleBridiTailSyntax::SelbriBridiTail {
                selbri,
                terms,
                vau,
                free_modifiers,
            }) => {
                let mut words = selbri.words();
                for term in terms {
                    words.extend(term.words());
                }
                if let Some(vau) = vau {
                    words.extend(unwrap_or_clone_arc(vau).words());
                }
                for free_modifier in free_modifiers {
                    words.extend(free_modifier.words());
                }
                words
            }
            data!(SimpleBridiTailSyntax::ForethoughtBridiTailConnection(
                forethought_connection
            )) => forethought_connection.words(),
            data!(SimpleBridiTailSyntax::TermPrefixedBridiTail { terms, bridi_tail }) => {
                let mut words = Vec::new();
                for term in terms {
                    words.extend(term.words());
                }
                words.extend(bridi_tail.words());
                words
            }
        }
    }
}

impl ForethoughtBridiConnectionSyntax {
    #[requires(true)]
    #[ensures(true)]
    pub fn words(self) -> Vec<Token> {
        match self.into_data() {
            data!(ForethoughtBridiConnectionSyntax::BridiConnection {
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
                    words.extend(unwrap_or_clone_arc(vau).words());
                }
                for free_modifier in free_modifiers {
                    words.extend(free_modifier.words());
                }
                words
            }
            data!(ForethoughtBridiConnectionSyntax::GroupedBridiConnection {
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
                    words.extend(unwrap_or_clone_arc(kehe).words());
                }
                words
            }
            data!(ForethoughtBridiConnectionSyntax::NegatedBridiConnection { na, inner }) => {
                let mut words = na.words();
                words.extend(inner.words());
                words
            }
        }
    }
}

impl SubbridiSyntax {
    #[requires(true)]
    #[ensures(true)]
    pub fn visit_words(&self, visitor: &mut impl FnMut(&Token)) {
        match self.as_data() {
            data!(SubbridiSyntax::Bridi(bridi)) => bridi.visit_words(visitor),
            data!(SubbridiSyntax::Prenex {
                prenex_terms,
                zohu,
                inner_subbridi,
            }) => {
                for term in prenex_terms {
                    term.visit_words(visitor);
                }
                zohu.visit_words(visitor);
                inner_subbridi.visit_words(visitor);
            }
        }
    }
}

impl FragmentSyntax {
    #[requires(true)]
    #[ensures(true)]
    pub fn visit_words(&self, visitor: &mut impl FnMut(&Token)) {
        match self.as_data() {
            data!(FragmentSyntax::Ek(connective))
            | data!(FragmentSyntax::BridiTailConnective(connective)) => {
                connective.visit_words(visitor);
            }
            data!(FragmentSyntax::Other(words)) => words.visit_words(visitor),
            data!(FragmentSyntax::BridiConnective { i, connective }) => {
                visitor(i);
                connective.visit_words(visitor);
            }
            data!(FragmentSyntax::Prenex { terms, zohu }) => {
                for term in terms {
                    term.visit_words(visitor);
                }
                zohu.visit_words(visitor);
            }
            data!(FragmentSyntax::LinkedSumti {
                be,
                fa,
                first_sumti,
                bei_links,
                beho,
            }) => {
                be.visit_words(visitor);
                if let Some(fa) = fa {
                    fa.visit_words(visitor);
                }
                if let Some(first_sumti) = first_sumti {
                    first_sumti.visit_words(visitor);
                }
                for bei_link in bei_links {
                    bei_link.visit_words(visitor);
                }
                if let Some(beho) = beho {
                    beho.visit_words(visitor);
                }
            }
            data!(FragmentSyntax::LinkedSumtiContinuation(bei_only_links)) => {
                for bei_link in bei_only_links {
                    bei_link.visit_words(visitor);
                }
            }
            data!(FragmentSyntax::RelativeClauses(relative_clauses)) => {
                for relative_clause in relative_clauses {
                    relative_clause.visit_words(visitor);
                }
            }
            data!(FragmentSyntax::Mekso(mekso)) => mekso.visit_words(visitor),
            data!(FragmentSyntax::Terms { terms, vau }) => {
                for term in terms {
                    term.visit_words(visitor);
                }
                if let Some(vau) = vau {
                    vau.visit_words(visitor);
                }
            }
            data!(FragmentSyntax::Selbri(selbri)) => selbri.visit_words(visitor),
        }
    }
}

impl TermSyntax {
    #[requires(true)]
    #[ensures(true)]
    pub fn visit_words(&self, visitor: &mut impl FnMut(&Token)) {
        match self.as_data() {
            data!(TermSyntax::Termset {
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
            data!(TermSyntax::ForethoughtTermsetConnection {
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
            data!(TermSyntax::TermsetGroup {
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
            data!(TermSyntax::TermsetConnection {
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
            data!(TermSyntax::Sumti(sumti)) => sumti.visit_words(visitor),
            data!(TermSyntax::PlaceTaggedSumti { fa, sumti, ku }) => {
                fa.visit_words(visitor);
                sumti.visit_words(visitor);
                if let Some(ku) = ku {
                    ku.visit_words(visitor);
                }
            }
            data!(TermSyntax::BridiNegation { na, na_ku }) => {
                visitor(na);
                na_ku.visit_words(visitor);
            }
            data!(TermSyntax::BareNegation(na)) => na.visit_words(visitor),
            data!(TermSyntax::RelativeAdverbialTerm {
                noiha,
                tail_elements,
                selbri,
                relative_clauses,
                fehu,
            }) => {
                noiha.visit_words(visitor);
                for tail_element in tail_elements {
                    tail_element.visit_words(visitor);
                }
                if let Some(selbri) = selbri {
                    selbri.visit_words(visitor);
                }
                for relative_clause in relative_clauses {
                    relative_clause.visit_words(visitor);
                }
                if let Some(fehu) = fehu {
                    fehu.visit_words(visitor);
                }
            }
            data!(TermSyntax::BridiVariableAdverbialTerm {
                poiha,
                tail_elements,
                selbri,
                relative_clauses,
                brigahi_ku,
            }) => {
                poiha.visit_words(visitor);
                for tail_element in tail_elements {
                    tail_element.visit_words(visitor);
                }
                if let Some(selbri) = selbri {
                    selbri.visit_words(visitor);
                }
                for relative_clause in relative_clauses {
                    relative_clause.visit_words(visitor);
                }
                brigahi_ku.visit_words(visitor);
            }
            data!(TermSyntax::AdHocBridiAdverbialTerm {
                fihoi,
                subbridi,
                fihau,
            }) => {
                fihoi.visit_words(visitor);
                subbridi.visit_words(visitor);
                if let Some(fihau) = fihau {
                    fihau.visit_words(visitor);
                }
            }
            data!(TermSyntax::ReciprocalBridiAdverbialTerm {
                soi,
                subbridi,
                sehu,
            }) => {
                soi.visit_words(visitor);
                subbridi.visit_words(visitor);
                if let Some(sehu) = sehu {
                    sehu.visit_words(visitor);
                }
            }
            data!(TermSyntax::JaiTaggedSumti { jai, tag, sumti }) => {
                jai.visit_words(visitor);
                if let Some(tag) = tag {
                    tag.visit_words(visitor);
                }
                sumti.visit_words(visitor);
            }
            data!(TermSyntax::TaggedSumti { tense_modal, sumti }) => {
                if let Some(tense_modal) = tense_modal {
                    tense_modal.visit_words(visitor);
                }
                sumti.visit_words(visitor);
            }
            data!(TermSyntax::TermConnection {
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
            data!(TermSyntax::BoundTermConnection {
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

impl SumtiTagSyntax {
    #[requires(true)]
    #[ensures(true)]
    pub fn visit_words(&self, visitor: &mut impl FnMut(&Token)) {
        match self.as_data() {
            data!(SumtiTagSyntax::TenseModal(tense_modal)) => tense_modal.visit_words(visitor),
            data!(SumtiTagSyntax::PlaceTag(fa)) => fa.visit_words(visitor),
        }
    }
}

impl MeksoSyntax {
    #[requires(true)]
    #[ensures(true)]
    pub fn visit_words(&self, visitor: &mut impl FnMut(&Token)) {
        match self.as_data() {
            data!(MeksoSyntax::NumberMekso(quantifier)) => quantifier.visit_words(visitor),
            data!(MeksoSyntax::LerfuStringMekso { letter, boi }) => {
                letter.visit_words(visitor);
                if let Some(boi) = boi {
                    boi.visit_words(visitor);
                }
            }
            data!(MeksoSyntax::ParenthesizedMekso {
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
            data!(MeksoSyntax::ForethoughtMeksoConnection {
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
            data!(MeksoSyntax::ForethoughtCall {
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
            data!(MeksoSyntax::ReversePolish {
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
            data!(MeksoSyntax::SelbriOperand { nihe, selbri, tehu }) => {
                nihe.visit_words(visitor);
                selbri.visit_words(visitor);
                if let Some(tehu) = tehu {
                    tehu.visit_words(visitor);
                }
            }
            data!(MeksoSyntax::SumtiOperand { mohe, sumti, tehu }) => {
                mohe.visit_words(visitor);
                sumti.visit_words(visitor);
                if let Some(tehu) = tehu {
                    tehu.visit_words(visitor);
                }
            }
            data!(MeksoSyntax::MeksoArray {
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
            data!(MeksoSyntax::QualifiedOperand {
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
            data!(MeksoSyntax::MeksoConnection {
                left_expression,
                connective,
                right_expression,
            }) => {
                left_expression.visit_words(visitor);
                connective.visit_words(visitor);
                right_expression.visit_words(visitor);
            }
            data!(MeksoSyntax::Infix {
                operator,
                left_expression,
                right_expression,
            }) => {
                left_expression.visit_words(visitor);
                operator.visit_words(visitor);
                right_expression.visit_words(visitor);
            }
            data!(MeksoSyntax::PrecedenceInfix {
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

impl SumtiSyntax {
    #[requires(true)]
    #[ensures(true)]
    pub fn visit_words(&self, visitor: &mut impl FnMut(&Token)) {
        match self.as_data() {
            data!(SumtiSyntax::QuotedSumti(quote)) => quote.visit_words(visitor),
            data!(SumtiSyntax::NumberSumti {
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
            data!(SumtiSyntax::LerfuStringSumti { letter, boi }) => {
                letter.visit_words(visitor);
                if let Some(boi) = boi {
                    boi.visit_words(visitor);
                }
            }
            data!(SumtiSyntax::QuantifiedSumti {
                quantifier,
                inner_sumti,
            }) => {
                quantifier.visit_words(visitor);
                inner_sumti.visit_words(visitor);
            }
            data!(SumtiSyntax::SumtiWithRelativeClauses {
                base_sumti,
                vuho,
                relative_clauses,
            }) => {
                base_sumti.visit_words(visitor);
                if let Some(vuho) = vuho {
                    vuho.visit_words(visitor);
                }
                for relative_clause in relative_clauses {
                    relative_clause.visit_words(visitor);
                }
            }
            data!(SumtiSyntax::SumtiWithComplexRelativeClauses {
                base_sumti,
                vuho_marker,
                relative_clauses,
                sumti_connection,
            }) => {
                base_sumti.visit_words(visitor);
                vuho_marker.visit_words(visitor);
                for relative_clause in relative_clauses {
                    relative_clause.visit_words(visitor);
                }
                if let Some(sumti_connection) = sumti_connection {
                    sumti_connection.connective.visit_words(visitor);
                    sumti_connection.sumti.visit_words(visitor);
                }
            }
            data!(SumtiSyntax::BridiDescription {
                lohoi,
                subbridi,
                kuhau,
            }) => {
                lohoi.visit_words(visitor);
                subbridi.visit_words(visitor);
                if let Some(kuhau) = kuhau {
                    kuhau.visit_words(visitor);
                }
            }
            data!(SumtiSyntax::NegatedSumti { na, ku }) => {
                visitor(na);
                ku.visit_words(visitor);
            }
            data!(SumtiSyntax::TaggedSumti { tag, inner_sumti }) => {
                tag.visit_words(visitor);
                inner_sumti.visit_words(visitor);
            }
            data!(SumtiSyntax::ScalarNegatedSumtiWithBo {
                nahe,
                bo,
                inner_sumti,
                luhu,
            }) => {
                visitor(nahe);
                bo.visit_words(visitor);
                inner_sumti.visit_words(visitor);
                if let Some(luhu) = luhu {
                    luhu.visit_words(visitor);
                }
            }
            data!(SumtiSyntax::ScalarNegatedSumti {
                nahe,
                inner_sumti,
                luhu,
            }) => {
                nahe.visit_words(visitor);
                inner_sumti.visit_words(visitor);
                if let Some(luhu) = luhu {
                    luhu.visit_words(visitor);
                }
            }
            data!(SumtiSyntax::QualifiedTerm {
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
            data!(SumtiSyntax::ProSumti(koha)) => koha.visit_words(visitor),
            data!(SumtiSyntax::ElidedSumti {
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
            data!(SumtiSyntax::ReferentSumti {
                lahe,
                relative_clauses,
                inner_sumti,
                luhu,
            }) => {
                lahe.visit_words(visitor);
                for relative_clause in relative_clauses {
                    relative_clause.visit_words(visitor);
                }
                inner_sumti.visit_words(visitor);
                if let Some(luhu) = luhu {
                    luhu.visit_words(visitor);
                }
            }
            data!(SumtiSyntax::SumtiConnection {
                leading_sumti,
                connective,
                trailing_sumti,
            }) => {
                leading_sumti.visit_words(visitor);
                connective.visit_words(visitor);
                trailing_sumti.visit_words(visitor);
            }
            data!(SumtiSyntax::GroupedSumti {
                ke,
                inner_sumti,
                kehe,
            }) => {
                ke.visit_words(visitor);
                inner_sumti.visit_words(visitor);
                if let Some(kehe) = kehe {
                    kehe.visit_words(visitor);
                }
            }
            data!(SumtiSyntax::BoundSumtiConnection {
                leading_sumti,
                bo_connective,
                bo_tense_modal,
                bo,
                trailing_sumti,
            }) => {
                leading_sumti.visit_words(visitor);
                if let Some(connective) = bo_connective {
                    connective.visit_words(visitor);
                }
                if let Some(tense_modal) = bo_tense_modal {
                    tense_modal.visit_words(visitor);
                }
                bo.visit_words(visitor);
                trailing_sumti.visit_words(visitor);
            }
            data!(SumtiSyntax::ForethoughtSumtiConnection {
                gek,
                leading_sumti,
                gik,
                trailing_sumti,
                gihi,
            }) => {
                gek.visit_words(visitor);
                leading_sumti.visit_words(visitor);
                gik.visit_words(visitor);
                trailing_sumti.visit_words(visitor);
                if let Some(gihi) = gihi {
                    visitor(gihi);
                }
            }
            data!(SumtiSyntax::Description(description)) => description.visit_words(visitor),
            data!(SumtiSyntax::DescriptionConnection(description_connection)) => {
                description_connection.visit_words(visitor);
            }
            data!(SumtiSyntax::NameDescription { la, names }) => {
                la.visit_words(visitor);
                names.visit_words(visitor);
            }
            data!(SumtiSyntax::NameWords(cmevla)) => cmevla.visit_words(visitor),
            data!(SumtiSyntax::SelbriVocative {
                leading_relative_clauses,
                selbri,
                trailing_relative_clauses,
            }) => {
                for relative_clause in leading_relative_clauses {
                    relative_clause.visit_words(visitor);
                }
                selbri.visit_words(visitor);
                for relative_clause in trailing_relative_clauses {
                    relative_clause.visit_words(visitor);
                }
            }
        }
    }
}

impl SumtiAssociationPhraseSyntax {
    #[requires(true)]
    #[ensures(true)]
    pub fn visit_words(&self, visitor: &mut impl FnMut(&Token)) {
        self.association_marker.visit_words(visitor);
        self.sumti.visit_words(visitor);
        if let Some(gehu) = &self.gehu {
            gehu.visit_words(visitor);
        }
    }
}

impl SelbriRelativePhraseSyntax {
    #[requires(true)]
    #[ensures(true)]
    pub fn visit_words(&self, visitor: &mut impl FnMut(&Token)) {
        self.nohoi.visit_words(visitor);
        self.selbri.visit_words(visitor);
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
            data!(RelativeClauseSyntax::SumtiAssociationPhrase(
                relative_clause
            )) => relative_clause.visit_words(visitor),
            data!(RelativeClauseSyntax::IncidentalRelativeBridi {
                noi,
                subbridi,
                kuho,
            })
            | data!(RelativeClauseSyntax::RestrictiveRelativeBridi {
                poi: noi,
                subbridi,
                kuho,
            }) => {
                noi.visit_words(visitor);
                subbridi.visit_words(visitor);
                if let Some(kuho) = kuho {
                    kuho.visit_words(visitor);
                }
            }
            data!(RelativeClauseSyntax::JoinedRelativeClauses { zihe, inner }) => {
                zihe.visit_words(visitor);
                inner.visit_words(visitor);
            }
            data!(RelativeClauseSyntax::RelativeClauseConnection { connective, inner }) => {
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
            data!(QuoteSyntax::TextQuote { lu, text, lihu }) => {
                lu.visit_words(visitor);
                text.visit_words(visitor);
                if let Some(lihu) = lihu {
                    lihu.visit_words(visitor);
                }
            }
            data!(QuoteSyntax::WordQuote(zo)) | data!(QuoteSyntax::DelimitedNonLojbanQuote(zo)) => {
                zo.visit_words(visitor)
            }
            data!(QuoteSyntax::DelimitedWordQuote(zohoi)) => zohoi.visit_words(visitor),
            data!(QuoteSyntax::WordsQuote(lohu)) => lohu.visit_words(visitor),
        }
    }
}

impl DescriptionSyntax {
    #[requires(true)]
    #[ensures(true)]
    pub fn visit_words(&self, visitor: &mut impl FnMut(&Token)) {
        if let Some(quantifier) = &self.outer_quantifier {
            quantifier.visit_words(visitor);
        }
        if let Some(description) = &self.description {
            description.visit_words(visitor);
        }
        for element in &self.tail_elements {
            element.visit_words(visitor);
        }
        if let Some(selbri) = &self.selbri {
            selbri.visit_words(visitor);
        }
        for relative_clause in &self.relative_clauses {
            relative_clause.visit_words(visitor);
        }
        if let Some(ku) = &self.ku {
            ku.visit_words(visitor);
        }
    }
}

impl DescriptionHeadSyntax {
    #[requires(true)]
    #[ensures(true)]
    pub fn visit_words(&self, visitor: &mut impl FnMut(&Token)) {
        self.description.visit_words(visitor);
    }
}

impl DescriptionConnectionSyntax {
    #[requires(true)]
    #[ensures(true)]
    pub fn visit_words(&self, visitor: &mut impl FnMut(&Token)) {
        self.leading_description_head.visit_words(visitor);
        self.connective.visit_words(visitor);
        self.trailing_description_head.visit_words(visitor);
        for element in &self.tail_elements {
            element.visit_words(visitor);
        }
        if let Some(selbri) = &self.selbri {
            selbri.visit_words(visitor);
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

#[requires(true)]
#[ensures(true)]
fn unwrap_or_clone_arc<T: Clone>(value: Arc<T>) -> T {
    Arc::try_unwrap(value).unwrap_or_else(|value| value.as_ref().clone())
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
                cmavo: Arc::new(cmavo),
                nai: nai.map(Arc::new),
            }),
            ConnectiveKind::Selbri => new!(ConnectiveSyntax::Selbri {
                se,
                nahe,
                na,
                cmavo: Arc::new(cmavo),
                nai: nai.map(Arc::new),
            }),
            ConnectiveKind::BridiTail => new!(ConnectiveSyntax::BridiTail {
                se,
                nahe,
                na,
                cmavo: Arc::new(cmavo),
                nai: nai.map(Arc::new),
            }),
            ConnectiveKind::Forethought => new!(ConnectiveSyntax::Forethought {
                se,
                nahe,
                na,
                cmavo: Arc::new(cmavo),
                nai: nai.map(Arc::new),
            }),
            ConnectiveKind::NonLogical => new!(ConnectiveSyntax::NonLogical {
                se,
                nahe,
                na,
                cmavo: Arc::new(cmavo),
                nai: nai.map(Arc::new),
            }),
            ConnectiveKind::Interval => new!(ConnectiveSyntax::Interval {
                se,
                nahe,
                na,
                cmavo: Arc::new(cmavo),
                nai: nai.map(Arc::new),
            }),
        }
    }

    #[requires(true)]
    #[ensures(true)]
    pub fn kind(&self) -> ConnectiveKind {
        match self.as_data() {
            data!(ConnectiveSyntax::Afterthought { .. }) => ConnectiveKind::Afterthought,
            data!(ConnectiveSyntax::Selbri { .. }) => ConnectiveKind::Selbri,
            data!(ConnectiveSyntax::BridiTail { .. }) => ConnectiveKind::BridiTail,
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
            | data!(ConnectiveSyntax::Selbri { cmavo, .. })
            | data!(ConnectiveSyntax::BridiTail { cmavo, .. })
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
                cmavo: unwrap_or_clone_arc(cmavo),
                nai: nai.map(unwrap_or_clone_arc),
            },
            data!(ConnectiveSyntax::Selbri {
                se,
                nahe,
                na,
                cmavo,
                nai,
            }) => ConnectiveSyntaxParts {
                kind: ConnectiveKind::Selbri,
                se,
                nahe,
                na,
                cmavo: unwrap_or_clone_arc(cmavo),
                nai: nai.map(unwrap_or_clone_arc),
            },
            data!(ConnectiveSyntax::BridiTail {
                se,
                nahe,
                na,
                cmavo,
                nai,
            }) => ConnectiveSyntaxParts {
                kind: ConnectiveKind::BridiTail,
                se,
                nahe,
                na,
                cmavo: unwrap_or_clone_arc(cmavo),
                nai: nai.map(unwrap_or_clone_arc),
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
                cmavo: unwrap_or_clone_arc(cmavo),
                nai: nai.map(unwrap_or_clone_arc),
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
                cmavo: unwrap_or_clone_arc(cmavo),
                nai: nai.map(unwrap_or_clone_arc),
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
                cmavo: unwrap_or_clone_arc(cmavo),
                nai: nai.map(unwrap_or_clone_arc),
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
            | data!(ConnectiveSyntax::Selbri {
                se,
                nahe,
                na,
                cmavo,
                nai,
            })
            | data!(ConnectiveSyntax::BridiTail {
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

impl AdditionalLinkedSumtiSyntax {
    #[requires(true)]
    #[ensures(true)]
    pub fn visit_words(&self, visitor: &mut impl FnMut(&Token)) {
        self.bei.visit_words(visitor);
        if let Some(fa) = &self.fa {
            fa.visit_words(visitor);
        }
        if let Some(sumti) = &self.sumti {
            sumti.visit_words(visitor);
        }
    }
}

impl DescriptionTailElementSyntax {
    #[requires(true)]
    #[ensures(true)]
    pub fn visit_words(&self, visitor: &mut impl FnMut(&Token)) {
        match self.as_data() {
            data!(DescriptionTailElementSyntax::DescriptionTailSumti(sumti)) => {
                sumti.visit_words(visitor)
            }
            data!(
                DescriptionTailElementSyntax::DescriptionTailRelativeClauses(relative_clauses)
            ) => {
                for relative_clause in relative_clauses {
                    relative_clause.visit_words(visitor);
                }
            }
            data!(DescriptionTailElementSyntax::DescriptionTailQuantifier(
                quantifier
            )) => {
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
            data!(QuantifierSyntax::NumberQuantifier { number, boi }) => {
                number.visit_words(visitor);
                if let Some(boi) = boi {
                    boi.visit_words(visitor);
                }
            }
            data!(QuantifierSyntax::MeksoQuantifier { vei, mekso, veho }) => {
                vei.visit_words(visitor);
                mekso.visit_words(visitor);
                if let Some(veho) = veho {
                    veho.visit_words(visitor);
                }
            }
        }
    }
}

impl MeksoOperatorSyntax {
    #[requires(true)]
    #[ensures(true)]
    pub fn visit_words(&self, visitor: &mut impl FnMut(&Token)) {
        match self.as_data() {
            data!(MeksoOperatorSyntax::Primitive(vuhu)) => vuhu.visit_words(visitor),
            data!(MeksoOperatorSyntax::OperandAsOperator { maho, mekso, tehu }) => {
                maho.visit_words(visitor);
                mekso.visit_words(visitor);
                if let Some(tehu) = tehu {
                    tehu.visit_words(visitor);
                }
            }
            data!(MeksoOperatorSyntax::Converted { se, inner_operator }) => {
                se.visit_words(visitor);
                inner_operator.visit_words(visitor);
            }
            data!(MeksoOperatorSyntax::ScalarNegated {
                nahe,
                inner_operator,
            }) => {
                nahe.visit_words(visitor);
                inner_operator.visit_words(visitor);
            }
            data!(MeksoOperatorSyntax::SelbriAsOperator { nahu, selbri, tehu }) => {
                nahu.visit_words(visitor);
                selbri.visit_words(visitor);
                if let Some(tehu) = tehu {
                    tehu.visit_words(visitor);
                }
            }
            data!(MeksoOperatorSyntax::GroupedOperator {
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
            data!(MeksoOperatorSyntax::BoundOperatorConnection {
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
            data!(MeksoOperatorSyntax::OperatorConnection {
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

impl SelbriSyntax {
    #[requires(true)]
    #[ensures(true)]
    pub fn visit_words(&self, visitor: &mut impl FnMut(&Token)) {
        match self.as_data() {
            data!(SelbriSyntax::SelbriConnection {
                connective,
                leading_selbri,
                trailing_selbri,
            }) => {
                leading_selbri.visit_words(visitor);
                connective.visit_words(visitor);
                trailing_selbri.visit_words(visitor);
            }
            data!(SelbriSyntax::InvertedTanru {
                leading_selbri,
                co,
                trailing_selbri,
            }) => {
                leading_selbri.visit_words(visitor);
                co.visit_words(visitor);
                trailing_selbri.visit_words(visitor);
            }
            data!(SelbriSyntax::BoundSelbriConnection {
                leading_selbri,
                bo_connective,
                bo_tense_modal,
                bo,
                trailing_selbri,
            }) => {
                leading_selbri.visit_words(visitor);
                if let Some(connective) = bo_connective {
                    connective.visit_words(visitor);
                }
                if let Some(tense_modal) = bo_tense_modal {
                    tense_modal.visit_words(visitor);
                }
                bo.visit_words(visitor);
                trailing_selbri.visit_words(visitor);
            }
            data!(SelbriSyntax::Negated { na, inner_selbri }) => {
                na.visit_words(visitor);
                inner_selbri.visit_words(visitor);
            }
            data!(SelbriSyntax::SelbriWord(word)) => visitor(word),
            data!(SelbriSyntax::ConvertedSelbri { se, inner_selbri }) => {
                se.visit_words(visitor);
                inner_selbri.visit_words(visitor);
            }
            data!(SelbriSyntax::GroupedSelbri {
                ke,
                selbri,
                kehe,
                ..
            }) => {
                ke.visit_words(visitor);
                selbri.visit_words(visitor);
                if let Some(kehe) = kehe {
                    kehe.visit_words(visitor);
                }
            }
            data!(SelbriSyntax::TaggedSelbri {
                tense_modal,
                inner_selbri,
            }) => {
                tense_modal.visit_words(visitor);
                inner_selbri.visit_words(visitor);
            }
            data!(SelbriSyntax::ForethoughtSelbriConnection {
                guhek,
                leading_bridi,
                gik,
                trailing_bridi,
                gihi,
            }) => {
                guhek.visit_words(visitor);
                leading_bridi.visit_words(visitor);
                gik.visit_words(visitor);
                trailing_bridi.visit_words(visitor);
                if let Some(gihi) = gihi {
                    visitor(gihi);
                }
            }
            data!(SelbriSyntax::Abstraction(abstraction)) => abstraction.visit_words(visitor),
            data!(SelbriSyntax::Tanru(units)) => {
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

impl TanruUnitSyntax {
    #[requires(true)]
    #[ensures(true)]
    pub fn visit_words(&self, visitor: &mut impl FnMut(&Token)) {
        match self.as_data() {
            data!(TanruUnitSyntax::TanruUnitWord(word)) => word.visit_words(visitor),
            data!(TanruUnitSyntax::ProBridi { goha, raho }) => {
                goha.visit_words(visitor);
                if let Some(raho) = raho {
                    raho.visit_words(visitor);
                }
            }
            data!(TanruUnitSyntax::ConvertedTanruUnit { se, inner_unit }) => {
                se.visit_words(visitor);
                inner_unit.visit_words(visitor);
            }
            data!(TanruUnitSyntax::GroupedTanruUnit {
                ke,
                selbri,
                kehe,
                ..
            }) => {
                ke.visit_words(visitor);
                selbri.visit_words(visitor);
                if let Some(kehe) = kehe {
                    kehe.visit_words(visitor);
                }
            }
            data!(TanruUnitSyntax::ScalarNegatedTanruUnit { nahe, inner_unit }) => {
                nahe.visit_words(visitor);
                inner_unit.visit_words(visitor);
            }
            data!(TanruUnitSyntax::BoundTanruUnitConnection {
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
            data!(TanruUnitSyntax::TanruUnitConnection {
                leading_unit,
                connective,
                trailing_unit,
            }) => {
                leading_unit.visit_words(visitor);
                connective.visit_words(visitor);
                trailing_unit.visit_words(visitor);
            }
            data!(TanruUnitSyntax::RelativeClauses {
                base,
                selbri_relative_clauses,
            }) => {
                base.visit_words(visitor);
                for selbri_relative_clause in selbri_relative_clauses {
                    selbri_relative_clause.visit_words(visitor);
                }
            }
            data!(TanruUnitSyntax::SelbriGroupTanruUnit(selbri)) => selbri.visit_words(visitor),
            data!(TanruUnitSyntax::ModalConversion {
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
            data!(TanruUnitSyntax::LinkedSumtiTanruUnit {
                base,
                be,
                fa,
                first_sumti,
                bei_links,
                beho,
            }) => {
                base.visit_words(visitor);
                be.visit_words(visitor);
                if let Some(fa) = fa {
                    fa.visit_words(visitor);
                }
                if let Some(first_sumti) = first_sumti {
                    first_sumti.visit_words(visitor);
                }
                for bei_link in bei_links {
                    bei_link.visit_words(visitor);
                }
                if let Some(beho) = beho {
                    beho.visit_words(visitor);
                }
            }
            data!(TanruUnitSyntax::PreposedLinkedSumtiTanruUnit {
                be,
                fa,
                first_sumti,
                bei_links,
                beho,
                base,
            }) => {
                be.visit_words(visitor);
                if let Some(fa) = fa {
                    fa.visit_words(visitor);
                }
                if let Some(first_sumti) = first_sumti {
                    first_sumti.visit_words(visitor);
                }
                for bei_link in bei_links {
                    bei_link.visit_words(visitor);
                }
                if let Some(beho) = beho {
                    beho.visit_words(visitor);
                }
                base.visit_words(visitor);
            }
            data!(TanruUnitSyntax::Abstraction(abstraction)) => abstraction.visit_words(visitor),
            data!(TanruUnitSyntax::SumtiSelbri {
                me,
                sumti,
                mehu,
                moi_marker,
            }) => {
                me.visit_words(visitor);
                sumti.visit_words(visitor);
                if let Some(mehu) = mehu {
                    mehu.visit_words(visitor);
                }
                if let Some(moi_marker) = moi_marker {
                    moi_marker.visit_words(visitor);
                }
            }
            data!(TanruUnitSyntax::QuotedWordSelbri(mehoi)) => mehoi.visit_words(visitor),
            data!(TanruUnitSyntax::QuotedBridiSelbri(gohoi)) => gohoi.visit_words(visitor),
            data!(TanruUnitSyntax::QuotedTextSelbri(muhoi)) => muhoi.visit_words(visitor),
            data!(TanruUnitSyntax::TextSelbri { luhei, text, liau }) => {
                luhei.visit_words(visitor);
                text.visit_words(visitor);
                if let Some(liau) = liau {
                    liau.visit_words(visitor);
                }
            }
            data!(TanruUnitSyntax::OrdinalSelbri { number, moi }) => {
                visit_word_slice(number, visitor);
                moi.visit_words(visitor);
            }
            data!(TanruUnitSyntax::OperatorSelbri {
                nuha,
                mekso_operator,
            }) => {
                nuha.visit_words(visitor);
                mekso_operator.visit_words(visitor);
            }
            data!(TanruUnitSyntax::TagSelbri { xohi, tag }) => {
                xohi.visit_words(visitor);
                tag.visit_words(visitor);
            }
            data!(TanruUnitSyntax::AssignedProBridi { base, assignments }) => {
                base.visit_words(visitor);
                for assignment in assignments {
                    assignment.cei.visit_words(visitor);
                    assignment.tanru_unit.visit_words(visitor);
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
        for abstractor_connections in &self.abstractor_connections {
            abstractor_connections.visit_words(visitor);
        }
        self.subbridi.visit_words(visitor);
        if let Some(kei) = &self.kei {
            kei.visit_words(visitor);
        }
    }
}

impl AbstractorConnectionSyntax {
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
            data!(CompositeTenseModalPartSyntax::Cmavo(word)) => out.push(word),
            data!(CompositeTenseModalPartSyntax::AdHocModal(fiho)) => {
                let data!(AdHocModalSyntax {
                    nahe: _,
                    fiho,
                    selbri,
                    fehu,
                }) = fiho.into_data();
                out.push(fiho.value);
                out.extend(selbri.words());
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
            data!(CompositeTenseModalPartSyntax::Cmavo(word)) => visitor(word),
            data!(CompositeTenseModalPartSyntax::AdHocModal(fiho)) => {
                fiho.fiho.visit_words(visitor);
                fiho.selbri.visit_words(visitor);
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
            data!(CompositeTenseModalPartSyntax::Cmavo(word)) => vec![word],
            data!(CompositeTenseModalPartSyntax::AdHocModal(fiho)) => {
                let data!(AdHocModalSyntax {
                    nahe: _,
                    fiho,
                    selbri,
                    fehu,
                }) = fiho.into_data();
                let mut words = vec![fiho.value];
                words.extend(selbri.words());
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
            data!(TenseModalSyntax::TimeDirection(word))
            | data!(TenseModalSyntax::TimeInterval(word))
            | data!(TenseModalSyntax::SpaceDistance(word))
            | data!(TenseModalSyntax::SpaceDirection(word))
            | data!(TenseModalSyntax::Actuality(word)) => word.visit_words(visitor),
            data!(TenseModalSyntax::TimeDirectionDistance { pu, distance }) => {
                visitor(pu);
                distance.visit_words(visitor);
            }
            data!(TenseModalSyntax::TimeDirectionActuality { pu, caha }) => {
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
            data!(TenseModalSyntax::Modal {
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
            data!(TenseModalSyntax::Sticky(ki)) => ki.visit_words(visitor),
            data!(TenseModalSyntax::AdHocModal { fiho, selbri, fehu }) => {
                fiho.visit_words(visitor);
                selbri.visit_words(visitor);
                if let Some(fehu) = fehu {
                    fehu.visit_words(visitor);
                }
            }
            data!(TenseModalSyntax::EventContour(words)) => words.visit_words(visitor),
            data!(TenseModalSyntax::IntervalProperty {
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

impl SubbridiSyntax {
    #[requires(true)]
    #[ensures(true)]
    pub fn words(self) -> Vec<Token> {
        match self.into_data() {
            data!(SubbridiSyntax::Bridi(bridi)) => bridi.words(),
            data!(SubbridiSyntax::Prenex {
                prenex_terms,
                zohu,
                inner_subbridi,
            }) => {
                let mut words = prenex_terms
                    .into_iter()
                    .flat_map(TermSyntax::words)
                    .collect::<Vec<_>>();
                words.extend(zohu.words());
                words.extend(inner_subbridi.words());
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
            data!(FragmentSyntax::Ek(connective))
            | data!(FragmentSyntax::BridiTailConnective(connective)) => connective.words(),
            data!(FragmentSyntax::Other(words)) => words.words(),
            data!(FragmentSyntax::BridiConnective { i, connective }) => {
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
            data!(FragmentSyntax::LinkedSumti {
                be,
                fa,
                first_sumti,
                bei_links,
                beho,
            }) => {
                let mut words = be.words();
                if let Some(fa) = fa {
                    words.extend(fa.words());
                }
                if let Some(first_sumti) = first_sumti {
                    words.extend(first_sumti.words());
                }
                words.extend(
                    bei_links
                        .into_iter()
                        .flat_map(AdditionalLinkedSumtiSyntax::words),
                );
                if let Some(beho) = beho {
                    words.extend(beho.words());
                }
                words
            }
            data!(FragmentSyntax::LinkedSumtiContinuation(bei_only_links)) => bei_only_links
                .into_iter()
                .flat_map(AdditionalLinkedSumtiSyntax::words)
                .collect(),
            data!(FragmentSyntax::RelativeClauses(relative_clauses)) => relative_clauses
                .into_iter()
                .flat_map(RelativeClauseSyntax::words)
                .collect(),
            data!(FragmentSyntax::Mekso(mekso)) => mekso.words(),
            data!(FragmentSyntax::Terms { terms, vau }) => {
                let mut words = Vec::new();
                for term in terms {
                    words.extend(term.words());
                }
                if let Some(vau) = vau {
                    words.extend(vau.words());
                }
                words
            }
            data!(FragmentSyntax::Selbri(selbri)) => selbri.words(),
        }
    }
}

impl TermSyntax {
    #[requires(true)]
    #[ensures(true)]
    pub fn words(self) -> Vec<Token> {
        match self.into_data() {
            data!(TermSyntax::Termset {
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
            data!(TermSyntax::ForethoughtTermsetConnection {
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
            data!(TermSyntax::TermsetGroup {
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
            data!(TermSyntax::TermsetConnection {
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
            data!(TermSyntax::Sumti(sumti)) => sumti.words(),
            data!(TermSyntax::PlaceTaggedSumti { fa, sumti, ku }) => {
                let mut words = fa.words();
                words.extend(sumti.words());
                if let Some(ku) = ku {
                    words.extend(ku.words());
                }
                words
            }
            data!(TermSyntax::BridiNegation { na, na_ku }) => {
                let mut words = vec![na];
                words.extend(na_ku.words());
                words
            }
            data!(TermSyntax::BareNegation(na)) => na.words(),
            data!(TermSyntax::RelativeAdverbialTerm {
                noiha,
                tail_elements,
                selbri,
                relative_clauses,
                fehu,
            }) => {
                let mut words = noiha.words();
                for tail_element in tail_elements {
                    words.extend(tail_element.words());
                }
                if let Some(selbri) = selbri {
                    words.extend(selbri.words());
                }
                for relative_clause in relative_clauses {
                    words.extend(relative_clause.words());
                }
                if let Some(fehu) = fehu {
                    words.extend(fehu.words());
                }
                words
            }
            data!(TermSyntax::BridiVariableAdverbialTerm {
                poiha,
                tail_elements,
                selbri,
                relative_clauses,
                brigahi_ku,
            }) => {
                let mut words = poiha.words();
                for tail_element in tail_elements {
                    words.extend(tail_element.words());
                }
                if let Some(selbri) = selbri {
                    words.extend(selbri.words());
                }
                for relative_clause in relative_clauses {
                    words.extend(relative_clause.words());
                }
                words.extend(brigahi_ku.words());
                words
            }
            data!(TermSyntax::AdHocBridiAdverbialTerm {
                fihoi,
                subbridi,
                fihau,
            }) => {
                let mut words = fihoi.words();
                words.extend((*subbridi).words());
                if let Some(fihau) = fihau {
                    words.extend(fihau.words());
                }
                words
            }
            data!(TermSyntax::ReciprocalBridiAdverbialTerm {
                soi,
                subbridi,
                sehu,
            }) => {
                let mut words = soi.words();
                words.extend((*subbridi).words());
                if let Some(sehu) = sehu {
                    words.extend(sehu.words());
                }
                words
            }
            data!(TermSyntax::JaiTaggedSumti { jai, tag, sumti }) => {
                let mut words = jai.words();
                if let Some(tag) = tag {
                    words.extend(tag.words());
                }
                words.extend(sumti.words());
                words
            }
            data!(TermSyntax::TaggedSumti { tense_modal, sumti }) => {
                let mut words = tense_modal
                    .into_iter()
                    .flat_map(|tense_modal| tense_modal.words())
                    .collect::<Vec<_>>();
                words.extend(sumti.words());
                words
            }
            data!(TermSyntax::TermConnection {
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
            data!(TermSyntax::BoundTermConnection {
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

impl SumtiTagSyntax {
    #[requires(true)]
    #[ensures(true)]
    pub fn words(self) -> Vec<Token> {
        match self.into_data() {
            data!(SumtiTagSyntax::TenseModal(tense_modal)) => tense_modal.words(),
            data!(SumtiTagSyntax::PlaceTag(fa)) => fa.words(),
        }
    }
}

impl MeksoSyntax {
    #[requires(true)]
    #[ensures(true)]
    pub fn words(self) -> Vec<Token> {
        match self.into_data() {
            data!(MeksoSyntax::NumberMekso(quantifier)) => quantifier.words(),
            data!(MeksoSyntax::LerfuStringMekso { letter, boi }) => {
                let mut words = letter.words();
                if let Some(boi) = boi {
                    words.extend(boi.words());
                }
                words
            }
            data!(MeksoSyntax::ParenthesizedMekso {
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
            data!(MeksoSyntax::ForethoughtMeksoConnection {
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
            data!(MeksoSyntax::ForethoughtCall {
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
            data!(MeksoSyntax::ReversePolish {
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
            data!(MeksoSyntax::SelbriOperand { nihe, selbri, tehu }) => {
                let mut words = nihe.words();
                words.extend(selbri.words());
                if let Some(tehu) = tehu {
                    words.extend(tehu.words());
                }
                words
            }
            data!(MeksoSyntax::SumtiOperand { mohe, sumti, tehu }) => {
                let mut words = mohe.words();
                words.extend(sumti.words());
                if let Some(tehu) = tehu {
                    words.extend(tehu.words());
                }
                words
            }
            data!(MeksoSyntax::MeksoArray {
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
            data!(MeksoSyntax::QualifiedOperand {
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
            data!(MeksoSyntax::MeksoConnection {
                left_expression,
                connective,
                right_expression,
            }) => {
                let mut words = left_expression.words();
                words.extend(connective.words());
                words.extend(right_expression.words());
                words
            }
            data!(MeksoSyntax::Infix {
                operator,
                left_expression,
                right_expression,
            }) => {
                let mut words = left_expression.words();
                words.extend(operator.words());
                words.extend(right_expression.words());
                words
            }
            data!(MeksoSyntax::PrecedenceInfix {
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

impl SumtiSyntax {
    #[requires(true)]
    #[ensures(true)]
    pub fn words(self) -> Vec<Token> {
        match self.into_data() {
            data!(SumtiSyntax::QuotedSumti(quote)) => quote.words(),
            data!(SumtiSyntax::NumberSumti {
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
            data!(SumtiSyntax::LerfuStringSumti { letter, boi }) => {
                let mut words = letter.words();
                if let Some(boi) = boi {
                    words.extend(boi.words());
                }
                words
            }
            data!(SumtiSyntax::QuantifiedSumti {
                quantifier,
                inner_sumti,
            }) => {
                let mut words = quantifier.words();
                words.extend(inner_sumti.words());
                words
            }
            data!(SumtiSyntax::SumtiWithRelativeClauses {
                base_sumti,
                vuho,
                relative_clauses,
            }) => {
                let mut words = base_sumti.words();
                if let Some(vuho) = vuho {
                    words.extend(vuho.words());
                }
                for relative_clause in relative_clauses {
                    words.extend(relative_clause.words());
                }
                words
            }
            data!(SumtiSyntax::SumtiWithComplexRelativeClauses {
                base_sumti,
                vuho_marker,
                relative_clauses,
                sumti_connection,
            }) => {
                let mut words = base_sumti.words();
                words.extend(vuho_marker.words());
                for relative_clause in relative_clauses {
                    words.extend(relative_clause.words());
                }
                if let Some(sumti_connection) = sumti_connection {
                    words.extend(sumti_connection.connective.words());
                    words.extend(sumti_connection.sumti.words());
                }
                words
            }
            data!(SumtiSyntax::BridiDescription {
                lohoi,
                subbridi,
                kuhau,
            }) => {
                let mut words = lohoi.words();
                words.extend(subbridi.words());
                if let Some(kuhau) = kuhau {
                    words.extend(kuhau.words());
                }
                words
            }
            data!(SumtiSyntax::NegatedSumti { na, ku }) => {
                let mut words = vec![na];
                words.extend(ku.words());
                words
            }
            data!(SumtiSyntax::TaggedSumti { tag, inner_sumti }) => {
                let mut words = tag.words();
                words.extend(inner_sumti.words());
                words
            }
            data!(SumtiSyntax::ScalarNegatedSumtiWithBo {
                nahe,
                bo,
                inner_sumti,
                luhu,
            }) => {
                let mut words = vec![nahe];
                words.extend(bo.words());
                words.extend(inner_sumti.words());
                if let Some(luhu) = luhu {
                    words.extend(luhu.words());
                }
                words
            }
            data!(SumtiSyntax::ScalarNegatedSumti {
                nahe,
                inner_sumti,
                luhu,
            }) => {
                let mut words = nahe.words();
                words.extend(inner_sumti.words());
                if let Some(luhu) = luhu {
                    words.extend(luhu.words());
                }
                words
            }
            data!(SumtiSyntax::QualifiedTerm {
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
            data!(SumtiSyntax::ProSumti(koha)) => koha.words(),
            data!(SumtiSyntax::ElidedSumti {
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
            data!(SumtiSyntax::ReferentSumti {
                lahe,
                relative_clauses,
                inner_sumti,
                luhu,
            }) => {
                let mut words = lahe.words();
                for relative_clause in relative_clauses {
                    words.extend(relative_clause.words());
                }
                words.extend(inner_sumti.words());
                if let Some(luhu) = luhu {
                    words.extend(luhu.words());
                }
                words
            }
            data!(SumtiSyntax::SumtiConnection {
                leading_sumti,
                connective,
                trailing_sumti,
            }) => {
                let mut words = leading_sumti.words();
                words.extend(connective.words());
                words.extend(trailing_sumti.words());
                words
            }
            data!(SumtiSyntax::GroupedSumti {
                ke,
                inner_sumti,
                kehe,
            }) => {
                let mut words = ke.words();
                words.extend(inner_sumti.words());
                if let Some(kehe) = kehe {
                    words.extend(kehe.words());
                }
                words
            }
            data!(SumtiSyntax::BoundSumtiConnection {
                leading_sumti,
                bo_connective,
                bo_tense_modal,
                bo,
                trailing_sumti,
            }) => {
                let mut words = leading_sumti.words();
                if let Some(connective) = bo_connective {
                    words.extend(connective.words());
                }
                if let Some(tense_modal) = bo_tense_modal {
                    words.extend(tense_modal.words());
                }
                words.extend(bo.words());
                words.extend(trailing_sumti.words());
                words
            }
            data!(SumtiSyntax::ForethoughtSumtiConnection {
                gek,
                leading_sumti,
                gik,
                trailing_sumti,
                gihi,
            }) => {
                let mut words = gek.words();
                words.extend(leading_sumti.words());
                words.extend(gik.words());
                words.extend(trailing_sumti.words());
                if let Some(gihi) = gihi {
                    words.push(gihi);
                }
                words
            }
            data!(SumtiSyntax::Description(description)) => description.words(),
            data!(SumtiSyntax::DescriptionConnection(description_connection)) => {
                description_connection.words()
            }
            data!(SumtiSyntax::NameDescription { la, names }) => {
                let mut words = la.words();
                words.extend(names.words());
                words
            }
            data!(SumtiSyntax::NameWords(cmevla)) => cmevla.words(),
            data!(SumtiSyntax::SelbriVocative {
                leading_relative_clauses,
                selbri,
                trailing_relative_clauses,
            }) => {
                let mut words = leading_relative_clauses
                    .into_iter()
                    .flat_map(RelativeClauseSyntax::words)
                    .collect::<Vec<_>>();
                words.extend(selbri.words());
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

impl SumtiAssociationPhraseSyntax {
    #[requires(true)]
    #[ensures(true)]
    pub fn words(self) -> Vec<Token> {
        let data!(SumtiAssociationPhraseSyntax {
            association_marker,
            sumti,
            gehu,
        }) = self.into_data();
        let mut words = association_marker.words();
        words.extend(sumti.words());
        if let Some(gehu) = gehu {
            words.extend(gehu.words());
        }
        words
    }
}

impl SelbriRelativePhraseSyntax {
    #[requires(true)]
    #[ensures(true)]
    pub fn words(self) -> Vec<Token> {
        let data!(SelbriRelativePhraseSyntax {
            nohoi,
            selbri,
            kuhoi,
        }) = self.into_data();
        let mut words = nohoi.words();
        words.extend(selbri.words());
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
            data!(RelativeClauseSyntax::SumtiAssociationPhrase(
                relative_clause
            )) => relative_clause.words(),
            data!(RelativeClauseSyntax::IncidentalRelativeBridi {
                noi,
                subbridi,
                kuho,
            }) => {
                let mut words = noi.words();
                words.extend(subbridi.words());
                if let Some(kuho) = kuho {
                    words.extend(kuho.words());
                }
                words
            }
            data!(RelativeClauseSyntax::RestrictiveRelativeBridi {
                poi,
                subbridi,
                kuho,
            }) => {
                let mut words = poi.words();
                words.extend(subbridi.words());
                if let Some(kuho) = kuho {
                    words.extend(kuho.words());
                }
                words
            }
            data!(RelativeClauseSyntax::JoinedRelativeClauses { zihe, inner }) => {
                let mut words = zihe.words();
                words.extend(inner.words());
                words
            }
            data!(RelativeClauseSyntax::RelativeClauseConnection { connective, inner }) => {
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
            data!(QuoteSyntax::TextQuote { lu, text, lihu }) => {
                let mut words = lu.words();
                words.extend(text.words());
                if let Some(lihu) = lihu {
                    words.extend(lihu.words());
                }
                words
            }
            data!(QuoteSyntax::WordQuote(zo)) | data!(QuoteSyntax::DelimitedNonLojbanQuote(zo)) => {
                zo.words()
            }
            data!(QuoteSyntax::DelimitedWordQuote(zohoi)) => zohoi.words(),
            data!(QuoteSyntax::WordsQuote(lohu)) => lohu.words(),
        }
    }
}

impl DescriptionSyntax {
    #[requires(true)]
    #[ensures(true)]
    pub fn words(self) -> Vec<Token> {
        let data!(DescriptionSyntax {
            outer_quantifier,
            description,
            tail_elements,
            selbri,
            relative_clauses,
            ku,
        }) = self.into_data();
        let mut words = outer_quantifier
            .into_iter()
            .flat_map(|quantifier| quantifier.words())
            .collect::<Vec<_>>();
        if let Some(description) = description {
            words.extend(description.words());
        }
        for element in tail_elements {
            words.extend(element.words());
        }
        if let Some(selbri) = selbri {
            words.extend(selbri.words());
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

impl DescriptionHeadSyntax {
    #[requires(true)]
    #[ensures(true)]
    pub fn words(self) -> Vec<Token> {
        let data!(DescriptionHeadSyntax { description }) = self.into_data();
        description.words()
    }
}

impl DescriptionConnectionSyntax {
    #[requires(true)]
    #[ensures(true)]
    pub fn words(self) -> Vec<Token> {
        let data!(DescriptionConnectionSyntax {
            leading_description_head,
            connective,
            trailing_description_head,
            tail_elements,
            selbri,
            relative_clauses,
            ku,
        }) = self.into_data();
        let mut words = leading_description_head.words();
        words.extend(connective.words());
        words.extend(trailing_description_head.words());
        for element in tail_elements {
            words.extend(element.words());
        }
        if let Some(selbri) = selbri {
            words.extend(selbri.words());
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

impl AdditionalLinkedSumtiSyntax {
    #[requires(true)]
    #[ensures(true)]
    pub fn words(self) -> Vec<Token> {
        let data!(AdditionalLinkedSumtiSyntax { bei, fa, sumti }) = self.into_data();
        let mut words = bei.words();
        if let Some(fa) = fa {
            words.extend(fa.words());
        }
        if let Some(sumti) = sumti {
            words.extend(sumti.words());
        }
        words
    }
}

impl DescriptionTailElementSyntax {
    #[requires(true)]
    #[ensures(true)]
    pub fn words(self) -> Vec<Token> {
        match self.into_data() {
            data!(DescriptionTailElementSyntax::DescriptionTailSumti(sumti)) => sumti.words(),
            data!(
                DescriptionTailElementSyntax::DescriptionTailRelativeClauses(relative_clauses)
            ) => relative_clauses
                .into_iter()
                .flat_map(RelativeClauseSyntax::words)
                .collect(),
            data!(DescriptionTailElementSyntax::DescriptionTailQuantifier(
                quantifier
            )) => quantifier.words(),
        }
    }
}

impl QuantifierSyntax {
    #[requires(true)]
    #[ensures(true)]
    pub fn words(self) -> Vec<Token> {
        match self.into_data() {
            data!(QuantifierSyntax::NumberQuantifier { number, boi }) => {
                let mut words = number.words();
                if let Some(boi) = boi {
                    words.extend(boi.words());
                }
                words
            }
            data!(QuantifierSyntax::MeksoQuantifier { vei, mekso, veho }) => {
                let mut words = vei.words();
                words.extend(mekso.words());
                if let Some(veho) = veho {
                    words.extend(veho.words());
                }
                words
            }
        }
    }
}

impl MeksoOperatorSyntax {
    #[requires(true)]
    #[ensures(true)]
    pub fn words(self) -> Vec<Token> {
        match self.into_data() {
            data!(MeksoOperatorSyntax::Primitive(vuhu)) => vuhu.words(),
            data!(MeksoOperatorSyntax::OperandAsOperator { maho, mekso, tehu }) => {
                let mut words = maho.words();
                words.extend(mekso.words());
                if let Some(tehu) = tehu {
                    words.extend(tehu.words());
                }
                words
            }
            data!(MeksoOperatorSyntax::Converted { se, inner_operator }) => {
                let mut words = se.words();
                words.extend(inner_operator.words());
                words
            }
            data!(MeksoOperatorSyntax::ScalarNegated {
                nahe,
                inner_operator,
            }) => {
                let mut words = nahe.words();
                words.extend(inner_operator.words());
                words
            }
            data!(MeksoOperatorSyntax::SelbriAsOperator { nahu, selbri, tehu }) => {
                let mut words = nahu.words();
                words.extend(selbri.words());
                if let Some(tehu) = tehu {
                    words.extend(tehu.words());
                }
                words
            }
            data!(MeksoOperatorSyntax::GroupedOperator {
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
            data!(MeksoOperatorSyntax::BoundOperatorConnection {
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
            data!(MeksoOperatorSyntax::OperatorConnection {
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

impl SelbriSyntax {
    #[requires(true)]
    #[ensures(true)]
    pub fn words(self) -> Vec<Token> {
        match self.into_data() {
            data!(SelbriSyntax::SelbriConnection {
                connective,
                leading_selbri,
                trailing_selbri,
            }) => {
                let mut words = leading_selbri.words();
                words.extend(connective.words());
                words.extend(trailing_selbri.words());
                words
            }
            data!(SelbriSyntax::InvertedTanru {
                leading_selbri,
                co,
                trailing_selbri,
            }) => {
                let mut words = leading_selbri.words();
                words.extend(co.words());
                words.extend(trailing_selbri.words());
                words
            }
            data!(SelbriSyntax::BoundSelbriConnection {
                leading_selbri,
                bo_connective,
                bo_tense_modal,
                bo,
                trailing_selbri,
            }) => {
                let mut words = leading_selbri.words();
                if let Some(connective) = bo_connective {
                    words.extend(connective.words());
                }
                if let Some(tense_modal) = bo_tense_modal {
                    words.extend(tense_modal.words());
                }
                words.extend(bo.words());
                words.extend(trailing_selbri.words());
                words
            }
            data!(SelbriSyntax::Negated { na, inner_selbri }) => {
                let mut words = na.words();
                words.extend(inner_selbri.words());
                words
            }
            data!(SelbriSyntax::SelbriWord(word)) => vec![word],
            data!(SelbriSyntax::ConvertedSelbri { se, inner_selbri }) => {
                let mut words = se.words();
                words.extend(inner_selbri.words());
                words
            }
            data!(SelbriSyntax::GroupedSelbri {
                ke,
                selbri,
                kehe,
                ..
            }) => {
                let mut words = ke.words();
                words.extend(selbri.words());
                if let Some(kehe) = kehe {
                    words.extend(kehe.words());
                }
                words
            }
            data!(SelbriSyntax::TaggedSelbri {
                tense_modal,
                inner_selbri,
            }) => {
                let mut words = tense_modal.words();
                words.extend(inner_selbri.words());
                words
            }
            data!(SelbriSyntax::ForethoughtSelbriConnection {
                guhek,
                leading_bridi,
                gik,
                trailing_bridi,
                gihi,
            }) => {
                let mut words = guhek.words();
                words.extend(leading_bridi.words());
                words.extend(gik.words());
                words.extend(trailing_bridi.words());
                if let Some(gihi) = gihi {
                    words.push(gihi);
                }
                words
            }
            data!(SelbriSyntax::Abstraction(abstraction)) => abstraction.words(),
            data!(SelbriSyntax::Tanru(units)) => (*units)
                .into_vec()
                .into_iter()
                .flat_map(TanruUnitSyntax::words)
                .collect(),
        }
    }
}

impl TanruUnitSyntax {
    #[requires(true)]
    #[ensures(true)]
    pub fn words(self) -> Vec<Token> {
        match self.into_data() {
            data!(TanruUnitSyntax::TanruUnitWord(word)) => word.words(),
            data!(TanruUnitSyntax::ProBridi { goha, raho }) => {
                let mut words = goha.words();
                if let Some(raho) = raho {
                    words.extend(raho.words());
                }
                words
            }
            data!(TanruUnitSyntax::ConvertedTanruUnit { se, inner_unit }) => {
                let mut words = se.words();
                words.extend(inner_unit.words());
                words
            }
            data!(TanruUnitSyntax::GroupedTanruUnit {
                ke,
                selbri,
                kehe,
                ..
            }) => {
                let mut words = ke.words();
                words.extend(selbri.words());
                if let Some(kehe) = kehe {
                    words.extend(kehe.words());
                }
                words
            }
            data!(TanruUnitSyntax::ScalarNegatedTanruUnit { nahe, inner_unit }) => {
                let mut words = nahe.words();
                words.extend(inner_unit.words());
                words
            }
            data!(TanruUnitSyntax::BoundTanruUnitConnection {
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
            data!(TanruUnitSyntax::TanruUnitConnection {
                leading_unit,
                connective,
                trailing_unit,
            }) => {
                let mut words = leading_unit.words();
                words.extend(connective.words());
                words.extend(trailing_unit.words());
                words
            }
            data!(TanruUnitSyntax::RelativeClauses {
                base,
                selbri_relative_clauses,
            }) => {
                let mut words = base.words();
                for selbri_relative_clause in selbri_relative_clauses {
                    words.extend(selbri_relative_clause.words());
                }
                words
            }
            data!(TanruUnitSyntax::SelbriGroupTanruUnit(selbri)) => selbri.words(),
            data!(TanruUnitSyntax::ModalConversion {
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
            data!(TanruUnitSyntax::LinkedSumtiTanruUnit {
                base,
                be,
                fa,
                first_sumti,
                bei_links,
                beho,
            }) => {
                let mut words = base.words();
                words.extend(be.words());
                if let Some(fa) = fa {
                    words.extend(fa.words());
                }
                if let Some(first_sumti) = first_sumti {
                    words.extend(first_sumti.words());
                }
                words.extend(
                    bei_links
                        .into_iter()
                        .flat_map(AdditionalLinkedSumtiSyntax::words),
                );
                if let Some(beho) = beho {
                    words.extend(beho.words());
                }
                words
            }
            data!(TanruUnitSyntax::PreposedLinkedSumtiTanruUnit {
                be,
                fa,
                first_sumti,
                bei_links,
                beho,
                base,
            }) => {
                let mut words = be.words();
                if let Some(fa) = fa {
                    words.extend(fa.words());
                }
                if let Some(first_sumti) = first_sumti {
                    words.extend(first_sumti.words());
                }
                words.extend(
                    bei_links
                        .into_iter()
                        .flat_map(AdditionalLinkedSumtiSyntax::words),
                );
                if let Some(beho) = beho {
                    words.extend(beho.words());
                }
                words.extend(base.words());
                words
            }
            data!(TanruUnitSyntax::Abstraction(abstraction)) => abstraction.words(),
            data!(TanruUnitSyntax::SumtiSelbri {
                me,
                sumti,
                mehu,
                moi_marker,
            }) => {
                let mut words = me.words();
                words.extend(sumti.words());
                if let Some(mehu) = mehu {
                    words.extend(mehu.words());
                }
                if let Some(moi_marker) = moi_marker {
                    words.extend(moi_marker.words());
                }
                words
            }
            data!(TanruUnitSyntax::QuotedWordSelbri(mehoi)) => mehoi.words(),
            data!(TanruUnitSyntax::QuotedBridiSelbri(gohoi)) => gohoi.words(),
            data!(TanruUnitSyntax::QuotedTextSelbri(muhoi)) => muhoi.words(),
            data!(TanruUnitSyntax::TextSelbri { luhei, text, liau }) => {
                let mut words = luhei.words();
                words.extend(text.words());
                if let Some(liau) = liau {
                    words.extend(liau.words());
                }
                words
            }
            data!(TanruUnitSyntax::OrdinalSelbri { number, moi }) => {
                let mut words = number.into_vec();
                words.extend(moi.words());
                words
            }
            data!(TanruUnitSyntax::OperatorSelbri {
                nuha,
                mekso_operator,
            }) => {
                let mut words = nuha.words();
                words.extend(mekso_operator.words());
                words
            }
            data!(TanruUnitSyntax::TagSelbri { xohi, tag }) => {
                let mut words = xohi.words();
                words.extend(tag.words());
                words
            }
            data!(TanruUnitSyntax::AssignedProBridi { base, assignments }) => {
                let mut words = base.words();
                for assignment in assignments {
                    let data!(ProBridiAssignmentSyntax { cei, tanru_unit }) =
                        assignment.into_data();
                    words.extend(cei.words());
                    words.extend(tanru_unit.words());
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
            abstractor_connections,
            subbridi,
            kei,
        }) = self.into_data();
        let mut words = nu.words();
        if let Some(nai) = nai {
            words.extend(nai.words());
        }
        for abstractor_connections in abstractor_connections {
            words.extend(abstractor_connections.words());
        }
        words.extend((*subbridi).words());
        if let Some(kei) = kei {
            words.extend(kei.words());
        }
        words
    }
}

impl AbstractorConnectionSyntax {
    #[requires(true)]
    #[ensures(true)]
    pub fn words(self) -> Vec<Token> {
        let data!(AbstractorConnectionSyntax {
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
            data!(TenseModalSyntax::TimeDirection(word))
            | data!(TenseModalSyntax::TimeInterval(word))
            | data!(TenseModalSyntax::SpaceDistance(word))
            | data!(TenseModalSyntax::SpaceDirection(word))
            | data!(TenseModalSyntax::Actuality(word)) => word.free_modifiers.len(),
            data!(TenseModalSyntax::TimeDirectionDistance { distance, .. }) => {
                distance.free_modifiers.len()
            }
            data!(TenseModalSyntax::TimeDirectionActuality { caha, .. }) => {
                caha.free_modifiers.len()
            }
            data!(TenseModalSyntax::SpaceMovement {
                direction,
                distance,
                ..
            }) => distance
                .as_ref()
                .map_or(direction.free_modifiers.len(), |distance| {
                    distance.free_modifiers.len()
                }),
            data!(TenseModalSyntax::Modal {
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
            data!(TenseModalSyntax::Sticky(ki)) => ki.free_modifiers.len(),
            data!(TenseModalSyntax::AdHocModal { fiho, fehu, .. }) => fehu
                .as_ref()
                .map_or(fiho.free_modifiers.len(), |fehu| fehu.free_modifiers.len()),
            data!(TenseModalSyntax::EventContour(words)) => words.free_modifiers.len(),
            data!(TenseModalSyntax::IntervalProperty {
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
            data!(TenseModalSyntax::TimeDirection(word))
            | data!(TenseModalSyntax::Actuality(word)) => (vec![word.value], word.free_modifiers),
            data!(TenseModalSyntax::TimeDirectionDistance { pu, distance }) => {
                (vec![pu, distance.value], distance.free_modifiers)
            }
            data!(TenseModalSyntax::TimeInterval(word)) => (vec![word.value], word.free_modifiers),
            data!(TenseModalSyntax::TimeDirectionActuality { pu, caha }) => {
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
            data!(TenseModalSyntax::Modal {
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
            data!(TenseModalSyntax::Sticky(ki)) => (vec![ki.value], ki.free_modifiers),
            data!(TenseModalSyntax::AdHocModal { fiho, selbri, fehu }) => {
                let mut words = vec![fiho.value];
                let mut free_modifiers = fiho.free_modifiers;
                words.extend((*selbri).words());
                if let Some(fehu) = fehu {
                    words.push(fehu.value);
                    free_modifiers = fehu.free_modifiers;
                }
                (words, free_modifiers)
            }
            data!(TenseModalSyntax::EventContour(words)) => (words.value, words.free_modifiers),
            data!(TenseModalSyntax::IntervalProperty {
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
            data!(StatementSyntax::TextGroup {
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
            data!(StatementSyntax::Bridi(bridi)) => bridi.visit_words(visitor),
            data!(StatementSyntax::StatementConnection {
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
            data!(StatementSyntax::PreposedIStatementConnection {
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
            data!(StatementSyntax::ExperimentalBridiContinuation {
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

impl BridiStatementContinuationSyntax {
    #[requires(true)]
    #[ensures(true)]
    pub fn visit_words(&self, visitor: &mut impl FnMut(&Token)) {
        self.connective.visit_words(visitor);
        if let Some(tense_modal) = &self.tense_modal {
            tense_modal.visit_words(visitor);
        }
        match self.marker.as_data() {
            data!(BridiStatementContinuationMarkerSyntax::BoGrouped(bo)) => {
                bo.visit_words(visitor);
                self.trailing_subbridi.visit_words(visitor);
            }
            data!(BridiStatementContinuationMarkerSyntax::KeGrouped { ke, kehe }) => {
                ke.visit_words(visitor);
                self.trailing_subbridi.visit_words(visitor);
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
            data!(FreeModifierSyntax::MetalinguisticBridi {
                sei,
                terms,
                cu,
                selbri,
                sehu,
            }) => {
                sei.visit_words(visitor);
                for term in terms {
                    term.visit_words(visitor);
                }
                if let Some(cu) = cu {
                    cu.visit_words(visitor);
                }
                selbri.visit_words(visitor);
                if let Some(sehu) = sehu {
                    sehu.visit_words(visitor);
                }
            }
            data!(FreeModifierSyntax::ParentheticalText { to, text, toi }) => {
                to.visit_words(visitor);
                text.visit_words(visitor);
                if let Some(toi) = toi {
                    toi.visit_words(visitor);
                }
            }
            data!(FreeModifierSyntax::Subscript { xi, expression }) => {
                xi.visit_words(visitor);
                expression.visit_words(visitor);
            }
            data!(FreeModifierSyntax::UtteranceOrdinal { number, mai }) => {
                visit_word_slice(number, visitor);
                mai.visit_words(visitor);
            }
            data!(FreeModifierSyntax::ReciprocalSumti {
                soi,
                leading_sumti,
                trailing_sumti,
                sehu,
            }) => {
                soi.visit_words(visitor);
                leading_sumti.visit_words(visitor);
                if let Some(sumti) = trailing_sumti {
                    sumti.visit_words(visitor);
                }
                if let Some(sehu) = sehu {
                    sehu.visit_words(visitor);
                }
            }
            data!(FreeModifierSyntax::Vocative {
                vocative_markers,
                sumti,
                dohu,
            }) => {
                vocative_markers.visit_words(visitor);
                if let Some(sumti) = sumti {
                    sumti.visit_words(visitor);
                }
                if let Some(dohu) = dohu {
                    dohu.visit_words(visitor);
                }
            }
            data!(FreeModifierSyntax::TextReplacement {
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
            data!(FreeModifierSyntax::MetalinguisticBridi { sei, .. }) => sei.first_word(),
            data!(FreeModifierSyntax::ParentheticalText { to, .. }) => to.first_word(),
            data!(FreeModifierSyntax::Subscript { xi, .. }) => xi.first_word(),
            data!(FreeModifierSyntax::UtteranceOrdinal { number, .. }) => Some(number.first()),
            data!(FreeModifierSyntax::ReciprocalSumti { soi, .. }) => soi.first_word(),
            data!(FreeModifierSyntax::Vocative {
                vocative_markers,
                ..
            }) => vocative_markers.first_word(),
            data!(FreeModifierSyntax::TextReplacement {
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

impl BridiSyntax {
    #[requires(true)]
    #[ensures(true)]
    pub fn visit_words(&self, visitor: &mut impl FnMut(&Token)) {
        for term in &self.leading_terms {
            term.visit_words(visitor);
        }
        if let Some(cu) = &self.cu {
            cu.visit_words(visitor);
        }
        self.bridi_tail.visit_words(visitor);
        for free_modifier in &self.free_modifiers {
            free_modifier.visit_words(visitor);
        }
    }
}

impl BridiTailSyntax {
    #[requires(true)]
    #[ensures(true)]
    pub fn visit_words(&self, visitor: &mut impl FnMut(&Token)) {
        self.first.visit_words(visitor);
        if let Some(ke_continuation) = &self.ke_continuation {
            ke_continuation.visit_words(visitor);
        }
    }
}

impl GroupedBridiTailConnectionSyntax {
    #[requires(true)]
    #[ensures(true)]
    pub fn visit_words(&self, visitor: &mut impl FnMut(&Token)) {
        self.connective.visit_words(visitor);
        if let Some(tense_modal) = &self.tense_modal {
            tense_modal.visit_words(visitor);
        }
        self.ke.visit_words(visitor);
        self.bridi_tail.visit_words(visitor);
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

impl AfterthoughtBridiTailSyntax {
    #[requires(true)]
    #[ensures(true)]
    pub fn visit_words(&self, visitor: &mut impl FnMut(&Token)) {
        self.first.visit_words(visitor);
        for continuation in &self.continuations {
            continuation.visit_words(visitor);
        }
    }
}

impl BridiTailConnectionSyntax {
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
        self.bridi_tail.visit_words(visitor);
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

impl BoGroupedBridiTailSyntax {
    #[requires(true)]
    #[ensures(true)]
    pub fn visit_words(&self, visitor: &mut impl FnMut(&Token)) {
        self.first.visit_words(visitor);
        if let Some(bo_continuation) = &self.bo_continuation {
            bo_continuation.visit_words(visitor);
        }
    }
}

impl BoundBridiTailConnectionSyntax {
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
        self.bridi_tail.visit_words(visitor);
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

impl SimpleBridiTailSyntax {
    #[requires(true)]
    #[ensures(true)]
    pub fn visit_words(&self, visitor: &mut impl FnMut(&Token)) {
        match self.as_data() {
            data!(SimpleBridiTailSyntax::SelbriBridiTail {
                selbri,
                terms,
                vau,
                free_modifiers,
            }) => {
                selbri.visit_words(visitor);
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
            data!(SimpleBridiTailSyntax::ForethoughtBridiTailConnection(
                forethought_connection
            )) => forethought_connection.visit_words(visitor),
            data!(SimpleBridiTailSyntax::TermPrefixedBridiTail { terms, bridi_tail }) => {
                for term in terms {
                    term.visit_words(visitor);
                }
                bridi_tail.visit_words(visitor);
            }
        }
    }
}

impl ForethoughtBridiConnectionSyntax {
    #[requires(true)]
    #[ensures(true)]
    pub fn visit_words(&self, visitor: &mut impl FnMut(&Token)) {
        match self.as_data() {
            data!(ForethoughtBridiConnectionSyntax::BridiConnection {
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
            data!(ForethoughtBridiConnectionSyntax::GroupedBridiConnection {
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
            data!(ForethoughtBridiConnectionSyntax::NegatedBridiConnection { na, inner }) => {
                na.visit_words(visitor);
                inner.visit_words(visitor);
            }
        }
    }
}
