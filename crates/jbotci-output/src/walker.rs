use crate::OutputError;
use crate::sexpr::{SExpr, empty_node, leaf, node};
use crate::surface::{format_word_with_modifiers, is_compound_word_with_modifiers};
use bityzba::{data, requires};
use jbotci_morphology::{WordWithModifiers, WordWithModifiersData};
use jbotci_syntax::{SyntaxNode, SyntaxValue, SyntaxValueData};

#[requires(true)]
#[ensures(true)]
pub(crate) fn to_sexpr(value: &SyntaxValue, source: &str) -> Result<SExpr, OutputError> {
    match value.as_data() {
        data!(SyntaxValue::Null)
        | data!(SyntaxValue::Bool { .. })
        | data!(SyntaxValue::Integer { .. })
        | data!(SyntaxValue::Json { .. }) => Ok(empty_node()),
        data!(SyntaxValue::Text { value }) => Ok(leaf(value.clone())),
        data!(SyntaxValue::Word { word }) => Ok(word_leaf(word, source)),
        data!(SyntaxValue::List { items }) => list_sexpr(items, source),
        data!(SyntaxValue::Node { node: syntax_node }) => node_sexpr(value, syntax_node, source),
    }
}

#[requires(true)]
#[ensures(true)]
fn node_sexpr(
    value: &SyntaxValue,
    syntax_node: &SyntaxNode,
    source: &str,
) -> Result<SExpr, OutputError> {
    let constructor = syntax_node.constructor.as_str();
    match constructor {
        "[]" | "Nothing" => Ok(empty_node()),
        "(:)" => cons_node_sexpr(value, source),
        "TenseModal" => tense_modal_sexpr(value, source),
        "PredicateStatementContinuation" => predicate_statement_continuation_sexpr(value, source),
        _ if is_compound_quote_node(constructor, value) => compound_quote_sexpr(value, source),
        "ConnectedOperator" => connected_operator_sexpr(value, source),
        "ReversePolishExpression" => reverse_polish_expression_sexpr(value, source),
        "CeiRelationUnit" => cei_relation_unit_sexpr(value, source),
        "TaggedArgument" => named_fields_sexpr(
            value,
            source,
            &["tagWords", "freeModifiers", "innerArgument"],
        ),
        "TermWrappedArgument" => named_fields_sexpr(
            value,
            source,
            &[
                "wrapper",
                "wrapperBo",
                "freeModifiers",
                "innerTerm",
                "luhu",
                "luhuFreeModifiers",
            ],
        ),
        "Connective" => named_fields_sexpr(
            value,
            source,
            &["se", "nahe", "na", "cmavo", "nai", "freeModifiers"],
        ),
        "FihoModal" => named_fields_sexpr(
            value,
            source,
            &[
                "fiho",
                "fihoFreeModifiers",
                "relation",
                "fehu",
                "fehuFreeModifiers",
            ],
        ),
        "ZoiQuote" | "LahoQuote" => named_fields_sexpr(
            value,
            source,
            &[
                constructor_to_leading_quote_field(constructor),
                "openingDelimiter",
                "quotedText",
                "closingDelimiter",
                "freeModifiers",
            ],
        ),
        "MuhoiRelationUnit"
            if first_word_field(value).is_some_and(is_compound_word_with_modifiers) =>
        {
            named_fields_sexpr(value, source, &["muhoi", "freeModifiers"])
        }
        "MuhoiRelationUnit" => named_fields_sexpr(
            value,
            source,
            &[
                "muhoi",
                "openingDelimiter",
                "quotedText",
                "closingDelimiter",
                "freeModifiers",
            ],
        ),
        _ => source_ordered_node_sexpr(value, syntax_node, source),
    }
}

#[requires(!constructor.is_empty())]
#[ensures(!ret.is_empty())]
fn constructor_to_leading_quote_field(constructor: &str) -> &'static str {
    match constructor {
        "ZoiQuote" => "zoi",
        "LahoQuote" => "laho",
        _ => "zoi",
    }
}

#[requires(true)]
#[ensures(true)]
fn source_ordered_node_sexpr(
    value: &SyntaxValue,
    syntax_node: &SyntaxNode,
    source: &str,
) -> Result<SExpr, OutputError> {
    if syntax_node.fields.is_empty() {
        return Ok(empty_node());
    }
    if constructor_uses_unnamed_source_order(syntax_node.constructor.as_str()) {
        return fields_in_stored_order_sexpr(syntax_node, source);
    }
    let Some(order) = source_field_order(syntax_node.constructor.as_str()) else {
        return Err(OutputError::InvalidSyntaxTree(format!(
            "no bracket walker source mapping for constructor `{}`",
            syntax_node.constructor
        )));
    };
    named_fields_sexpr(value, source, order)
}

#[requires(!constructor.is_empty())]
#[ensures(true)]
fn constructor_uses_unnamed_source_order(constructor: &str) -> bool {
    matches!(
        constructor,
        "(,)"
            | "Just"
            | "PlainSubsentence"
            | "PrenexSubsentence"
            | "ArgumentTailArgument"
            | "ArgumentTailRelativeClauses"
            | "ArgumentTailQuantifier"
    )
}

