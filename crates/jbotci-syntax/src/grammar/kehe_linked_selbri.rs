//! Model mapping for the Zantufa KEhE-linked selbri owner (#834).

use bityzba::requires;

use super::generated_model::recovered;

// The owner's first reach is from `zantufa_selbri_entry`, ahead of the atom priority arm, where it
// must yield a whole selbri. Its model position is `untagged_selbri`'s variant, so it maps there.
impl From<super::generated_model::ZantufaKeheLinkedSelbriSyntax>
    for super::generated_model::SelbriSyntax
{
    #[requires(true)]
    #[bityzba::ensures(true)]
    fn from(value: super::generated_model::ZantufaKeheLinkedSelbriSyntax) -> Self {
        Self::UntaggedSelbri(std::sync::Arc::new(
            super::generated_model::UntaggedSelbriSyntax::ZantufaKeheLinkedSelbri(
                std::sync::Arc::new(value),
            ),
        ))
    }
}

#[bityzba::contract_trait]
impl super::generated_runtime::GrammarMapTo<recovered::Recovered<recovered::SelbriSyntax>>
    for recovered::Recovered<recovered::ZantufaKeheLinkedSelbriSyntax>
{
    fn grammar_map_to(self) -> recovered::Recovered<recovered::SelbriSyntax> {
        recovered::Recovered::valid(recovered::SelbriSyntax::UntaggedSelbri(
            std::sync::Arc::new(recovered::Recovered::valid(
                recovered::UntaggedSelbriSyntax::ZantufaKeheLinkedSelbri(std::sync::Arc::new(self)),
            )),
        ))
    }
}
