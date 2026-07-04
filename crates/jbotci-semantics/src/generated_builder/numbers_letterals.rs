use super::*;

#[requires(!tokens.is_empty())]
#[ensures(true)]
pub(super) fn simple_pa_integer_from_tokens(tokens: &[Token]) -> Option<i64> {
    let mut value = 0i64;
    for token in tokens {
        value = value.checked_mul(10)?;
        value = value.checked_add(pa_digit_value(token.cmavo()?)?)?;
    }
    Some(value)
}

#[requires(!tokens.is_empty())]
#[requires(!text.is_empty())]
#[ensures(true)]
pub(super) fn parse_generated_recurrence_integer(tokens: &[Token], text: &str) -> Option<i64> {
    parse_generated_relational_pa_integer(text).or_else(|| simple_pa_integer_from_tokens(tokens))
}

#[requires(!text.is_empty())]
#[ensures(true)]
pub(super) fn parse_generated_relational_pa_integer(text: &str) -> Option<i64> {
    let (prefix, rest) = text.split_once(char::is_whitespace)?;
    if !matches!(prefix, "su'o" | "su'e" | "za'u" | "me'i" | "su'a") {
        return None;
    }
    let rest = rest.trim();
    if rest.is_empty() {
        return None;
    }
    parse_generated_simple_pa_integer(rest)
}

#[requires(!text.is_empty())]
#[ensures(true)]
pub(super) fn parse_generated_simple_pa_integer(text: &str) -> Option<i64> {
    let mut words = text.split_whitespace();
    let first = words.next()?;
    let (sign, first_digit) = match first {
        "ni'u" => (-1_i64, words.next()?),
        "ma'u" => (1_i64, words.next()?),
        _ => (1_i64, first),
    };
    let mut value = i64::from(pa_digit_value_for_text(first_digit)?);
    for word in words {
        let digit = i64::from(pa_digit_value_for_text(word)?);
        value = value.checked_mul(10)?.checked_add(digit)?;
    }
    Some(sign * value)
}

#[requires(!word.is_empty())]
#[ensures(ret.is_none_or(|digit| digit <= 9))]
pub(super) fn pa_digit_value_for_text(word: &str) -> Option<u8> {
    match word {
        "no" => Some(0),
        "pa" => Some(1),
        "re" => Some(2),
        "ci" => Some(3),
        "vo" => Some(4),
        "mu" => Some(5),
        "xa" => Some(6),
        "ze" => Some(7),
        "bi" => Some(8),
        "so" => Some(9),
        _ => None,
    }
}

#[requires(true)]
#[ensures(ret.is_none_or(|digit| (0..=9).contains(&digit)))]
pub(super) fn pa_digit_value(cmavo: Cmavo) -> Option<i64> {
    match cmavo {
        Cmavo::No => Some(0),
        Cmavo::Pa => Some(1),
        Cmavo::Re => Some(2),
        Cmavo::Ci => Some(3),
        Cmavo::Vo => Some(4),
        Cmavo::Mu => Some(5),
        Cmavo::Xa => Some(6),
        Cmavo::Ze => Some(7),
        Cmavo::Bi => Some(8),
        Cmavo::So => Some(9),
        _ => None,
    }
}

#[requires(!text.is_empty())]
#[ensures(true)]
pub(super) fn math_literal_for_pa_text(text: String) -> MathLiteral {
    if let Some(components) = parse_generated_mixed_radix_pa_components(&text) {
        return MathLiteral::mixed_radix(components);
    }
    parse_generated_simple_pa_integer(&text)
        .map(MathLiteral::integer)
        .or_else(|| {
            parse_generated_simple_pa_decimal(&text)
                .map(|decimal| MathLiteral::text(MathLiteralKind::Decimal, decimal))
        })
        .unwrap_or_else(|| MathLiteral::text(MathLiteralKind::Number, text))
}

