#[allow(unused_imports)]
use bityzba::{ensures, requires};
use jbotci_syntax::ExperimentalConstruct;

#[test]
#[requires(true)]
#[ensures(true)]
fn warning_codes_and_messages_remain_stable() {
    let expected = [
        (
            ExperimentalConstruct::ExperimentalCmavo,
            "syntax.warning.experimental-cmavo",
            "experimental cmavo",
        ),
        (
            ExperimentalConstruct::ExperimentalZohOiQuote,
            "syntax.warning.experimental-zoh-oi-quote",
            "ZOhOI single-word foreign quote",
        ),
        (
            ExperimentalConstruct::ExperimentalMehOiSelbriUnit,
            "syntax.warning.experimental-meh-oi-selbri-unit",
            "MEhOI stage-0 fu'ivla selbri unit",
        ),
        (
            ExperimentalConstruct::ExperimentalLohOiBridiDescription,
            "syntax.warning.experimental-loh-oi-bridi-description",
            "LOhOI/KUhAU bridi description sumti",
        ),
        (
            ExperimentalConstruct::ExperimentalJaKeTanruConnective,
            "syntax.warning.experimental-ja-ke-tanru-connective",
            "JA connective before a KE tanru group",
        ),
        (
            ExperimentalConstruct::ExperimentalIntervalSentenceConnective,
            "syntax.warning.experimental-interval-sentence-connective",
            "BIhI interval connective joining bridi tails",
        ),
        (
            ExperimentalConstruct::ExperimentalJaGiForethoughtConnective,
            "syntax.warning.experimental-ja-gi-forethought-connective",
            "JA connective before GI",
        ),
        (
            ExperimentalConstruct::ExperimentalNaJoiConnective,
            "syntax.warning.experimental-na-joi-connective",
            "NA before JOI as one connective",
        ),
        (
            ExperimentalConstruct::ExperimentalMexQuantifier,
            "syntax.warning.experimental-mex-quantifier",
            "mex quantifier without VEI",
        ),
        (
            ExperimentalConstruct::ExperimentalWholeMexQualifier,
            "syntax.warning.experimental-whole-mex-qualifier",
            "qualifier over a whole mex",
        ),
        (
            ExperimentalConstruct::ExperimentalSplitNumberLerfu,
            "syntax.warning.experimental-split-number-lerfu",
            "separate number and lerfu strings",
        ),
        (
            ExperimentalConstruct::ExperimentalMexMeSelbriUnit,
            "syntax.warning.experimental-mex-me-selbri-unit",
            "ME with a mex body",
        ),
        (
            ExperimentalConstruct::ExperimentalMexMoiSelbriUnit,
            "syntax.warning.experimental-mex-moi-selbri-unit",
            "mex before MOI",
        ),
        (
            ExperimentalConstruct::ExperimentalMexSubscript,
            "syntax.warning.experimental-mex-subscript",
            "XI with a mex that is not a number or VEI",
        ),
        (
            ExperimentalConstruct::ExperimentalMexUtteranceOrdinal,
            "syntax.warning.experimental-mex-utterance-ordinal",
            "mex before MAI",
        ),
        (
            ExperimentalConstruct::ExperimentalLohAiReplacementFree,
            "syntax.warning.experimental-loh-ai-replacement-free",
            "LOhAI/LEhAI replacement free modifier",
        ),
        (
            ExperimentalConstruct::ExperimentalJacuPredicateTailConnective,
            "syntax.warning.experimental-jacu-bridi-tail-connective",
            "JA/JOI connective used in a bridi-tail connective slot",
        ),
        (
            ExperimentalConstruct::ExperimentalNaheArgumentWithoutBo,
            "syntax.warning.experimental-nahe-sumti-without-bo",
            "NAhE argument without BO",
        ),
        (
            ExperimentalConstruct::ExperimentalVuhoScopedAttachment,
            "syntax.warning.experimental-vuho-scoped-attachment",
            "VUhO scoped attachment enhancement",
        ),
        (
            ExperimentalConstruct::ExperimentalNohoiSelbriRelativeClause,
            "syntax.warning.experimental-nohoi-selbri-relative-clause",
            "NOhOI/KUhOI selbri relative clause",
        ),
        (
            ExperimentalConstruct::ExperimentalRelativeClauseConnective,
            "syntax.warning.experimental-relative-clause-connective",
            "JA/JOI connective used between relative clauses",
        ),
        (
            ExperimentalConstruct::ExperimentalMexOperatorConnective,
            "syntax.warning.experimental-mex-operator-connective",
            "camxes-exp BO-bound or connective-atom MEX operator",
        ),
        (
            ExperimentalConstruct::ExperimentalCuTermsSelbri,
            "syntax.warning.experimental-cu-terms-selbri",
            "CU followed by terms before the main selbri",
        ),
        (
            ExperimentalConstruct::ExperimentalLaheNaheTermWrapper,
            "syntax.warning.experimental-lahe-nahe-term-wrapper",
            "LAhE/NAhE term wrapper",
        ),
        (
            ExperimentalConstruct::ExperimentalVuhuConnective,
            "syntax.warning.experimental-vuhu-connective",
            "VUhU used as a non-MEX connective",
        ),
        (
            ExperimentalConstruct::ExperimentalFaAsTag,
            "syntax.warning.experimental-fa-as-tag",
            "FA place tag used as a tag/stag atom",
        ),
        (
            ExperimentalConstruct::ExperimentalFlattenedTag,
            "syntax.warning.experimental-flattened-tag",
            "experimental flattened tag form",
        ),
        (
            ExperimentalConstruct::ExperimentalCbmCmevlaSelbriWord,
            "syntax.warning.experimental-cbm-cmevla-selbri-word",
            "CBM cmevla used as a selbri word",
        ),
        (
            ExperimentalConstruct::ExperimentalCbmLaNameAsDescriptor,
            "syntax.warning.experimental-cbm-la-name-as-description",
            "CBM LA name form parsed as a description",
        ),
        (
            ExperimentalConstruct::ExperimentalDictionaryDoiVocative,
            "syntax.warning.experimental-dictionary-doi-vocative",
            "dictionary-first DOI experimental vocative/attribution cmavo",
        ),
        (
            ExperimentalConstruct::ExperimentalDictionaryCoiVocative,
            "syntax.warning.experimental-dictionary-coi-vocative",
            "dictionary-first COI experimental vocative cmavo",
        ),
        (
            ExperimentalConstruct::ExperimentalDictionaryPaNumber,
            "syntax.warning.experimental-dictionary-pa-number",
            "dictionary-first PA experimental number word",
        ),
        (
            ExperimentalConstruct::ExperimentalDictionaryFahaTag,
            "syntax.warning.experimental-dictionary-faha-tag",
            "dictionary-first FAhA experimental spatial tag",
        ),
        (
            ExperimentalConstruct::ExperimentalDictionaryUiIndicator,
            "syntax.warning.experimental-dictionary-ui-indicator",
            "dictionary-first UI3a experimental indicator",
        ),
        (
            ExperimentalConstruct::ExperimentalFihoiAdverbial,
            "syntax.warning.experimental-fihoi-adverbial",
            "FIhOI bridi/subbridi adverbial term",
        ),
        (
            ExperimentalConstruct::ExperimentalSoiAdverbial,
            "syntax.warning.experimental-soi-adverbial",
            "SOI/XOI bridi/subbridi adverbial term",
        ),
        (
            ExperimentalConstruct::ExperimentalPreposedLinkargs,
            "syntax.warning.experimental-preposed-linkargs",
            "BE linkargs before a selbri unit",
        ),
        (
            ExperimentalConstruct::ExperimentalTermBoConnection,
            "syntax.warning.experimental-term-bo-connection",
            "BO-bound term or linked-argument connection",
        ),
        (
            ExperimentalConstruct::ExperimentalTermLooseConnection,
            "syntax.warning.experimental-term-loose-connection",
            "loose term or linked-argument connection",
        ),
        (
            ExperimentalConstruct::ExperimentalExpDescriptionLeadingSumti,
            "syntax.warning.experimental-exp-description-leading-sumti",
            "full sumti as the leading element of a description tail",
        ),
        (
            ExperimentalConstruct::CllProhibitedFreeModifierPlacement,
            "syntax.warning.cll-prohibited-free-modifier-placement",
            "free modifier placement prohibited by CLL grammar",
        ),
        (
            ExperimentalConstruct::ExperimentalSpaceBeforeTimeTense,
            "syntax.warning.experimental-space-before-time-tense",
            "space before time in one tense. CLL 10.4 puts the time part first",
        ),
    ];
    assert_eq!(ExperimentalConstruct::ALL.len(), expected.len());
    for (actual, (kind, code, message)) in ExperimentalConstruct::ALL.iter().zip(expected) {
        assert_eq!(*actual, kind);
        assert_eq!(actual.code().as_bytes(), code.as_bytes());
        assert_eq!(actual.message().as_bytes(), message.as_bytes());
    }
}
