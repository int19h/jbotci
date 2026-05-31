extern crate bityzba;

use std::env;
use std::error::Error;
use std::fs;
use std::path::{Path, PathBuf};

use bityzba::{invariant, requires};
use jbotci_dictionary::import::{
    ImportedDictionary, ImportedDictionaryEntry, ImportedDictionaryUser, ImportedKeyword,
    parse_lensisku_json,
};
use jbotci_dictionary::{
    Dictionary, DictionaryEntry, DictionaryUser, EntryIndex, Keyword, OwnedDictionaryIndexes,
    OwnedRafsiIndexEntry, OwnedSelmahoIndexEntry, OwnedWordIndexEntry, Rafsi, RafsiIndexEntry,
    RafsiIndexTarget, RafsiSource, RawSelmaho, SelmahoIndexEntry, WordIndexEntry, WordType,
    build_owned_indexes,
};
use proc_macro2::{Literal, TokenStream};
use quote::quote;
use serde::Deserialize;
use sha2::{Digest, Sha256};

const VENDORED_DICTIONARY: &str = "../../vendor/lensisku/dictionary-en.json";
const VENDORED_METADATA: &str = "../../vendor/lensisku/dictionary-en.metadata.toml";

#[derive(Debug, Clone, Deserialize)]
#[invariant(true)]
struct DictionaryMetadata {
    language_tag: String,
    language_realname: String,
    format: String,
    filename: String,
    metadata_url: String,
    download_url: String,
    lensisku_created_at: String,
    sha256: String,
    entry_count: usize,
}

#[requires(true)]
#[ensures(true)]
fn main() {
    bityzba::require_contracts().unwrap();
    if let Err(error) = run() {
        panic!("failed to generate embedded dictionary: {error}");
    }
}

#[requires(true)]
#[ensures(true)]
fn run() -> Result<(), Box<dyn Error>> {
    let manifest_dir = PathBuf::from(env::var("CARGO_MANIFEST_DIR")?);
    let dictionary_path = manifest_dir.join(VENDORED_DICTIONARY);
    let metadata_path = manifest_dir.join(VENDORED_METADATA);
    println!("cargo:rerun-if-changed={}", dictionary_path.display());
    println!("cargo:rerun-if-changed={}", metadata_path.display());

    let input = fs::read_to_string(&dictionary_path)?;
    let metadata = load_dictionary_metadata(&metadata_path)?;
    let imported = parse_lensisku_json(&input)?;
    validate_dictionary_metadata(&metadata, &imported, input.as_bytes())?;
    let leaked_entries = leak_entries(&imported);
    let indexes = build_owned_indexes(leaked_entries);
    let word_index = leak_word_index(&indexes.word_index);
    let rafsi_index = leak_rafsi_index(&indexes.rafsi_index);
    let selmaho_index = leak_selmaho_index(&indexes.selmaho_index);
    let dictionary =
        Dictionary::from_static_slices(leaked_entries, word_index, rafsi_index, selmaho_index);
    dictionary.validate()?;

    let generated = render_dictionary(&imported, &indexes, &metadata)?;
    let out_dir = PathBuf::from(env::var("OUT_DIR")?);
    fs::write(out_dir.join("dictionary_en.rs"), generated)?;
    Ok(())
}

#[requires(true)]
#[ensures(!ret.is_empty() || dictionary.entries.is_empty())]
fn leak_entries(dictionary: &ImportedDictionary) -> &'static [DictionaryEntry<'static>] {
    dictionary
        .entries
        .iter()
        .map(leak_entry)
        .collect::<Vec<_>>()
        .leak()
}

#[requires(true)]
#[ensures(!ret.word.is_empty() || entry.word.is_empty())]
fn leak_entry(entry: &ImportedDictionaryEntry) -> DictionaryEntry<'static> {
    DictionaryEntry {
        word: leak_str(&entry.word),
        word_type: entry.word_type,
        definition: leak_str(&entry.definition),
        definition_id: entry.definition_id,
        notes: leak_str(&entry.notes),
        score: entry.score,
        gloss_keywords: leak_keywords(&entry.gloss_keywords),
        place_keywords: leak_keywords(&entry.place_keywords),
        rafsi: leak_rafsi(&entry.rafsi),
        selmaho: entry
            .selmaho
            .as_deref()
            .map(|value| RawSelmaho(leak_str(value))),
        etymology: entry.etymology.as_deref().map(leak_str),
        jargon: entry.jargon.as_deref().map(leak_str),
        user: leak_user(&entry.user),
    }
}