#[requires(true)]
#[ensures(true)]
fn source_field_order(constructor: &str) -> Option<&'static [&'static str]> {
    Some(match constructor {
        "LojbanText" => &[
            "leadingNai",
            "leadingCmevla",
            "leadingIndicators",
            "leadingFreeModifiers",
            "leadingConnective",
            "paragraphs",
        ],
        "Paragraph" => &["i", "niho", "freeModifiers", "statements"],
        "ParagraphStatement" => &["i", "connective", "freeModifiers", "statement"],
        "Prenex" => &["terms", "zohu", "zohuFreeModifiers"],
        "StatementPredicate" => &["predicate"],
        "StatementFragment" => &["fragment"],
        "ConnectedStatement" => &["leadingStatement", "i", "connective", "trailingStatement"],
        "PreIConnectedStatement" => &["leadingStatement", "connective", "i", "trailingStatement"],
        "ExperimentalPredicateContinuationStatement" => &["leadingStatement", "continuation"],
        "IauStatement" => &["innerStatement", "iau", "iauFreeModifiers", "resetTerms"],
        "TuheStatement" => &[
            "tenseModal",
            "tuhe",
            "tuheFreeModifiers",
            "paragraphs",
            "tuhu",
            "tuhuFreeModifiers",
        ],
        "PrenexStatement" => &["prenexTerms", "zohu", "zohuFreeModifiers", "innerStatement"],
        "PredicateStatementBo" => &["bo", "freeModifiers"],
        "PredicateStatementKe" => &["ke", "keFreeModifiers", "kehe", "keheFreeModifiers"],
        "ArgumentFragment" => &["argument"],
        "TermFragment" => &["terms", "vau", "vauFreeModifiers"],
        "RelationFragment" => &["relation"],
        "VocativeFragment" => &[
            "vocativeMarkers",
            "freeModifiers",
            "vocativeArgument",
            "dohu",
            "dohuFreeModifiers",
        ],
        "EkFragment" | "GihekFragment" => &["connective", "freeModifiers"],
        "IjekFragment" => &["i", "connective"],
        "PrenexFragment" => &["terms", "zohu", "zohuFreeModifiers"],
        "RelativeClauseFragment" => &["relativeClauses"],
        "BeLinkFragment" => &[
            "be",
            "freeModifiers",
            "fa",
            "faFreeModifiers",
            "firstArgument",
            "beiLinks",
            "beho",
            "behoFreeModifiers",
        ],
        "BeiLinkFragment" => &["beiOnlyLinks"],
        "OtherFragment" => &["otherWords", "freeModifiers"],
        "MathExpressionFragment" => &["mathExpression"],
        "Predicate" => &[
            "leadingTerms",
            "cu",
            "cuFreeModifiers",
            "predicateTail",
            "freeModifiers",
        ],
        "PredicateTail" => &["first", "keContinuation"],
        "KePredicateTail" => &[
            "connective",
            "tenseModal",
            "ke",
            "keFreeModifiers",
            "predicateTail",
            "kehe",
            "keheFreeModifiers",
            "tailTerms",
            "vau",
        ],
        "PredicateTail1" => &["first", "continuations"],
        "PredicateTailContinuation" => &[
            "connective",
            "tenseModal",
            "cu",
            "cuFreeModifiers",
            "predicateTail",
            "tailTerms",
            "vau",
            "freeModifiers",
        ],
        "PredicateTail2" => &["first", "boContinuation"],
        "BoPredicateTail" => &[
            "connective",
            "tenseModal",
            "bo",
            "freeModifiers",
            "cu",
            "cuFreeModifiers",
            "predicateTail",
            "tailTerms",
            "vau",
        ],
        "RelationPredicateTail3" => &["relation", "terms", "vau", "freeModifiers"],
        "GekSentencePredicateTail3" => &["gekSentence"],
        "GekSentencePair" => &[
            "gek",
            "first",
            "gik",
            "second",
            "tailTerms",
            "vau",
            "freeModifiers",
        ],
        "KeGekSentence" => &[
            "tenseModal",
            "ke",
            "keFreeModifiers",
            "inner",
            "kehe",
            "keheFreeModifiers",
        ],
        "NaGekSentence" => &["na", "freeModifiers", "inner"],
        "ArgumentTerm" => &["argument"],
        "TaggedTerm" => &["tenseModal", "freeModifiers", "argument"],
        "FaTerm" => &["fa", "freeModifiers", "argument", "ku", "kuFreeModifiers"],
        "NuhiTermset" => &[
            "nuhi",
            "nuhiFreeModifiers",
            "termset",
            "nuhu",
            "nuhuFreeModifiers",
        ],
        "GekNuhiTermset" => &[
            "mNuhi",
            "nuhiFreeModifiers",
            "gek",
            "terms",
            "nuhu",
            "nuhuFreeModifiers",
            "gik",
            "gikTerms",
            "gikNuhu",
            "gikNuhuFreeModifiers",
        ],
        "NaKuTerm" => &["na", "naKu", "freeModifiers"],
        "BareNaTerm" => &["na", "freeModifiers"],
        "NoihaAdverbialTerm" => &[
            "noiha",
            "leadingFreeModifiers",
            "tailElements",
            "relation",
            "relativeClauses",
            "fehu",
            "trailingFreeModifiers",
        ],
        "PoihaBrigahiTerm" => &[
            "poiha",
            "leadingFreeModifiers",
            "tailElements",
            "relation",
            "relativeClauses",
            "brigahiKu",
            "trailingFreeModifiers",
        ],
        "FihoiAdverbialTerm" => &[
            "fihoi",
            "leadingFreeModifiers",
            "subsentence",
            "fihau",
            "trailingFreeModifiers",
        ],
        "SoiAdverbialTerm" => &[
            "soi",
            "leadingFreeModifiers",
            "subsentence",
            "sehu",
            "trailingFreeModifiers",
        ],
        "CeheTerm" => &["leadingTerms", "cehe", "freeModifiers", "trailingTerms"],
        "PeheTerm" => &[
            "leadingTerms",
            "pehe",
            "freeModifiers",
            "connective",
            "trailingTerms",
        ],
        "ConnectedTerm" => &["leadingTerms", "connective", "trailingTerms"],
        "BoConnectedTerm" => &[
            "leadingTerms",
            "boConnective",
            "tenseModal",
            "bo",
            "freeModifiers",
            "trailingTerm",
        ],
        "BaseRelation" => &["word"],
        "CompoundRelation" => &["relationUnits"],
        "AbstractionRelation" => &["abstraction"],
        "ConnectedRelation" => &["leadingRelation", "connective", "trailingRelation"],
        "NaRelation" => &["na", "freeModifiers", "innerRelation"],
        "SeRelation" => &["se", "freeModifiers", "innerRelation"],
        "KeRelation" => &[
            "keTenseModal",
            "ke",
            "keFreeModifiers",
            "innerRelation",
            "kehe",
            "keheFreeModifiers",
        ],
        "CoRelation" => &["leadingRelation", "co", "freeModifiers", "trailingRelation"],
        "BoRelation" => &[
            "leadingRelation",
            "boConnective",
            "boTenseModal",
            "bo",
            "freeModifiers",
            "trailingRelation",
        ],
        "TenseModalRelation" => &["tenseModal", "innerRelation"],
        "GuhaRelation" => &["guhek", "leadingPredicate", "gik", "trailingPredicate"],
        "WordRelationUnit" => &["word", "freeModifiers"],
        "AbstractionRelationUnit" => &["abstraction"],
        "MeRelationUnit" => &[
            "me",
            "meFreeModifiers",
            "argument",
            "mehu",
            "mehuFreeModifiers",
            "moiMarker",
            "moiFreeModifiers",
        ],
        "MehoiRelationUnit" => &["mehoi", "quotedText", "freeModifiers"],
        "GohoiRelationUnit" => &["gohoi", "quotedText", "freeModifiers"],
        "LuheiRelationUnit" => &[
            "luhei",
            "luheiFreeModifiers",
            "text",
            "liau",
            "liauFreeModifiers",
        ],
        "XohiRelationUnit" => &["xohi", "freeModifiers", "tag"],
        "BeRelationUnit" => &[
            "base",
            "be",
            "freeModifiers",
            "fa",
            "faFreeModifiers",
            "firstArgument",
            "beiLinks",
            "beho",
            "behoFreeModifiers",
        ],
        "PreposedBeRelationUnit" => &[
            "be",
            "freeModifiers",
            "fa",
            "faFreeModifiers",
            "firstArgument",
            "beiLinks",
            "beho",
            "behoFreeModifiers",
            "base",
        ],
        "SeRelationUnit" => &["se", "freeModifiers", "innerUnit"],
        "KeRelationUnit" => &[
            "keTenseModal",
            "ke",
            "keFreeModifiers",
            "relation",
            "kehe",
            "keheFreeModifiers",
        ],
        "GohaRelationUnit" => &["goha", "raho", "freeModifiers"],
        "MoiRelationUnit" => &["number", "moi", "freeModifiers"],
        "NuhaRelationUnit" => &["nuha", "freeModifiers", "mathOperator"],
        "BoRelationUnit" => &[
            "leadingUnit",
            "boConnective",
            "boTenseModal",
            "bo",
            "freeModifiers",
            "trailingUnit",
        ],
        "JaiRelationUnit" => &["jai", "freeModifiers", "tenseModal", "innerUnit"],
        "NaheRelationUnit" => &["nahe", "freeModifiers", "innerUnit"],
        "ConnectedRelationUnit" => &["leadingUnit", "connective", "trailingUnit"],
        "SelbriRelativeClauseRelationUnit" => &["base", "selbriRelativeClauses"],
        "WrappedRelationUnit" => &["relation"],
        "CeiAssignment" => &["cei", "freeModifiers", "relationUnit"],
        "Abstraction" => &[
            "nu",
            "nai",
            "freeModifiers",
            "additionalNu",
            "subsentence",
            "kei",
            "keiFreeModifiers",
        ],
        "AdditionalNu" => &["connective", "nu", "nai", "freeModifiers"],
        "KohaArgument" => &["koha", "freeModifiers"],
        "DescriptorArgument" => &["descriptor"],
        "ConnectedDescriptorArgument" => &["connectedDescriptor"],
        "NameArgument" => &["la", "laFreeModifiers", "names", "nameFreeModifiers"],
        "QuoteArgument" => &["quote", "freeModifiers"],
        "MathExpressionArgument" => &[
            "li",
            "liFreeModifiers",
            "mathExpression",
            "loho",
            "lohoFreeModifiers",
        ],
        "LetterArgument" => &["letter", "boi", "boiFreeModifiers"],
        "ConnectedArgument" => &["leadingArgument", "connective", "trailingArgument"],
        "LaheArgument" => &[
            "lahe",
            "freeModifiers",
            "laheRelativeClauses",
            "innerArgument",
            "luhu",
            "luhuFreeModifiers",
        ],
        "NaheBoArgument" => &[
            "nahe",
            "bo",
            "freeModifiers",
            "innerArgument",
            "luhu",
            "luhuFreeModifiers",
        ],
        "NaheArgument" => &[
            "nahe",
            "freeModifiers",
            "innerArgument",
            "luhu",
            "luhuFreeModifiers",
        ],
        "RelativeClauseArgument" => &[
            "baseArgument",
            "vuho",
            "vuhoFreeModifiers",
            "relativeClauses",
        ],
        "VuhoArgument" => &[
            "baseArgument",
            "vuhoMarker",
            "vuhoFreeModifiers",
            "relativeClauses",
            "connectedArgument",
        ],
        "BridiDescriptionArgument" => &[
            "lohoi",
            "lohoiFreeModifiers",
            "subsentence",
            "kuhau",
            "kuhauFreeModifiers",
        ],
        "KeArgument" => &[
            "ke",
            "keFreeModifiers",
            "innerArgument",
            "kehe",
            "keheFreeModifiers",
        ],
        "BoArgument" => &[
            "leadingArgument",
            "boConnective",
            "boTenseModal",
            "bo",
            "freeModifiers",
            "trailingArgument",
        ],
        "GekArgument" => &["gek", "leadingArgument", "gik", "trailingArgument"],
        "ZoheArgument" => &["tagWords", "maybeKu", "freeModifiers"],
        "CmevlaArgument" => &["cmevla", "freeModifiers"],
        "RelationVocativeArgument" => &[
            "leadingRelativeClauses",
            "relation",
            "trailingRelativeClauses",
        ],
        "NaKuArgument" => &["na", "ku", "freeModifiers"],
        "QuantifiedArgument" => &["quantifier", "innerArgument"],
        "Descriptor" => &[
            "outerQuantifier",
            "descriptor",
            "descriptorFreeModifiers",
            "tailElements",
            "relation",
            "relativeClauses",
            "ku",
            "kuFreeModifiers",
        ],
        "DescriptorHead" => &["descriptor", "descriptorFreeModifiers"],
        "ConnectedDescriptor" => &[
            "leadingDescriptorHead",
            "connective",
            "trailingDescriptorHead",
            "tailElements",
            "relation",
            "relativeClauses",
            "ku",
            "kuFreeModifiers",
        ],
        "BeiLink" => &[
            "bei",
            "beiFreeModifiers",
            "fa",
            "faFreeModifiers",
            "argument",
        ],
        "NumberQuantifier" => &["number", "boi", "boiFreeModifiers"],
        "VeiQuantifier" => &[
            "vei",
            "freeModifiers",
            "mathExpression",
            "veho",
            "vehoFreeModifiers",
        ],
        "PoiRelativeClause" => &[
            "poi",
            "leadingFreeModifiers",
            "subsentence",
            "kuho",
            "trailingFreeModifiers",
        ],
        "NoiRelativeClause" => &[
            "noi",
            "leadingFreeModifiers",
            "subsentence",
            "kuho",
            "trailingFreeModifiers",
        ],
        "GoiRelativeClause" => &[
            "goi",
            "leadingFreeModifiers",
            "argument",
            "gehu",
            "trailingFreeModifiers",
        ],
        "ZiheRelativeClause" => &["zihe", "freeModifiers", "inner"],
        "ConnectedRelativeClause" => &["connective", "inner"],
        "SelbriRelativeClause" => &[
            "nohoi",
            "leadingFreeModifiers",
            "relation",
            "kuhoi",
            "trailingFreeModifiers",
        ],
        "NumberExpression" => &["number", "boi", "freeModifiers"],
        "LetterExpression" => &["letter", "boi", "freeModifiers"],
        "BinaryExpression" => &["leftExpression", "operator", "rightExpression"],
        "UnaryExpression" => &["operator", "innerExpression"],
        "VeiExpression" => &[
            "vei",
            "freeModifiers",
            "innerExpression",
            "veho",
            "vehoFreeModifiers",
        ],
        "ForethoughtExpression" => &[
            "peho",
            "freeModifiers",
            "operator",
            "operands",
            "kuhe",
            "kuheFreeModifiers",
        ],
        "BoExpression" => &[
            "leftExpression",
            "operator",
            "bo",
            "freeModifiers",
            "rightExpression",
        ],
        "BiheExpression" => &[
            "leftExpression",
            "bihe",
            "freeModifiers",
            "operator",
            "rightExpression",
        ],
        "NiheExpression" => &[
            "nihe",
            "freeModifiers",
            "relation",
            "tehu",
            "tehuFreeModifiers",
        ],
        "MoheExpression" => &[
            "mohe",
            "freeModifiers",
            "argument",
            "tehu",
            "tehuFreeModifiers",
        ],
        "JohiExpression" => &[
            "johi",
            "freeModifiers",
            "expressions",
            "tehu",
            "tehuFreeModifiers",
        ],
        "ConnectedExpression" => &["leftExpression", "connective", "rightExpression"],
        "LaheExpression" => &[
            "markers",
            "freeModifiers",
            "innerExpression",
            "luhu",
            "luhuFreeModifiers",
        ],
        "GekExpression" => &["gek", "leftExpression", "gik", "rightExpression"],
        "VuhuOperator" => &["vuhu", "freeModifiers"],
        "SeOperator" => &["se", "freeModifiers", "innerOperator"],
        "NaheOperator" => &["nahe", "freeModifiers", "innerOperator"],
        "KeOperator" => &[
            "ke",
            "keFreeModifiers",
            "innerOperator",
            "kehe",
            "keheFreeModifiers",
        ],
        "BoOperator" => &["leftOperator", "bo", "freeModifiers", "rightOperator"],
        "MahoOperator" => &[
            "maho",
            "freeModifiers",
            "mathExpression",
            "tehu",
            "tehuFreeModifiers",
        ],
        "NahuOperator" => &[
            "nahu",
            "freeModifiers",
            "relation",
            "tehu",
            "tehuFreeModifiers",
        ],
        "JohiOperator" => &[
            "johi",
            "freeModifiers",
            "expressions",
            "tehu",
            "tehuFreeModifiers",
        ],
        "NumberOperator" => &["number"],
        "LuQuote" => &["lu", "freeModifiers", "text", "lihu", "lihuFreeModifiers"],
        "ZoQuote" => &["zo", "word", "freeModifiers"],
        "ZohOiQuote" => &["zohoi", "quotedText", "freeModifiers"],
        "LohuQuote" => &["lohu", "quotedWords", "lehu", "lehuFreeModifiers"],
        "MehoQuote" => &["meho", "freeModifiers", "mathExpression"],
        "SeiFree" => &[
            "sei",
            "leadingFreeModifiers",
            "terms",
            "cu",
            "cuFreeModifiers",
            "relation",
            "sehu",
            "sehuFreeModifiers",
        ],
        "SoiFree" => &[
            "soi",
            "freeModifiers",
            "leadingArgument",
            "trailingArgument",
            "sehu",
            "sehuFreeModifiers",
        ],
        "MaiFree" => &["number", "mai", "freeModifiers"],
        "ToFree" => &["to", "freeModifiers", "text", "toi", "toiFreeModifiers"],
        "XiFree" => &["xi", "freeModifiers", "mathExpression"],
        "VocativeFree" => &[
            "vocativeMarkers",
            "freeModifiers",
            "argument",
            "dohu",
            "dohuFreeModifiers",
        ],
        "ReplacementFree" => &[
            "lohai",
            "oldWords",
            "sahai",
            "newWords",
            "lehai",
            "freeModifiers",
        ],
        _ => return None,
    })
}