#[requires(!text.is_empty())]
#[ensures(ret.as_ref().is_none_or(|components| components.len() >= 2))]
pub(super) fn parse_generated_mixed_radix_pa_components(
    text: &str,
) -> Option<Vec<MixedRadixComponent>> {
    let words = text.split_whitespace().collect::<Vec<_>>();
    if !words.contains(&"pi'e") {
        return None;
    }
    let mut components = Vec::new();
    let mut current = Vec::new();
    for word in words {
        if word == "pi'e" {
            if current.is_empty() {
                return None;
            }
            let component_text = current.join(" ");
            components.push(MixedRadixComponent::new(
                component_text.clone(),
                parse_generated_simple_pa_integer(&component_text),
            ));
            current.clear();
        } else {
            current.push(word);
        }
    }
    if current.is_empty() {
        return None;
    }
    let component_text = current.join(" ");
    components.push(MixedRadixComponent::new(
        component_text.clone(),
        parse_generated_simple_pa_integer(&component_text),
    ));
    (components.len() >= 2).then_some(components)
}

#[requires(!text.is_empty())]
#[ensures(ret.as_ref().is_none_or(|value| !value.is_empty()))]
pub(super) fn parse_generated_simple_pa_decimal(text: &str) -> Option<String> {
    let words = text.split_whitespace().collect::<Vec<_>>();
    let (sign, words) = match words.as_slice() {
        ["ni'u", rest @ ..] => ("-", rest),
        ["ma'u", rest @ ..] => ("", rest),
        _ => ("", words.as_slice()),
    };
    let point = words.iter().position(|word| *word == "pi")?;
    if words[point + 1..].is_empty() {
        return None;
    }
    let integer = if point == 0 {
        "0".to_owned()
    } else {
        words[..point]
            .iter()
            .map(|word| pa_digit_value_for_text(word).map(|digit| char::from(b'0' + digit)))
            .collect::<Option<String>>()?
    };
    let fraction = words[point + 1..]
        .iter()
        .map(|word| pa_digit_value_for_text(word).map(|digit| char::from(b'0' + digit)))
        .collect::<Option<String>>()?;
    Some(format!("{sign}{integer}.{fraction}"))
}

#[requires(true)]
#[ensures(tokens.is_empty() || !ret.is_empty())]
pub(super) fn letteral_units_for_tokens(tokens: &[Token]) -> Vec<LetteralUnit> {
    let mut units = Vec::new();
    let mut index = 0;
    while index < tokens.len() {
        match token_cmavo(&tokens[index]) {
            Some(Cmavo::Tei) => {
                if let Some(relative_end) = tokens[index + 1..]
                    .iter()
                    .position(|token| token_cmavo(token) == Some(Cmavo::Foi))
                {
                    let end = index + 1 + relative_end;
                    let inner = letteral_units_for_tokens(&tokens[index + 1..end]);
                    if !inner.is_empty() {
                        let source_words = letteral_source_words_for_tokens(&tokens[index..=end]);
                        let value = letteral_unit_values_joined(&inner);
                        units.push(LetteralUnit::compound(source_words, value, inner));
                        index = end + 1;
                        continue;
                    }
                }
                units.push(letteral_unit_for_token(&tokens[index]));
                index += 1;
            }
            Some(Cmavo::Sehe) => {
                let source_words = letteral_source_words_for_tokens(&tokens[index..]);
                let value = if tokens[index + 1..].is_empty() {
                    None
                } else {
                    Some(
                        tokens[index + 1..]
                            .iter()
                            .map(token_text)
                            .collect::<Vec<_>>()
                            .join(""),
                    )
                };
                units.push(LetteralUnit::simple(
                    LetteralUnitKind::CharacterCode,
                    source_words,
                    Some(token_list_text(tokens[index..].iter())),
                    value,
                    None,
                    None,
                ));
                break;
            }
            Some(Cmavo::Tau | Cmavo::Zai | Cmavo::Ceha) if index + 1 < tokens.len() => {
                let marker = token_text(&tokens[index]);
                let next = &tokens[index + 1];
                units.push(LetteralUnit::simple(
                    LetteralUnitKind::Shift,
                    letteral_source_words_for_tokens(&tokens[index..=index + 1]),
                    Some(format!("{marker} {}", token_text(next))),
                    Some(token_text(next)),
                    letteral_shift_modifier(&tokens[index]),
                    None,
                ));
                index += 2;
            }
            Some(
                Cmavo::Gahe
                | Cmavo::Toha
                | Cmavo::Naha
                | Cmavo::Loha
                | Cmavo::Geho
                | Cmavo::Jeho
                | Cmavo::Joho
                | Cmavo::Ruho,
            ) => {
                units.push(LetteralUnit::simple(
                    LetteralUnitKind::Shift,
                    letteral_source_words_for_token(&tokens[index]),
                    Some(token_text(&tokens[index])),
                    None,
                    letteral_shift_modifier(&tokens[index]),
                    None,
                ));
                index += 1;
            }
            _ => {
                units.push(letteral_unit_for_token(&tokens[index]));
                index += 1;
            }
        }
    }
    units
}