#[requires(true)]
#[ensures(!ret.is_empty() || keywords.is_empty())]
fn leak_keywords(keywords: &[ImportedKeyword]) -> &'static [Keyword<'static>] {
    keywords
        .iter()
        .map(|keyword| Keyword {
            word: leak_str(&keyword.word),
            meaning: keyword.meaning.as_deref().map(leak_str),
        })
        .collect::<Vec<_>>()
        .leak()
}

#[requires(true)]
#[ensures(!ret.is_empty() || rafsi.is_empty())]
fn leak_rafsi(rafsi: &[String]) -> &'static [Rafsi<'static>] {
    rafsi
        .iter()
        .map(|value| Rafsi(leak_str(value)))
        .collect::<Vec<_>>()
        .leak()
}

#[requires(true)]
#[ensures(!ret.username.is_empty() || user.username.is_empty())]
fn leak_user(user: &ImportedDictionaryUser) -> DictionaryUser<'static> {
    DictionaryUser {
        username: leak_str(&user.username),
        realname: user.realname.as_deref().map(leak_str),
    }
}

#[requires(true)]
#[ensures(true)]
fn leak_word_index(index: &[OwnedWordIndexEntry]) -> &'static [WordIndexEntry<'static>] {
    index
        .iter()
        .map(|entry| WordIndexEntry {
            key: leak_str(&entry.key),
            targets: entry.targets.clone().leak(),
        })
        .collect::<Vec<_>>()
        .leak()
}

#[requires(true)]
#[ensures(true)]
fn leak_rafsi_index(index: &[OwnedRafsiIndexEntry]) -> &'static [RafsiIndexEntry<'static>] {
    index
        .iter()
        .map(|entry| RafsiIndexEntry {
            key: leak_str(&entry.key),
            targets: entry.targets.clone().leak(),
        })
        .collect::<Vec<_>>()
        .leak()
}

#[requires(true)]
#[ensures(true)]
fn leak_selmaho_index(index: &[OwnedSelmahoIndexEntry]) -> &'static [SelmahoIndexEntry<'static>] {
    index
        .iter()
        .map(|entry| SelmahoIndexEntry {
            key: leak_str(&entry.key),
            targets: entry.targets.clone().leak(),
        })
        .collect::<Vec<_>>()
        .leak()
}

#[requires(true)]
#[ensures(true)]
fn leak_str(value: &str) -> &'static str {
    Box::leak(value.to_owned().into_boxed_str())
}

#[requires(true)]
#[ensures(ret.as_ref().is_ok_and(|metadata| !metadata.lensisku_created_at.is_empty()))]
fn load_dictionary_metadata(path: &Path) -> Result<DictionaryMetadata, Box<dyn Error>> {
    let input = fs::read_to_string(path)?;
    Ok(toml::from_str(&input)?)
}

#[requires(true)]
#[ensures(true)]
fn validate_dictionary_metadata(
    metadata: &DictionaryMetadata,
    dictionary: &ImportedDictionary,
    dictionary_bytes: &[u8],
) -> Result<(), Box<dyn Error>> {
    if metadata.entry_count != dictionary.entries.len() {
        return Err(format!(
            "metadata entry_count {} does not match {} dictionary entries",
            metadata.entry_count,
            dictionary.entries.len()
        )
        .into());
    }

    let sha256 = sha256_hex(dictionary_bytes);
    if metadata.sha256 != sha256 {
        return Err(format!(
            "metadata sha256 {} does not match dictionary sha256 {sha256}",
            metadata.sha256
        )
        .into());
    }

    Ok(())
}

#[requires(true)]
#[ensures(true)]
fn render_dictionary(
    dictionary: &ImportedDictionary,
    indexes: &OwnedDictionaryIndexes,
    metadata: &DictionaryMetadata,
) -> Result<String, Box<dyn Error>> {
    let entries = dictionary.entries.iter().map(render_entry);
    let word_index = indexes.word_index.iter().map(render_word_index_entry);
    let rafsi_index = indexes.rafsi_index.iter().map(render_rafsi_index_entry);
    let selmaho_index = indexes.selmaho_index.iter().map(render_selmaho_index_entry);
    let rendered_metadata = render_metadata(metadata);

    let tokens = quote! {
        pub static ENTRIES: &[jbotci_dictionary::DictionaryEntry<'static>] = &[
            #(#entries,)*
        ];

        static WORD_INDEX: &[jbotci_dictionary::WordIndexEntry<'static>] = &[
            #(#word_index,)*
        ];

        static RAFSI_INDEX: &[jbotci_dictionary::RafsiIndexEntry<'static>] = &[
            #(#rafsi_index,)*
        ];

        static SELMAHO_INDEX: &[jbotci_dictionary::SelmahoIndexEntry<'static>] = &[
            #(#selmaho_index,)*
        ];

        pub static ENGLISH: jbotci_dictionary::Dictionary<'static> =
            jbotci_dictionary::Dictionary::from_static_slices(
                ENTRIES,
                WORD_INDEX,
                RAFSI_INDEX,
                SELMAHO_INDEX,
            );

        pub static ENGLISH_METADATA: crate::DictionarySnapshotMetadata = #rendered_metadata;
    };

    let syntax = syn::parse2(tokens)?;
    Ok(prettyplease::unparse(&syntax))
}