#[requires(true)]
#[ensures(true)]
fn list_sexpr(items: &[SyntaxValue], source: &str) -> Result<SExpr, OutputError> {
    if let [head, tail] = items
        && syntax_value_is_list_tail(tail)
    {
        let mut children = vec![to_sexpr(head, source)?];
        children.extend(list_tail_items(tail, source)?);
        return Ok(node(children));
    }
    let children = items
        .iter()
        .map(|item| to_sexpr(item, source))
        .collect::<Result<Vec<_>, _>>()?;
    Ok(node(children))
}

#[requires(true)]
#[ensures(true)]
fn syntax_value_is_list_tail(value: &SyntaxValue) -> bool {
    matches!(
        value.as_data(),
        data!(SyntaxValue::Node { node }) if node.constructor == "[]" || node.constructor == "(:)"
    )
}

#[requires(true)]
#[ensures(true)]
fn cons_node_sexpr(value: &SyntaxValue, source: &str) -> Result<SExpr, OutputError> {
    let data!(SyntaxValue::Node { node: syntax_node }) = value.as_data() else {
        return Ok(empty_node());
    };
    cons_list_items(syntax_node, source).map(node)
}

#[requires(true)]
#[ensures(true)]
fn cons_list_items(syntax_node: &SyntaxNode, source: &str) -> Result<Vec<SExpr>, OutputError> {
    let Some(head) = syntax_node.fields.first() else {
        return Ok(Vec::new());
    };
    let Some(tail) = syntax_node.fields.get(1) else {
        return Ok(vec![to_sexpr(&head.value, source)?]);
    };

    let mut children = vec![to_sexpr(&head.value, source)?];
    children.extend(list_tail_items(&tail.value, source)?);
    Ok(children)
}

