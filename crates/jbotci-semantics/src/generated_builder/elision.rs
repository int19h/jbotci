use super::*;

impl<'a, 'dict, 'tree> GeneratedGraphBuilder<'a, 'dict, 'tree> {
    #[requires(formula.object_kind() == crate::model::SemanticObjectKind::Formula)]
    #[requires(crate::model::argument_object_kind_can_fill(argument_object.object_kind()))]
    #[ensures(true)]
    pub(super) fn fill_first_elided_generated_formula_argument_with_object(
        &mut self,
        formula: SemanticObjectId,
        argument_object: SemanticObjectId,
    ) -> Result<(), SemanticsError> {
        self.fill_first_elided_generated_formula_argument_with_argument(
            formula,
            &ArgumentValue::filled(argument_object, None),
        )
    }

    #[requires(argument.value.is_some())]
    #[requires(formula.object_kind() == crate::model::SemanticObjectKind::Formula)]
    #[ensures(true)]
    pub(super) fn fill_first_elided_generated_formula_argument_with_argument(
        &mut self,
        formula: SemanticObjectId,
        argument: &ArgumentValue,
    ) -> Result<(), SemanticsError> {
        let Some(object) = self
            .objects
            .get(&formula)
            .and_then(SemanticObject::formula_traversal)
        else {
            return Ok(());
        };
        let object = object.into_data();
        if let Some(predication) = object.predication
            && self.fill_first_elided_generated_predication_argument_with_argument(
                predication,
                argument,
            )?
        {
            return Ok(());
        }
        for child in object.children {
            self.fill_first_elided_generated_formula_argument_with_argument(child, argument)?;
        }
        if let Some(restriction) = object.restriction {
            self.fill_first_elided_generated_formula_argument_with_argument(restriction, argument)?;
        }
        if let Some(body) = object.body {
            self.fill_first_elided_generated_formula_argument_with_argument(body, argument)?;
        }
        Ok(())
    }

    #[requires(formula.object_kind() == crate::model::SemanticObjectKind::Formula)]
    #[requires(parameter.object_kind() == crate::model::SemanticObjectKind::Parameter)]
    #[ensures(true)]
    pub(super) fn replace_first_elided_generated_formula_argument(
        &mut self,
        formula: SemanticObjectId,
        parameter: SemanticObjectId,
        preferred_selbri: Option<&'tree SelbriSyntax>,
    ) -> Result<bool, SemanticsError> {
        let Some(object) = self
            .objects
            .get(&formula)
            .and_then(SemanticObject::formula_traversal)
        else {
            return Ok(false);
        };
        let object = object.into_data();
        if let Some(predication) = object.predication
            && self.replace_first_elided_generated_predication_argument(
                predication,
                parameter,
                preferred_selbri,
            )?
        {
            return Ok(true);
        }
        for child in object.children {
            if self.replace_first_elided_generated_formula_argument(
                child,
                parameter,
                preferred_selbri,
            )? {
                return Ok(true);
            }
        }
        if let Some(restriction) = object.restriction
            && self.replace_first_elided_generated_formula_argument(
                restriction,
                parameter,
                preferred_selbri,
            )?
        {
            return Ok(true);
        }
        if let Some(body) = object.body
            && self.replace_first_elided_generated_formula_argument(
                body,
                parameter,
                preferred_selbri,
            )?
        {
            return Ok(true);
        }
        Ok(false)
    }

    #[requires(predication.object_kind() == crate::model::SemanticObjectKind::Predication)]
    #[requires(parameter.object_kind() == crate::model::SemanticObjectKind::Parameter)]
    #[ensures(true)]
    pub(super) fn replace_first_elided_generated_predication_argument(
        &mut self,
        predication: SemanticObjectId,
        parameter: SemanticObjectId,
        preferred_selbri: Option<&'tree SelbriSyntax>,
    ) -> Result<bool, SemanticsError> {
        let object = self.objects.get(&predication).ok_or_else(|| {
            invalid_graph(format!(
                "semantic builder could not find abstraction predication {predication}"
            ))
        })?;
        let mut selected_place: Option<(usize, usize, PlaceIndex)> = None;
        let object = object.as_predication().ok_or_else(|| {
            invalid_graph(format!(
                "semantic builder expected {predication} to be a predication"
            ))
        })?;
        for (place, argument) in &object.arguments {
            if argument.kind != ArgumentValueKind::Elided {
                continue;
            }
            let index = argument_place_index(place);
            let visible_rank = preferred_selbri
                .map(|selbri| generated_raw_place_visible_rank_for_selbri(selbri, index))
                .transpose()?
                .unwrap_or(index);
            if selected_place
                .as_ref()
                .is_none_or(|(best_visible, best_index, _)| {
                    (visible_rank, index) < (*best_visible, *best_index)
                })
            {
                selected_place = Some((visible_rank, index, *place));
            }
        }
        let Some((_visible_rank, _index, place)) = selected_place else {
            return Ok(false);
        };
        let object = self.objects.get_mut(&predication).ok_or_else(|| {
            invalid_graph(format!(
                "semantic builder could not find abstraction predication {predication}"
            ))
        })?;
        object.update_predication(|object| {
            let data = object.into_data();
            let mut arguments = data.arguments;
            if let Some(argument) = arguments.get(&place) {
                let source = argument.source.clone();
                arguments.insert(place, ArgumentValue::filled(parameter, source));
            }
            PredicationNode::from_data(data!(PredicationNode {
                arguments: arguments,
                ..data
            }))
        });
        Ok(true)
    }

    #[requires(predication.object_kind() == crate::model::SemanticObjectKind::Predication)]
    #[requires(argument.value.is_some())]
    #[ensures(ret.as_ref().is_ok_and(|filled| *filled || !*filled) || ret.is_err())]
    pub(super) fn fill_first_elided_generated_predication_argument_with_argument(
        &mut self,
        predication: SemanticObjectId,
        argument: &ArgumentValue,
    ) -> Result<bool, SemanticsError> {
        let object = self.objects.get_mut(&predication).ok_or_else(|| {
            invalid_graph(format!(
                "semantic builder could not find shared-head predication {predication}"
            ))
        })?;
        let object_data = object.as_predication().ok_or_else(|| {
            invalid_graph(format!(
                "semantic builder expected {predication} to be a predication"
            ))
        })?;
        let Some(place) = object_data
            .arguments
            .iter()
            .filter(|(_place, argument)| argument.kind == ArgumentValueKind::Elided)
            .map(|(place, _argument)| (argument_place_index(place), place))
            .min_by_key(|(index, _place)| *index)
            .map(|(_index, place)| *place)
        else {
            return Ok(false);
        };
        let argument = argument.clone();
        object.update_predication(|object| {
            let data = object.into_data();
            let mut arguments = data.arguments;
            arguments.insert(place, argument);
            PredicationNode::from_data(data!(PredicationNode {
                arguments: arguments,
                ..data
            }))
        });
        Ok(true)
    }
}