#[requires(true)]
#[ensures(true)]
fn render_metadata(metadata: &DictionaryMetadata) -> TokenStream {
    let language_tag = string_literal(&metadata.language_tag);
    let language_realname = string_literal(&metadata.language_realname);
    let format = string_literal(&metadata.format);
    let filename = string_literal(&metadata.filename);
    let metadata_url = string_literal(&metadata.metadata_url);
    let download_url = string_literal(&metadata.download_url);
    let lensisku_created_at = string_literal(&metadata.lensisku_created_at);
    let sha256 = string_literal(&metadata.sha256);
    let entry_count = usize_literal(metadata.entry_count);

    quote! {
        crate::DictionarySnapshotMetadata {
            language_tag: #language_tag,
            language_realname: #language_realname,
            format: #format,
            filename: #filename,
            metadata_url: #metadata_url,
            download_url: #download_url,
            lensisku_created_at: #lensisku_created_at,
            sha256: #sha256,
            entry_count: #entry_count,
        }
    }
}

#[requires(true)]
#[ensures(true)]
fn render_entry(entry: &ImportedDictionaryEntry) -> TokenStream {
    let word = string_literal(&entry.word);
    let word_type = render_word_type(entry.word_type);
    let definition = string_literal(&entry.definition);
    let definition_id = u64_literal(entry.definition_id.0);
    let notes = string_literal(&entry.notes);
    let score = f64_literal(entry.score.0);
    let gloss_keywords = entry.gloss_keywords.iter().map(render_keyword);
    let place_keywords = entry.place_keywords.iter().map(render_keyword);
    let rafsi = entry.rafsi.iter().map(|value| {
        let value = string_literal(value);
        quote! { jbotci_dictionary::Rafsi(#value) }
    });
    let selmaho = render_optional_string_newtype(entry.selmaho.as_deref(), "RawSelmaho");
    let etymology = render_optional_str(entry.etymology.as_deref());
    let jargon = render_optional_str(entry.jargon.as_deref());
    let user = render_user(&entry.user);

    quote! {
        jbotci_dictionary::DictionaryEntry {
            word: #word,
            word_type: #word_type,
            definition: #definition,
            definition_id: jbotci_dictionary::DefinitionId(#definition_id),
            notes: #notes,
            score: jbotci_dictionary::Score(#score),
            gloss_keywords: &[#(#gloss_keywords,)*],
            place_keywords: &[#(#place_keywords,)*],
            rafsi: &[#(#rafsi,)*],
            selmaho: #selmaho,
            etymology: #etymology,
            jargon: #jargon,
            user: #user,
        }
    }
}

#[requires(true)]
#[ensures(true)]
fn render_keyword(keyword: &ImportedKeyword) -> TokenStream {
    let word = string_literal(&keyword.word);
    let meaning = render_optional_str(keyword.meaning.as_deref());
    quote! {
        jbotci_dictionary::Keyword {
            word: #word,
            meaning: #meaning,
        }
    }
}

#[requires(true)]
#[ensures(true)]
fn render_user(user: &ImportedDictionaryUser) -> TokenStream {
    let username = string_literal(&user.username);
    let realname = render_optional_str(user.realname.as_deref());
    quote! {
        jbotci_dictionary::DictionaryUser {
            username: #username,
            realname: #realname,
        }
    }
}

#[requires(true)]
#[ensures(true)]
fn render_word_index_entry(entry: &OwnedWordIndexEntry) -> TokenStream {
    let key = string_literal(&entry.key);
    let targets = entry.targets.iter().map(render_entry_index);
    quote! {
        jbotci_dictionary::WordIndexEntry {
            key: #key,
            targets: &[#(#targets,)*],
        }
    }
}

#[requires(true)]
#[ensures(true)]
fn render_rafsi_index_entry(entry: &OwnedRafsiIndexEntry) -> TokenStream {
    let key = string_literal(&entry.key);
    let targets = entry.targets.iter().map(render_rafsi_index_target);
    quote! {
        jbotci_dictionary::RafsiIndexEntry {
            key: #key,
            targets: &[#(#targets,)*],
        }
    }
}

#[requires(true)]
#[ensures(true)]
fn render_selmaho_index_entry(entry: &OwnedSelmahoIndexEntry) -> TokenStream {
    let key = string_literal(&entry.key);
    let targets = entry.targets.iter().map(render_entry_index);
    quote! {
        jbotci_dictionary::SelmahoIndexEntry {
            key: #key,
            targets: &[#(#targets,)*],
        }
    }
}

#[requires(true)]
#[ensures(true)]
fn render_entry_index(index: &EntryIndex) -> TokenStream {
    let value = usize_literal(index.0);
    quote! { jbotci_dictionary::EntryIndex(#value) }
}

#[requires(true)]
#[ensures(true)]
fn render_rafsi_index_target(target: &RafsiIndexTarget) -> TokenStream {
    let entry_index = render_entry_index(&target.entry_index);
    let source = render_rafsi_source(target.source);
    quote! {
        jbotci_dictionary::RafsiIndexTarget {
            entry_index: #entry_index,
            source: #source,
        }
    }
}

#[requires(true)]
#[ensures(true)]
fn render_word_type(word_type: WordType) -> TokenStream {
    match word_type {
        WordType::Gismu => quote! { jbotci_dictionary::WordType::Gismu },
        WordType::ExperimentalGismu => quote! { jbotci_dictionary::WordType::ExperimentalGismu },
        WordType::Lujvo => quote! { jbotci_dictionary::WordType::Lujvo },
        WordType::ZeiLujvo => quote! { jbotci_dictionary::WordType::ZeiLujvo },
        WordType::ObsoleteZeiLujvo => quote! { jbotci_dictionary::WordType::ObsoleteZeiLujvo },
        WordType::Cmavo => quote! { jbotci_dictionary::WordType::Cmavo },
        WordType::ExperimentalCmavo => quote! { jbotci_dictionary::WordType::ExperimentalCmavo },
        WordType::ObsoleteCmavo => quote! { jbotci_dictionary::WordType::ObsoleteCmavo },
        WordType::CmavoCompound => quote! { jbotci_dictionary::WordType::CmavoCompound },
        WordType::Fuivla => quote! { jbotci_dictionary::WordType::Fuivla },
        WordType::ObsoleteFuivla => quote! { jbotci_dictionary::WordType::ObsoleteFuivla },
        WordType::Cmevla => quote! { jbotci_dictionary::WordType::Cmevla },
        WordType::ObsoleteCmevla => quote! { jbotci_dictionary::WordType::ObsoleteCmevla },
        WordType::BuLetteral => quote! { jbotci_dictionary::WordType::BuLetteral },
        WordType::Phrase => quote! { jbotci_dictionary::WordType::Phrase },
    }
}

#[requires(true)]
#[ensures(true)]
fn render_rafsi_source(source: RafsiSource) -> TokenStream {
    match source {
        RafsiSource::Listed => quote! { jbotci_dictionary::RafsiSource::Listed },
        RafsiSource::UniversalShort => quote! { jbotci_dictionary::RafsiSource::UniversalShort },
        RafsiSource::UniversalLong => quote! { jbotci_dictionary::RafsiSource::UniversalLong },
    }
}

#[requires(true)]
#[ensures(true)]
fn render_optional_string_newtype(value: Option<&str>, type_name: &str) -> TokenStream {
    match value {
        Some(value) => {
            let type_name = syn::Ident::new(type_name, proc_macro2::Span::call_site());
            let value = string_literal(value);
            quote! { Some(jbotci_dictionary::#type_name(#value)) }
        }
        None => quote! { None },
    }
}

#[requires(true)]
#[ensures(true)]
fn render_optional_str(value: Option<&str>) -> TokenStream {
    match value {
        Some(value) => {
            let value = string_literal(value);
            quote! { Some(#value) }
        }
        None => quote! { None },
    }
}

#[requires(true)]
#[ensures(true)]
fn string_literal(value: &str) -> Literal {
    Literal::string(value)
}

#[requires(true)]
#[ensures(ret.len() == 64)]
fn sha256_hex(bytes: &[u8]) -> String {
    let digest = Sha256::digest(bytes);
    digest.iter().map(|byte| format!("{byte:02x}")).collect()
}

#[requires(true)]
#[ensures(true)]
fn usize_literal(value: usize) -> Literal {
    Literal::usize_unsuffixed(value)
}

#[requires(true)]
#[ensures(true)]
fn u64_literal(value: u64) -> Literal {
    Literal::u64_unsuffixed(value)
}

#[requires(true)]
#[ensures(true)]
fn f64_literal(value: f64) -> Literal {
    Literal::f64_unsuffixed(value)
}