#[requires(true)]
#[ensures(true)]
fn list_tail_items(value: &SyntaxValue, source: &str) -> Result<Vec<SExpr>, OutputError> {
    match value.as_data() {
        data!(SyntaxValue::Node { node }) if node.constructor == "[]" => Ok(Vec::new()),
        data!(SyntaxValue::Node { node }) if node.constructor == "(:)" => {
            cons_list_items(node, source)
        }
        _ => Ok(vec![to_sexpr(value, source)?]),
    }
}

#[requires(true)]
#[ensures(true)]
fn fields_in_stored_order_sexpr(
    syntax_node: &SyntaxNode,
    source: &str,
) -> Result<SExpr, OutputError> {
    syntax_node
        .fields
        .iter()
        .map(|field| {
            field_value_sexpr(
                syntax_node.constructor.as_str(),
                field.name.as_deref(),
                &field.value,
                source,
            )
        })
        .collect::<Result<Vec<_>, _>>()
        .map(node)
}

#[requires(true)]
#[ensures(true)]
fn field_value_sexpr(
    constructor: &str,
    field_name: Option<&str>,
    value: &SyntaxValue,
    source: &str,
) -> Result<SExpr, OutputError> {
    match (constructor, field_name, value.as_data()) {
        ("ZoiQuote" | "LahoQuote", Some("quotedText"), data!(SyntaxValue::Text { value })) => {
            Ok(leaf(format!("\"{value}\"")))
        }
        (
            "ZohOiQuote" | "MehoiRelationUnit" | "GohoiRelationUnit" | "MuhoiRelationUnit",
            Some("quotedText"),
            data!(SyntaxValue::Text { value }),
        ) => Ok(leaf(format!("«{value}»"))),
        _ => to_sexpr(value, source),
    }
}