#[requires(true)]
#[ensures(!ret.source_words.is_empty())]
pub(super) fn letteral_unit_for_token(token: &Token) -> LetteralUnit {
    let source_words = letteral_source_words_for_token(token);
    let source_text = token_text(token);
    let bu_depth = letteral_bu_depth(token.core_word());
    let value = basic_letteral_value(&source_words);
    let kind = if parse_generated_simple_pa_integer(&source_text).is_some() && bu_depth == 0 {
        LetteralUnitKind::Digit
    } else {
        LetteralUnitKind::Glyph
    };
    LetteralUnit::simple(
        kind,
        source_words,
        Some(source_text),
        value,
        None,
        (bu_depth > 0).then_some(bu_depth),
    )
}

#[requires(true)]
#[ensures(true)]
pub(super) fn letteral_shift_modifier(token: &Token) -> Option<String> {
    let modifier = match token_cmavo(token)? {
        Cmavo::Gahe => "upperCase",
        Cmavo::Toha => "lowerCase",
        Cmavo::Tau => "singleCaseShift",
        Cmavo::Zai => "script",
        Cmavo::Ceha => "font",
        Cmavo::Naha => "cancel",
        Cmavo::Loha => "lojbanScript",
        Cmavo::Geho => "greekScript",
        Cmavo::Jeho => "hebrewScript",
        Cmavo::Joho => "arabicScript",
        Cmavo::Ruho => "cyrillicScript",
        _ => return None,
    };
    Some(modifier.to_owned())
}

#[requires(true)]
#[ensures(tokens.is_empty() || !ret.is_empty())]
pub(super) fn letteral_source_words_for_tokens(tokens: &[Token]) -> Vec<String> {
    let mut words = Vec::new();
    for token in tokens {
        words.extend(letteral_source_words_for_token(token));
    }
    words
}

#[requires(true)]
#[ensures(!ret.is_empty())]
pub(super) fn letteral_source_words_for_token(token: &Token) -> Vec<String> {
    let mut words = Vec::new();
    letteral_source_words_for_word_like(token.core_word(), &mut words);
    if words.is_empty() {
        words.push(token_text(token));
    }
    words
}

#[requires(true)]
#[ensures(true)]
pub(super) fn letteral_source_words_for_word_like(word_like: &WordLike, out: &mut Vec<String>) {
    match word_like.as_data() {
        data!(WordLike::PlainWord(word)) => out.push(word_text(word)),
        data!(WordLike::LerfuWord { base, bu }) => {
            letteral_source_words_for_word_like(base, out);
            out.push(word_text(bu));
        }
        _ => out.push(word_like.to_string()),
    }
}