#[requires(!field_names.is_empty())]
#[ensures(true)]
fn named_fields_sexpr(
    value: &SyntaxValue,
    source: &str,
    field_names: &[&str],
) -> Result<SExpr, OutputError> {
    field_names
        .iter()
        .map(|field_name| named_field_sexpr(value, source, field_name))
        .collect::<Result<Vec<_>, _>>()
        .map(node)
}

#[requires(!field_name.is_empty())]
#[ensures(true)]
fn named_field_sexpr(
    value: &SyntaxValue,
    source: &str,
    field_name: &str,
) -> Result<SExpr, OutputError> {
    named_field(value, field_name).map_or_else(
        || Ok(empty_node()),
        |field| {
            field_value_sexpr(
                node_constructor(Some(value)).unwrap_or_default(),
                Some(field_name),
                field,
                source,
            )
        },
    )
}

#[requires(!name.is_empty())]
#[ensures(true)]
fn named_field<'tree>(value: &'tree SyntaxValue, name: &str) -> Option<&'tree SyntaxValue> {
    let data!(SyntaxValue::Node { node }) = value.as_data() else {
        return None;
    };
    node.fields
        .iter()
        .find(|field| field.name.as_deref() == Some(name))
        .map(|field| &field.value)
}

#[requires(true)]
#[ensures(true)]
fn predicate_statement_continuation_sexpr(
    value: &SyntaxValue,
    source: &str,
) -> Result<SExpr, OutputError> {
    let marker = named_field(value, "marker");
    let mut children = vec![
        named_field_sexpr(value, source, "connective")?,
        named_field_sexpr(value, source, "tenseModal")?,
    ];
    children.extend(predicate_statement_marker_before(marker, source)?);
    children.push(named_field_sexpr(value, source, "trailingSubsentence")?);
    children.extend(predicate_statement_marker_after(marker, source)?);
    Ok(node(children))
}