#[requires(true)]
#[ensures(true)]
pub(super) fn token_cmavo(token: &Token) -> Option<Cmavo> {
    token.core_word().bare_word().and_then(Word::cmavo)
}

#[requires(true)]
#[ensures(true)]
pub(super) fn letteral_bu_depth(word_like: &WordLike) -> usize {
    match word_like.as_data() {
        data!(WordLike::LerfuWord { base, .. }) => 1 + letteral_bu_depth(base),
        _ => 0,
    }
}

#[requires(true)]
#[ensures(ret.as_ref().is_none_or(|value| !value.is_empty()))]
pub(super) fn basic_letteral_value(source_words: &[String]) -> Option<String> {
    match source_words {
        [word] => basic_letteral_word_value(word).or_else(|| {
            parse_generated_simple_pa_integer(word)
                .filter(|value| (0..=9).contains(value))
                .map(|value| value.to_string())
        }),
        [base, bu] if bu == "bu" => match base.as_str() {
            "ky" => Some("q".to_owned()),
            "vy" => Some("w".to_owned()),
            "y'y" => Some("h".to_owned()),
            "a" | "e" | "i" | "o" | "u" | "y" => Some(base.clone()),
            _ => basic_letteral_word_value(base),
        },
        _ => None,
    }
}

#[requires(true)]
#[ensures(ret.as_ref().is_none_or(|value| !value.is_empty()))]
pub(super) fn basic_letteral_word_value(word: &str) -> Option<String> {
    let value = match word {
        "by" => "b",
        "cy" => "c",
        "dy" => "d",
        "fy" => "f",
        "gy" => "g",
        "jy" => "j",
        "ky" => "k",
        "ly" => "l",
        "my" => "m",
        "ny" => "n",
        "py" => "p",
        "ry" => "r",
        "sy" => "s",
        "ty" => "t",
        "vy" => "v",
        "xy" => "x",
        "zy" => "z",
        "y'y" => "'",
        _ => return None,
    };
    Some(value.to_owned())
}

#[requires(true)]
#[ensures(ret.as_ref().is_none_or(|value| !value.is_empty()))]
pub(super) fn letteral_unit_values_joined(units: &[LetteralUnit]) -> Option<String> {
    let mut value = String::new();
    for unit in units {
        value.push_str(unit.value.as_ref()?);
    }
    Some(value)
}

#[requires(true)]
#[ensures(ret.as_ref().is_none_or(|value| !value.is_empty()))]
pub(super) fn letteral_display_text(units: &[LetteralUnit]) -> Option<String> {
    if units.iter().all(|unit| {
        matches!(unit.kind, LetteralUnitKind::Glyph | LetteralUnitKind::Digit)
            && unit.value.is_some()
    }) {
        letteral_unit_values_joined(units)
    } else {
        None
    }
}

#[requires(!text.is_empty())]
#[ensures(true)]
pub(super) fn quantity_form_for_text(text: &str) -> QuantityForm {
    match text {
        "ro" => QuantityForm::All,
        text if text.starts_with("su'o") => QuantityForm::AtLeast,
        text if text.starts_with("su'e") => QuantityForm::AtMost,
        text if text.starts_with("za'u") => QuantityForm::MoreThan,
        text if text.starts_with("me'i") => QuantityForm::LessThan,
        text if text.starts_with("ji'i") => QuantityForm::Approximate,
        "so'a" => QuantityForm::TooFew,
        "so'e" => QuantityForm::Enough,
        "so'i" | "so'o" | "so'u" => QuantityForm::Indefinite,
        "du'e" => QuantityForm::TooMany,
        _ => QuantityForm::Exact,
    }
}

#[requires(true)]
#[ensures(true)]
pub(super) fn quantity_form_for_value(value: &QuantityValue) -> QuantityForm {
    value
        .text
        .as_deref()
        .map(quantity_form_for_text)
        .unwrap_or(QuantityForm::Exact)
}