#[requires(true)]
#[ensures(true)]
fn predicate_statement_marker_before(
    marker: Option<&SyntaxValue>,
    source: &str,
) -> Result<Vec<SExpr>, OutputError> {
    match marker.and_then(|value| node_constructor(Some(value))) {
        Some("PredicateStatementBo") => Ok(vec![
            named_field_sexpr(marker.unwrap(), source, "bo")?,
            named_field_sexpr(marker.unwrap(), source, "freeModifiers")?,
        ]),
        Some("PredicateStatementKe") => Ok(vec![
            named_field_sexpr(marker.unwrap(), source, "ke")?,
            named_field_sexpr(marker.unwrap(), source, "keFreeModifiers")?,
        ]),
        _ => Ok(Vec::new()),
    }
}

#[requires(true)]
#[ensures(true)]
fn predicate_statement_marker_after(
    marker: Option<&SyntaxValue>,
    source: &str,
) -> Result<Vec<SExpr>, OutputError> {
    match marker.and_then(|value| node_constructor(Some(value))) {
        Some("PredicateStatementKe") => Ok(vec![
            named_field_sexpr(marker.unwrap(), source, "kehe")?,
            named_field_sexpr(marker.unwrap(), source, "keheFreeModifiers")?,
        ]),
        _ => Ok(Vec::new()),
    }
}

#[requires(true)]
#[ensures(true)]
fn tense_modal_sexpr(value: &SyntaxValue, source: &str) -> Result<SExpr, OutputError> {
    let fiho = named_field(value, "fiho");
    let connectives = named_field(value, "connectives");
    let fiho_items = field_list_items(fiho);
    let connective_items = field_list_items(connectives);
    if fiho_items.len() > 1 && !connective_items.is_empty() {
        let connective_offsets = connective_items
            .iter()
            .filter_map(|value| min_word_byte_start(value))
            .collect::<Vec<_>>();
        let leaves = field_list_items(named_field(value, "leaves"))
            .into_iter()
            .filter(|leaf_value| {
                min_word_byte_start(leaf_value)
                    .is_none_or(|offset| !connective_offsets.contains(&offset))
            })
            .map(|leaf_value| to_sexpr(leaf_value, source))
            .collect::<Result<Vec<_>, _>>()?;
        let mut interleaved = Vec::new();
        let mut connectives_iter = connective_items.iter();
        for (index, fiho_value) in fiho_items.iter().enumerate() {
            if index > 0
                && let Some(connective) = connectives_iter.next()
            {
                interleaved.push(to_sexpr(connective, source)?);
            }
            interleaved.push(to_sexpr(fiho_value, source)?);
        }
        return Ok(node(vec![
            node(leaves),
            named_field_sexpr(value, source, "freeModifiers")?,
            node(interleaved),
        ]));
    }
    named_fields_sexpr(value, source, &["leaves", "freeModifiers", "fiho"])
}

#[requires(true)]
#[ensures(true)]
fn connected_operator_sexpr(value: &SyntaxValue, source: &str) -> Result<SExpr, OutputError> {
    let Some(connective) = named_field(value, "connective") else {
        return named_fields_sexpr(
            value,
            source,
            &["leftOperator", "connective", "rightOperator"],
        );
    };
    if node_constructor(named_field(connective, "kind")) == Some("ForethoughtConnective") {
        let cmavo = field_list_items(named_field(connective, "cmavo"));
        if let Some((gi_tok, forethought)) = cmavo.split_last()
            && !forethought.is_empty()
        {
            let forethought_connective = connective_with_cmavo(connective, forethought, source)?;
            return Ok(node(vec![
                forethought_connective,
                named_field_sexpr(value, source, "leftOperator")?,
                to_sexpr(gi_tok, source)?,
                named_field_sexpr(value, source, "rightOperator")?,
            ]));
        }
    }
    named_fields_sexpr(
        value,
        source,
        &["leftOperator", "connective", "rightOperator"],
    )
}

#[requires(!forethought.is_empty())]
#[ensures(true)]
fn connective_with_cmavo(
    connective: &SyntaxValue,
    forethought: &[&SyntaxValue],
    source: &str,
) -> Result<SExpr, OutputError> {
    Ok(node(vec![
        named_field_sexpr(connective, source, "se")?,
        named_field_sexpr(connective, source, "nahe")?,
        named_field_sexpr(connective, source, "na")?,
        node(
            forethought
                .iter()
                .map(|value| to_sexpr(value, source))
                .collect::<Result<Vec<_>, _>>()?,
        ),
        named_field_sexpr(connective, source, "nai")?,
        named_field_sexpr(connective, source, "freeModifiers")?,
    ]))
}

#[requires(true)]
#[ensures(true)]
fn reverse_polish_expression_sexpr(
    value: &SyntaxValue,
    source: &str,
) -> Result<SExpr, OutputError> {
    let mut body = field_list_items(named_field(value, "operands"))
        .into_iter()
        .chain(field_list_items(named_field(value, "operators")))
        .filter_map(|item| min_word_byte_start(item).map(|offset| (offset, item)))
        .collect::<Vec<_>>();
    body.sort_by_key(|(offset, _)| *offset);
    Ok(node(vec![
        named_field_sexpr(value, source, "fuha")?,
        named_field_sexpr(value, source, "freeModifiers")?,
        node(
            body.into_iter()
                .map(|(_, item)| to_sexpr(item, source))
                .collect::<Result<Vec<_>, _>>()?,
        ),
    ]))
}

#[requires(true)]
#[ensures(true)]
fn cei_relation_unit_sexpr(value: &SyntaxValue, source: &str) -> Result<SExpr, OutputError> {
    let mut children = vec![named_field_sexpr(value, source, "base")?];
    children.extend(
        field_list_items(named_field(value, "assignments"))
            .into_iter()
            .map(|assignment| to_sexpr(assignment, source))
            .collect::<Result<Vec<_>, _>>()?,
    );
    Ok(node(children))
}

#[requires(true)]
#[ensures(true)]
fn is_compound_quote_node(constructor: &str, value: &SyntaxValue) -> bool {
    matches!(
        constructor,
        "ZoQuote" | "ZohOiQuote" | "ZoiQuote" | "LahoQuote" | "LohuQuote"
    ) && first_word_field(value).is_some_and(is_compound_word_with_modifiers)
}

#[requires(true)]
#[ensures(true)]
fn compound_quote_sexpr(value: &SyntaxValue, source: &str) -> Result<SExpr, OutputError> {
    let word = first_word_field(value).ok_or_else(|| {
        OutputError::InvalidSyntaxTree("compound quote node has no leading word".to_owned())
    })?;
    let mut children = vec![word_leaf(word, source)];
    if let Some(free_modifiers) = named_field(value, "freeModifiers") {
        children.push(to_sexpr(free_modifiers, source)?);
    } else if let Some(free_modifiers) = named_field(value, "lehuFreeModifiers") {
        children.push(to_sexpr(free_modifiers, source)?);
    }
    Ok(node(children))
}

#[requires(true)]
#[ensures(true)]
fn first_word_field(value: &SyntaxValue) -> Option<&WordWithModifiers> {
    let data!(SyntaxValue::Node { node }) = value.as_data() else {
        return None;
    };
    node.fields.iter().find_map(|field| {
        let data!(SyntaxValue::Word { word }) = field.value.as_data() else {
            return None;
        };
        Some(word.as_ref())
    })
}

#[requires(true)]
#[ensures(true)]
fn field_list_items(value: Option<&SyntaxValue>) -> Vec<&SyntaxValue> {
    let Some(value) = value else {
        return Vec::new();
    };
    syntax_list_items(value)
}

#[requires(true)]
#[ensures(true)]
fn syntax_list_items(value: &SyntaxValue) -> Vec<&SyntaxValue> {
    match value.as_data() {
        data!(SyntaxValue::List { items }) => items.iter().collect(),
        data!(SyntaxValue::Node { node }) if node.constructor == "[]" => Vec::new(),
        data!(SyntaxValue::Node { node }) if node.constructor == "(:)" => {
            let Some(head) = node.fields.first().map(|field| &field.value) else {
                return Vec::new();
            };
            let Some(tail) = node.fields.get(1).map(|field| &field.value) else {
                return vec![head];
            };
            let mut values = vec![head];
            values.extend(syntax_list_items(tail));
            values
        }
        _ => vec![value],
    }
}

#[requires(true)]
#[ensures(true)]
fn min_word_byte_start(value: &SyntaxValue) -> Option<usize> {
    match value.as_data() {
        data!(SyntaxValue::Word { word }) => word_byte_start(word),
        data!(SyntaxValue::List { items }) => items.iter().filter_map(min_word_byte_start).min(),
        data!(SyntaxValue::Node { node }) => node
            .fields
            .iter()
            .filter_map(|field| min_word_byte_start(&field.value))
            .min(),
        _ => None,
    }
}

#[requires(true)]
#[ensures(true)]
fn word_byte_start(word: &WordWithModifiers) -> Option<usize> {
    match word.as_data() {
        bityzba::data!(WordWithModifiers::BaseWord { word_like }) => {
            word_like_byte_start(word_like)
        }
        bityzba::data!(WordWithModifiers::StandaloneIndicator { indicator, .. }) => {
            Some(indicator.span.byte_start)
        }
        bityzba::data!(WordWithModifiers::Emphasized { bahe, .. }) => Some(bahe.span.byte_start),
        bityzba::data!(WordWithModifiers::WithIndicator { base, .. }) => word_byte_start(base),
        bityzba::data!(WordWithModifiers::NotEof) => None,
    }
}

#[requires(true)]
#[ensures(true)]
fn word_like_byte_start(word_like: &jbotci_morphology::WordLike) -> Option<usize> {
    match word_like.as_data() {
        bityzba::data!(jbotci_morphology::WordLike::Bare { word }) => Some(word.span.byte_start),
        bityzba::data!(jbotci_morphology::WordLike::ZoQuote { zo, .. }) => Some(zo.span.byte_start),
        bityzba::data!(jbotci_morphology::WordLike::ZoiQuote { zoi, .. }) => {
            Some(zoi.span.byte_start)
        }
        bityzba::data!(jbotci_morphology::WordLike::LohuQuote { lohu, .. }) => {
            Some(lohu.span.byte_start)
        }
        bityzba::data!(jbotci_morphology::WordLike::SingleWordQuote { marker, .. }) => {
            Some(marker.span.byte_start)
        }
        bityzba::data!(jbotci_morphology::WordLike::Letter { base, .. }) => {
            word_like_byte_start(base)
        }
        bityzba::data!(jbotci_morphology::WordLike::ZeiLujvo { left, .. }) => {
            word_like_byte_start(left)
        }
    }
}

#[requires(true)]
#[ensures(true)]
fn node_constructor(value: Option<&SyntaxValue>) -> Option<&str> {
    let Some(value) = value else {
        return None;
    };
    let data!(SyntaxValue::Node { node }) = value.as_data() else {
        return None;
    };
    Some(node.constructor.as_str())
}

#[requires(true)]
#[ensures(true)]
fn word_leaf(word: &WordWithModifiers, source: &str) -> SExpr {
    leaf(format_word_with_modifiers(word, source))
}
