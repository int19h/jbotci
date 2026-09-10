//! The interaction endpoint: what Discord sends, and what jbotci does about it.
//!
//! Four kinds arrive at one URL and are handled as four different things: a
//! ping, a slash command, the ⚙️ button, and a form submission. Two deadlines
//! shape everything here. Discord drops an interaction that is not answered
//! within three seconds, and a modal can only be opened by that immediate
//! answer, never after a deferred one, so opening a form does its own work
//! inline while running a tool defers first and edits the message afterwards.
//!
//! What a message says is the state: the ⚙️ button's identifier and the input
//! block beside it rebuild the request, so a form opens correctly after every
//! cache is gone and the process has restarted. Message revisions, per-message
//! locks and reading the published message before writing it are what keep a
//! slow submission from overwriting a newer one.

use std::sync::Arc;

#[allow(unused_imports)]
use bityzba::{data, ensures, invariant, new, requires};
use serde_json::Value;
use tokio::time::{Duration, Instant};

use super::assemble::{AssembleError, AssembledMessage, assemble};
use super::codec::{
    HeaderDecodeError, INPUT_ATTACHMENT_FILENAME, INPUT_COMPONENT_ID, ModalHeader, PageControl,
    RequestHeader, decode_input_block,
};
use super::components::{
    InteractionResponse, MAX_CONTENT_UNITS, MessageComponent, MessagePayload, PayloadError,
    TYPE_BUTTON, TextDisplay, bound_content,
};
use super::dedupe::{Admission, DeliveryTicket, RecentInteractions};
use super::diagram::DiagramLimits;
use super::links::{AppLinkTooLarge, app_link};
use super::locks::{MessageGuard, MessageLocks};
use super::modal::{Submission, SubmissionError, build as build_modal, parse_submission};
use super::operations::{
    OperationContext, OperationError, RequestValidationError, ToolOutcome, run_request,
};
use super::present::{markdown, render, render_failure, render_validation_error};
use super::request::{BuildTag, PublishedRequest, Revision, Snowflake};
use super::schema::{CommandDecodeError, decode_command};
use super::transport::{DiscordApi, InteractionToken, TransportError};
use super::work::{WorkGovernor, WorkKeepalive};
use crate::ToolServices;

/// How long one interaction's work may take before the handler gives up. The
/// message can still be edited for fifteen minutes, but a request nobody is
/// waiting for should not hold a worker.
const HANDLER_BUDGET: Duration = Duration::from_secs(20);
/// How long opening a form may take. Discord drops an interaction that is not
/// answered within three seconds, and only an immediate answer can be a form.
const MODAL_BUDGET: Duration = Duration::from_millis(2200);
/// How long finishing an interaction may take once its work is over. Saying
/// what happened is not the work, so it is not paid for out of the work's
/// budget: a request that spent every second it had must still be able to
/// report that. The interaction's token stays valid for fifteen minutes, so
/// this only bounds how long the network lane is held for it.
const SETTLE_BUDGET: Duration = Duration::from_secs(5);
/// Most bytes a source attachment may carry back into a form.
const SOURCE_ATTACHMENT_CAP: usize = 1 << 20;
/// How long to wait for Discord to create the message its acknowledgement
/// promised, and how often to look.
const ORIGINAL_CREATION_BUDGET: Duration = Duration::from_secs(5);
const ORIGINAL_RETRY_PAUSE: Duration = Duration::from_millis(150);
/// Interactions remembered for duplicate delivery.
const RECENT_INTERACTIONS: usize = 512;
/// Waiters allowed per message, and messages tracked at once.
const LOCKED_MESSAGES: usize = 256;
const WAITERS_PER_MESSAGE: usize = 4;

/// Where this deployment's Discord app lives.
#[invariant(!public_key.is_empty())]
#[invariant(!api_base.is_empty() && !public_base_url.is_empty())]
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct DiscordConfig {
    /// The application's Ed25519 public key, hex, for request signatures.
    pub(crate) public_key: String,
    /// The Discord API root, so tests can point at their own server.
    pub(crate) api_base: String,
    /// The public root of the web app, for "Open in app" links.
    pub(crate) public_base_url: String,
}

/// Everything one interaction needs, shared by every handler.
#[invariant(true)]
#[derive(Debug)]
pub(crate) struct DiscordService {
    config: DiscordConfig,
    tools: ToolServices,
    api: DiscordApi,
    governor: Arc<WorkGovernor>,
    locks: Arc<MessageLocks>,
    recent: Arc<RecentInteractions>,
    build_tag: BuildTag,
}

impl DiscordService {
    #[requires(true)]
    #[ensures(true)]
    pub(crate) fn new(config: DiscordConfig, tools: ToolServices) -> Self {
        let api = DiscordApi::new(&config.api_base);
        Self {
            config,
            tools,
            api,
            governor: Arc::new(WorkGovernor::new(Default::default())),
            locks: Arc::new(MessageLocks::new(LOCKED_MESSAGES, WAITERS_PER_MESSAGE)),
            recent: Arc::new(RecentInteractions::new(RECENT_INTERACTIONS)),
            build_tag: BuildTag::current(),
        }
    }

    #[requires(true)]
    #[ensures(true)]
    pub(crate) fn public_key(&self) -> &str {
        &self.config.public_key
    }

    /// Answer one interaction. The returned response is what Discord reads
    /// immediately; work that outlives it continues in a spawned task under
    /// the same delivery ticket.
    #[requires(true)]
    #[ensures(true)]
    pub(crate) async fn handle(self: &Arc<Self>, body: &Value) -> InteractionResponse {
        let Some(interaction) = Interaction::read(body) else {
            return InteractionResponse::ephemeral(
                "This interaction is not one jbotci understands.",
            );
        };
        match interaction {
            Interaction::Ping => InteractionResponse::Pong,
            Interaction::Command(command) => self.handle_command(command).await,
            Interaction::Component(component) => self.handle_component(component).await,
            Interaction::ModalSubmit(submit) => self.handle_modal_submit(submit).await,
        }
    }

    /// A slash command publishes a result: acknowledge first, then compute and
    /// edit the message the acknowledgement created.
    #[requires(true)]
    #[ensures(true)]
    async fn handle_command(self: &Arc<Self>, command: CommandInteraction) -> InteractionResponse {
        let ticket = match self.recent.admit(&command.id) {
            Admission::First(ticket) => ticket,
            // A redelivery of a command already being handled: acknowledge it
            // the same way and let the first delivery finish the work.
            Admission::Duplicate => return InteractionResponse::DeferredChannelMessage,
            Admission::AtCapacity => {
                return InteractionResponse::ephemeral(
                    "jbotci is handling as much as it can right now; try again in a moment.",
                );
            }
        };
        let request = match decode_command(&command.data) {
            Ok(request) => request,
            Err(error) => return ephemeral_about("Not run.", &command_error_text(&error)),
        };
        // A request whose form could not offer its own link is refused before
        // anything is published, so every published result can reopen.
        if let Err(error) = app_link(&request, &self.config.public_base_url) {
            return ephemeral_about("Not run.", &error.to_string());
        }
        let published = PublishedRequest {
            request,
            revision: Revision::INITIAL,
            initiator: command.actor.clone(),
            build_tag: self.build_tag.clone(),
        };
        let service = Arc::clone(self);
        let ticket = Arc::new(ticket);
        tokio::spawn(async move {
            service.publish_command(command, published, ticket).await;
        });
        InteractionResponse::DeferredChannelMessage
    }

    /// Compute and publish a command's first result. Discord creates the
    /// message its acknowledgement promised a moment later, so this waits for
    /// it; and a replayed command whose message already carries a result
    /// leaves it alone, because the reader may have edited it since and
    /// recomputing would put the defaults back.
    #[requires(true)]
    #[ensures(true)]
    async fn publish_command(
        &self,
        command: CommandInteraction,
        published: PublishedRequest,
        ticket: Arc<DeliveryTicket>,
    ) {
        let deadline = Instant::now() + HANDLER_BUDGET;
        let keepalive: WorkKeepalive = ticket.clone();
        let target = Target {
            application_id: command.application_id.clone(),
            token: command.token.clone(),
        };
        match self
            .await_original(&target, deadline, Some(keepalive.clone()))
            .await
        {
            Ok(message) => {
                if RequestHeader::of_message(&message).is_some() {
                    // Already published, and possibly edited since.
                    return;
                }
            }
            Err(error) => {
                // Whether a result is already published is now unknown, and a
                // first result written over an edited one would lose the
                // reader's work. Publishing on a guess is the worse outcome,
                // so this says what happened and writes nothing.
                self.report_privately(
                    &target,
                    "jbotci could not read its own message back, so nothing was published.",
                    Some(&error.to_string()),
                    Some(keepalive.clone()),
                )
                .await;
                return;
            }
        }
        // A slash command that cannot run yet is still a result: it carries
        // its ⚙️ form so the reader can complete it.
        let message = match self
            .run_for_publication(
                &published,
                deadline,
                Some(keepalive.clone()),
                command.attachment_limit,
            )
            .await
        {
            Ok(message) => message,
            Err(error) => {
                // The acknowledgement already promised a message. A private
                // note beside an indicator that never resolves would leave
                // the reader watching nothing, so the failure becomes the
                // message instead.
                self.publish_failure(
                    &target,
                    &published,
                    &error,
                    command.attachment_limit,
                    Some(keepalive),
                )
                .await;
                return;
            }
        };
        self.publish_first(&target, &message.payload, deadline, Some(keepalive))
            .await;
    }

    /// Write a first message, once and only once. Every path that publishes
    /// one reads the message immediately beforehand: a replay that saw a
    /// pending message a moment ago must not overwrite a result published in
    /// the meantime, and a read that fails settles nothing either way, so an
    /// unknown state is a reason to write nothing rather than to write over
    /// it. Failures publish through here for the same reason results do.
    #[requires(true)]
    #[ensures(true)]
    async fn publish_first(
        &self,
        target: &Target,
        payload: &MessagePayload,
        deadline: Instant,
        keepalive: Option<WorkKeepalive>,
    ) {
        match self
            .read_original(target, deadline, keepalive.clone())
            .await
        {
            Ok(current) if RequestHeader::of_message(&current).is_some() => return,
            Ok(_) => {}
            Err(error) => {
                self.report_privately(
                    target,
                    "jbotci could not check the message before publishing, so nothing was written.",
                    Some(&error.to_string()),
                    keepalive,
                )
                .await;
                return;
            }
        }
        self.write_and_confirm(target, payload, deadline, keepalive)
            .await;
    }

    /// Finish a first publication whose result could not be produced. Where
    /// the request still travels in a message, the failure carries its ⚙️
    /// form and can be corrected there; where it does not, the message says
    /// so and carries no form, because a form that cannot rebuild its request
    /// would promise more than it can keep.
    #[requires(true)]
    #[ensures(true)]
    async fn publish_failure(
        &self,
        target: &Target,
        published: &PublishedRequest,
        error: &ResultError,
        attachment_limit: Option<u64>,
        keepalive: Option<WorkKeepalive>,
    ) {
        let deadline = Instant::now() + SETTLE_BUDGET;
        let reason = error.to_string();
        // A result is editable only where its ⚙️ form could reopen it. A
        // request whose "Open in app" link does not fit was refused admission
        // before publication, so it does not become an editable result whose
        // form would have to offer a link that cannot exist: it is a refusal,
        // said once, with nothing to change here.
        let assembled = match error {
            ResultError::Link(_) => None,
            _ => assemble(
                &render_failure(&reason, &published.request),
                published,
                self.attachment_limit(attachment_limit),
            )
            .ok(),
        };
        if let Some(message) = assembled {
            self.publish_first(target, &message.payload, deadline, keepalive)
                .await;
            return;
        }
        let text = format!(
            "**Not run:** {}\n{}",
            markdown::escape(&reason),
            markdown::subtext(
                "jbotci publishes only a result its ⚙️ form can reopen, so nothing was published here. Run the command again with a smaller request."
            )
        );
        match text_only_payload(&text) {
            Ok(payload) => {
                self.publish_first(target, &payload, deadline, keepalive)
                    .await;
            }
            Err(_) => {
                self.report_privately(target, "Nothing was published.", Some(&reason), keepalive)
                    .await;
            }
        }
    }

    /// The ⚙️ button opens the form. This answer must be immediate, so the
    /// state is read from the message Discord sent with the interaction, and
    /// only a source that travelled as a file costs a request.
    #[requires(true)]
    #[ensures(true)]
    async fn handle_component(
        self: &Arc<Self>,
        component: ComponentInteraction,
    ) -> InteractionResponse {
        // Two controls sit on a result, and they are told apart by what they
        // are rather than by where they were: the gear opens the form, a page
        // button asks for a page. An identifier that says it is a page button
        // and then is not one belongs to neither, and is refused where it is
        // read rather than tried on the other reader.
        match PageControl::decode(&component.custom_id) {
            Ok(control) => return self.handle_page(component, control).await,
            Err(HeaderDecodeError::UnsupportedSchema { .. }) => {}
            Err(error) => {
                return ephemeral_about(
                    "That page button is not one this build wrote.",
                    &error.to_string(),
                );
            }
        }
        let ticket = match self.recent.admit(&component.id) {
            Admission::First(ticket) => ticket,
            Admission::Duplicate => {
                return InteractionResponse::ephemeral(
                    "That form is already opening; try the button again in a moment.",
                );
            }
            Admission::AtCapacity => {
                return InteractionResponse::ephemeral(
                    "jbotci is handling as much as it can right now; try the ⚙️ button again in a moment.",
                );
            }
        };
        let deadline = Instant::now() + MODAL_BUDGET;
        let keepalive: WorkKeepalive = Arc::new(ticket);
        let published = match self
            .published_state(&component.message, deadline, Some(keepalive.clone()))
            .await
        {
            Ok(published) => published,
            Err(error) => return ephemeral_about("The form did not open.", &error.to_string()),
        };
        // The link is part of the form. A published state whose link no
        // longer fits (this deployment's address changed since) still opens,
        // saying so rather than refusing to reopen at all.
        let link = match app_link(&published.request, &self.config.public_base_url) {
            Ok(link) => link,
            Err(error) => Some(format!("-# Open in app is unavailable: {error}")),
        };
        match build_modal(&published, link.as_deref()) {
            Ok(modal) => InteractionResponse::Modal(modal),
            Err(error) => ephemeral_about("jbotci could not open the form.", &error.to_string()),
        }
    }

    /// Whether `message` offers `custom_id` as something to press: a button
    /// with that identifier, in the message's components, which is not greyed
    /// out. A greyed button is drawn so the reader can see the direction
    /// exists, and Discord will not send a click for it; one that arrives
    /// anyway asks for what the message does not offer. Only buttons count,
    /// and only where components live: an identifier that appears elsewhere
    /// in the message — on a select, or in whatever else a message carries —
    /// is not a button of it.
    #[requires(true)]
    #[ensures(!custom_id.is_empty() || !ret)]
    fn message_offers(message: &Value, custom_id: &str) -> bool {
        /// Walk what holds components: a row or a container holds more of
        /// them, and a section holds one beside its text.
        #[requires(!custom_id.is_empty())]
        #[ensures(true)]
        fn walk(component: &Value, custom_id: &str) -> bool {
            if component.get("type").and_then(Value::as_u64) == Some(u64::from(TYPE_BUTTON))
                && component.get("custom_id").and_then(Value::as_str) == Some(custom_id)
            {
                return !component
                    .get("disabled")
                    .and_then(Value::as_bool)
                    .unwrap_or(false);
            }
            let held = component
                .get("components")
                .and_then(Value::as_array)
                .map(|items| items.iter().any(|item| walk(item, custom_id)))
                .unwrap_or(false);
            held || component
                .get("accessory")
                .is_some_and(|accessory| walk(accessory, custom_id))
        }
        !custom_id.is_empty() && walk(message, custom_id)
    }

    /// A page button turns one page of the message it sits on.
    ///
    /// The button says only which page and which revision it was drawn for.
    /// Everything else, the settings, the source and who published it, is read
    /// from the message when the click arrives, so a control that was forged
    /// or kept from an older message can ask for a page and change nothing
    /// else. A button from a revision the message has moved past is refused.
    #[requires(true)]
    #[ensures(true)]
    async fn handle_page(
        self: &Arc<Self>,
        component: ComponentInteraction,
        control: PageControl,
    ) -> InteractionResponse {
        let ticket = match self.recent.admit(&component.id) {
            Admission::First(ticket) => ticket,
            // A second click of the same button is the same turn.
            Admission::Duplicate => return InteractionResponse::DeferredUpdateMessage,
            Admission::AtCapacity => {
                return InteractionResponse::ephemeral(
                    "jbotci is handling as much as it can right now; try the page again in a moment.",
                );
            }
        };
        let deadline = Instant::now() + MODAL_BUDGET;
        let ticket = Arc::new(ticket);
        let keepalive: WorkKeepalive = ticket.clone();
        let published = match self
            .published_state(&component.message, deadline, Some(keepalive))
            .await
        {
            Ok(published) => published,
            Err(error) => return ephemeral_about("That page did not open.", &error.to_string()),
        };
        // Only the reader who asked may change what everyone else sees, and
        // the message the click carries is what says who that is.
        if component.actor != published.initiator {
            return InteractionResponse::ephemeral(
                "Only the person who ran this command can turn its pages. Run your own /jbotci to page through your own results.",
            );
        }
        if published.revision != control.from_revision {
            return InteractionResponse::ephemeral(
                "This result changed after that page button was drawn. Use the buttons on the result as it stands now.",
            );
        }
        // What the message offers is what may be pressed. This is the
        // control itself, greyed or missing; the checks below are about the
        // page it asks for.
        if !Self::message_offers(&component.message, &component.custom_id) {
            return InteractionResponse::ephemeral(
                "That page button is not one this result offers. Use the buttons on the result as it stands now.",
            );
        }
        let current = published.request.page();
        if current == control.target {
            // Both of the message's buttons carry a page, and the one that
            // cannot go anywhere carries the page it is already on; a click
            // on it says nothing to do.
            return InteractionResponse::ephemeral("That is the page you are on.");
        }
        // The message offers one step in each direction, and that is all a
        // click may ask for. A well-formed identifier for some other page was
        // not drawn on this message, whatever else is true of it.
        if ![current.previous(), current.next()]
            .into_iter()
            .flatten()
            .any(|offered| offered == control.target)
        {
            return InteractionResponse::ephemeral(
                "That page button is not one of this result's. Use the buttons on the result as it stands now.",
            );
        }
        let Some(request) = published.request.clone().with_page(control.target) else {
            return InteractionResponse::ephemeral("This result does not have pages to turn.");
        };
        let Some(revision) = published.revision.next() else {
            return InteractionResponse::ephemeral(
                "This result has been changed as many times as jbotci records. Run the command again to start over.",
            );
        };
        let next = PublishedRequest {
            request,
            revision,
            initiator: published.initiator.clone(),
            build_tag: self.build_tag.clone(),
        };
        let service = Arc::clone(self);
        tokio::spawn(async move {
            service
                .apply_page(component, next, published.revision, ticket)
                .await;
        });
        InteractionResponse::DeferredUpdateMessage
    }

    /// Compute the requested page and write it, under the message's lock and
    /// against the revision the button was drawn for. A page that cannot be
    /// produced changes nothing: the reader keeps the page they were on.
    #[requires(true)]
    #[ensures(true)]
    async fn apply_page(
        &self,
        component: ComponentInteraction,
        next: PublishedRequest,
        opened_from: Revision,
        ticket: Arc<DeliveryTicket>,
    ) {
        let deadline = Instant::now() + HANDLER_BUDGET;
        let keepalive: WorkKeepalive = ticket.clone();
        let target = Target {
            application_id: component.application_id.clone(),
            token: component.token.clone(),
        };
        let Ok(guard) = self.locks.acquire(&component.message_id, deadline).await else {
            self.report_privately(
                &target,
                "jbotci is already changing this result; try the page again in a moment.",
                None,
                Some(keepalive),
            )
            .await;
            return;
        };
        // The lock belongs to the work that writes, as it does for a form.
        let keepalive: WorkKeepalive = Arc::new(Retained {
            _ticket: ticket,
            _guard: guard,
        });
        // The message as Discord holds it now is the authority on both the
        // revision and who may change it.
        match self
            .read_original(&target, deadline, Some(keepalive.clone()))
            .await
        {
            Ok(message) => match RequestHeader::of_message(&message) {
                Some(header) if header.revision != opened_from => {
                    self.report_privately(
                        &target,
                        "This result changed while that page was loading, so nothing was turned. Use the buttons as they stand now.",
                        None,
                        Some(keepalive.clone()),
                    )
                    .await;
                    return;
                }
                Some(header) if header.initiator != component.actor => {
                    self.report_privately(
                        &target,
                        "Only the person who ran this command can turn its pages, so nothing was changed.",
                        None,
                        Some(keepalive.clone()),
                    )
                    .await;
                    return;
                }
                // The message as it stands has to offer the control that was
                // pressed, and it has to be a step from the page it is
                // showing. The revision check above already implies both,
                // since turning a page raises the revision; these ask the
                // message itself rather than relying on that.
                Some(header)
                    if !Self::message_offers(&message, &component.custom_id)
                        || ![header.page.previous(), header.page.next()]
                            .into_iter()
                            .flatten()
                            .any(|offered| offered == next.request.page()) =>
                {
                    self.report_privately(
                        &target,
                        "That page button is not one of this result's, so nothing was turned.",
                        None,
                        Some(keepalive.clone()),
                    )
                    .await;
                    return;
                }
                Some(_) => {}
                None => {
                    self.report_privately(
                        &target,
                        "jbotci could not find its own settings on that message, so no page was turned.",
                        None,
                        Some(keepalive.clone()),
                    )
                    .await;
                    return;
                }
            },
            Err(error) => {
                self.report_privately(
                    &target,
                    "jbotci could not read the result before turning the page.",
                    Some(&error.to_string()),
                    Some(keepalive.clone()),
                )
                .await;
                return;
            }
        }
        let message = match self
            .run_for_edit(
                &next,
                deadline,
                Some(keepalive.clone()),
                component.attachment_limit,
                false,
            )
            .await
        {
            Ok(message) => message,
            Err(error) => {
                self.report_privately(
                    &target,
                    "The page was not turned.",
                    Some(&error.to_string()),
                    Some(keepalive.clone()),
                )
                .await;
                return;
            }
        };
        self.write_and_confirm(&target, &message.payload, deadline, Some(keepalive))
            .await;
    }

    /// A submitted form applies to the message it was opened from.
    #[requires(true)]
    #[ensures(true)]
    async fn handle_modal_submit(
        self: &Arc<Self>,
        submit: ModalSubmitInteraction,
    ) -> InteractionResponse {
        let ticket = match self.recent.admit(&submit.id) {
            Admission::First(ticket) => ticket,
            Admission::Duplicate => return InteractionResponse::DeferredUpdateMessage,
            Admission::AtCapacity => {
                return InteractionResponse::ephemeral(
                    "jbotci is handling as much as it can right now; try again in a moment.",
                );
            }
        };
        let header = match ModalHeader::decode(&submit.custom_id) {
            Ok(header) => header,
            Err(_) => {
                return InteractionResponse::ephemeral(
                    "That form came from an older version of jbotci. Open it again with the ⚙️ button.",
                );
            }
        };
        // Anyone may read a result and open its form, but only the reader who
        // asked for it may change what everyone else sees.
        if submit.actor != header.initiator {
            return InteractionResponse::ephemeral(
                "Only the person who ran this command can change it. Run your own /jbotci to work with these settings.",
            );
        }
        let deadline = Instant::now() + MODAL_BUDGET;
        let ticket = Arc::new(ticket);
        let keepalive: WorkKeepalive = ticket.clone();
        let published = match self
            .published_state(&submit.message, deadline, Some(keepalive.clone()))
            .await
        {
            Ok(published) => published,
            Err(error) => return ephemeral_about("Nothing was applied.", &error.to_string()),
        };
        if published.revision != header.revision {
            return InteractionResponse::ephemeral(
                "This result changed while the form was open. Open the ⚙️ form again to work from what it shows now.",
            );
        }
        // The form's identifier and the message both name who asked, and both
        // come back from Discord. They must agree with each other and with
        // whoever is submitting: the message is the published one, so it has
        // the last word.
        if submit.actor != published.initiator {
            return InteractionResponse::ephemeral(
                "Only the person who ran this command can change it. Run your own /jbotci to work with these settings.",
            );
        }
        let submission = match Submission::read(&submit.data) {
            Ok(submission) => submission,
            Err(error) => {
                return ephemeral_about("Nothing was applied.", &submission_error_text(&error));
            }
        };
        let request = match parse_submission(&published.request, &submission) {
            Ok(request) => request,
            Err(error) => {
                return ephemeral_about("Nothing was applied.", &submission_error_text(&error));
            }
        };
        if let Err(error) = app_link(&request, &self.config.public_base_url) {
            return ephemeral_about(
                "Not applied, and the result is unchanged.",
                &error.to_string(),
            );
        }
        let Some(revision) = published.revision.next() else {
            return InteractionResponse::ephemeral(
                "This result has been edited as many times as jbotci records. Run the command again to start over.",
            );
        };
        // The result being replaced was made by whatever build published it;
        // this one is made by the running build. When those differ the reader
        // is told, because the text and the image they are looking at are the
        // work of a different version of jbotci than the one they saw.
        let rebuilt_by_another_build = published.build_tag != self.build_tag;
        let next = PublishedRequest {
            request,
            revision,
            initiator: published.initiator.clone(),
            build_tag: self.build_tag.clone(),
        };
        let service = Arc::clone(self);
        tokio::spawn(async move {
            service
                .apply_submission(
                    submit,
                    next,
                    header.revision,
                    rebuilt_by_another_build,
                    ticket,
                )
                .await;
        });
        InteractionResponse::DeferredUpdateMessage
    }

    /// Compute the edited result and write it, under the message's lock and
    /// against the revision the form was opened from. A submission that
    /// cannot run changes nothing: the reader is told privately, and the
    /// message keeps its result, its attachments and its revision.
    #[requires(true)]
    #[ensures(true)]
    async fn apply_submission(
        &self,
        submit: ModalSubmitInteraction,
        next: PublishedRequest,
        opened_from: Revision,
        rebuilt_by_another_build: bool,
        ticket: Arc<DeliveryTicket>,
    ) {
        let deadline = Instant::now() + HANDLER_BUDGET;
        // Until the lock is held, the delivery ticket alone stands for this
        // work; reporting that the lock could not be taken needs nothing more.
        let keepalive: WorkKeepalive = ticket.clone();
        let target = Target {
            application_id: submit.application_id.clone(),
            token: submit.token.clone(),
        };
        let Ok(guard) = self.locks.acquire(&submit.message_id, deadline).await else {
            self.report_privately(
                &target,
                "jbotci is already changing this result; try again in a moment.",
                None,
                Some(keepalive.clone()),
            )
            .await;
            return;
        };
        // From here the lock belongs to the work rather than to whoever is
        // waiting for it. A write whose caller stopped waiting is still a
        // write: releasing the message then would let a second submission
        // read the old revision and publish, only for the first write to land
        // afterwards and overwrite it. Carried in the keepalive, the lock is
        // held for exactly as long as the work that writes.
        let keepalive: WorkKeepalive = Arc::new(Retained {
            _ticket: ticket,
            _guard: guard,
        });
        // The message is read again under the lock: an in-memory revision is
        // not a durable one, and another submission may have landed first.
        match self
            .read_original(&target, deadline, Some(keepalive.clone()))
            .await
        {
            Ok(message) => match RequestHeader::of_message(&message) {
                Some(header) if header.revision != opened_from => {
                    self.report_privately(
                        &target,
                        "This result changed while your form was open, so nothing was applied. Open the ⚙️ form again.",
                        None,
                        Some(keepalive.clone()),
                    )
                    .await;
                    return;
                }
                // Checked again here against the message as Discord holds
                // it now, which is the only authority on who published it.
                Some(header) if header.initiator != submit.actor => {
                    self.report_privately(
                        &target,
                        "Only the person who ran this command can change it, so nothing was applied.",
                        None,
                        Some(keepalive.clone()),
                    )
                    .await;
                    return;
                }
                Some(_) => {}
                None => {
                    self.report_privately(
                        &target,
                        "jbotci could not find its own settings on that message, so nothing was applied.",
                        None,
                        Some(keepalive.clone()),
                    )
                    .await;
                    return;
                }
            },
            Err(error) => {
                self.report_privately(
                    &target,
                    "jbotci could not read the result before changing it, so nothing was applied.",
                    Some(&error.to_string()),
                    Some(keepalive.clone()),
                )
                .await;
                return;
            }
        }
        let message = match self
            .run_for_edit(
                &next,
                deadline,
                Some(keepalive.clone()),
                submit.attachment_limit,
                rebuilt_by_another_build,
            )
            .await
        {
            Ok(message) => message,
            Err(error) => {
                self.report_privately(
                    &target,
                    "Nothing was changed.",
                    Some(&error.to_string()),
                    Some(keepalive.clone()),
                )
                .await;
                return;
            }
        };
        self.write_and_confirm(&target, &message.payload, deadline, Some(keepalive))
            .await;
    }

    /// Run a request for publication: what cannot run yet is still a result,
    /// carrying its ⚙️ form so the reader can complete or correct it.
    #[requires(true)]
    #[ensures(true)]
    async fn run_for_publication(
        &self,
        published: &PublishedRequest,
        deadline: Instant,
        keepalive: Option<WorkKeepalive>,
        attachment_limit: Option<u64>,
    ) -> Result<AssembledMessage, ResultError> {
        // A result whose form could not carry its link is refused before
        // anything is published, so every published result of a tool with a
        // page has an exact one to reopen it with.
        app_link(&published.request, &self.config.public_base_url).map_err(ResultError::Link)?;
        let rendered = match self
            .run_tool(published, deadline, keepalive, attachment_limit)
            .await
        {
            Ok(outcome) => render(&outcome, &published.request),
            Err(RunError::Invalid(error)) => render_validation_error(&error, &published.request),
            Err(RunError::Failed(error)) => return Err(ResultError::Operation(error)),
        };
        assemble(
            &rendered,
            published,
            self.attachment_limit(attachment_limit),
        )
        .map_err(ResultError::Assemble)
    }

    /// Run a request for an edit. A request that cannot run replaces nothing:
    /// the message it would replace still holds the result the reader asked
    /// for, and losing that to a typo would be worse than not applying it.
    #[requires(true)]
    #[ensures(true)]
    async fn run_for_edit(
        &self,
        published: &PublishedRequest,
        deadline: Instant,
        keepalive: Option<WorkKeepalive>,
        attachment_limit: Option<u64>,
        rebuilt_by_another_build: bool,
    ) -> Result<AssembledMessage, ResultError> {
        app_link(&published.request, &self.config.public_base_url).map_err(ResultError::Link)?;
        let outcome = self
            .run_tool(published, deadline, keepalive, attachment_limit)
            .await
            .map_err(|error| match error {
                RunError::Invalid(error) => ResultError::Invalid(error),
                RunError::Failed(error) => ResultError::Operation(error),
            })?;
        let mut rendered = render(&outcome, &published.request);
        if rebuilt_by_another_build {
            // Only when the versions actually differ: a note on every ordinary
            // edit would be noise, and this one carries information.
            let note = markdown::subtext(
                "Recomputed by a different version of jbotci than the one that produced the previous result.",
            );
            rendered.notice = Some(match rendered.notice.take() {
                Some(existing) => format!("{existing}\n{note}"),
                None => note,
            });
        }
        assemble(
            &rendered,
            published,
            self.attachment_limit(attachment_limit),
        )
        .map_err(ResultError::Assemble)
    }

    #[requires(true)]
    #[ensures(true)]
    async fn run_tool(
        &self,
        published: &PublishedRequest,
        deadline: Instant,
        keepalive: Option<WorkKeepalive>,
        attachment_limit: Option<u64>,
    ) -> Result<ToolOutcome, RunError> {
        // `run_request` admits the request itself, so nothing is validated
        // twice and no part of it runs on this thread.
        let context = OperationContext {
            tools: &self.tools,
            governor: &self.governor,
            deadline,
            keepalive,
            attachment_size_limit: Some(self.attachment_limit(attachment_limit) as u64),
            diagram_limits: DiagramLimits::default(),
        };
        match run_request(published.request.clone(), context).await {
            Ok(outcome) => Ok(outcome),
            Err(OperationError::Invalid(error)) => Err(RunError::Invalid(error)),
            Err(error) => Err(RunError::Failed(error)),
        }
    }

    /// What one attachment may weigh: the smaller of what this interaction
    /// allows and what this application will send.
    #[requires(true)]
    #[ensures(ret > 0)]
    fn attachment_limit(&self, interaction_limit: Option<u64>) -> usize {
        interaction_limit
            .and_then(|limit| usize::try_from(limit).ok())
            .filter(|limit| *limit > 0)
            .unwrap_or(DEFAULT_ATTACHMENT_LIMIT)
            .min(DEFAULT_ATTACHMENT_LIMIT)
            .max(1)
    }

    /// Write the message and make sure of the outcome. A write whose verdict
    /// never arrived may still have landed, so the message is read back and
    /// compared with what was meant to be there.
    #[requires(true)]
    #[ensures(true)]
    async fn write_and_confirm(
        &self,
        target: &Target,
        payload: &MessagePayload,
        deadline: Instant,
        keepalive: Option<WorkKeepalive>,
    ) {
        match self
            .write_original(target, payload, deadline, keepalive.clone())
            .await
        {
            Ok(()) => {}
            Err(WriteError::Ambiguous { reason }) => {
                let landed = match self
                    .read_original(target, deadline, keepalive.clone())
                    .await
                {
                    Ok(current) => message_fingerprint(&current) == payload_fingerprint(payload),
                    Err(_) => false,
                };
                if !landed {
                    self.report_privately(
                        target,
                        "jbotci could not confirm the change; the result may be unchanged. Open the ⚙️ form again to check.",
                        Some(&reason),
                        keepalive,
                    )
                    .await;
                }
            }
            Err(WriteError::Failed { reason }) => {
                self.report_privately(target, "Nothing was changed.", Some(&reason), keepalive)
                    .await;
            }
        }
    }

    /// The published request a message carries. Everything but a source that
    /// travelled as a file is read from the message itself.
    #[requires(true)]
    #[ensures(true)]
    async fn published_state(
        &self,
        message: &Value,
        deadline: Instant,
        keepalive: Option<WorkKeepalive>,
    ) -> Result<PublishedRequest, StateError> {
        let header = RequestHeader::of_message(message).ok_or(StateError::NoSettings)?;
        let block = match header.store {
            super::request::SourceStore::Inline => {
                input_block_of(message).ok_or(StateError::NoSettings)?
            }
            super::request::SourceStore::Attachment => {
                let url = attachment_url(message, INPUT_ATTACHMENT_FILENAME)
                    .ok_or(StateError::NoSettings)?;
                let api = self.api.clone();
                let bytes = self
                    .governor
                    .run_fetch_keeping(deadline, keepalive, move || {
                        api.fetch_attachment(&url, SOURCE_ATTACHMENT_CAP, remaining_at(deadline))
                    })
                    .await
                    .map_err(|_| StateError::SourceUnavailable)?
                    .map_err(|_| StateError::SourceUnavailable)?;
                String::from_utf8(bytes).map_err(|_| StateError::SourceUnavailable)?
            }
        };
        let fields =
            decode_input_block(header.tool, header.presence, &block).map_err(StateError::Block)?;
        header.rebuild(fields).map_err(StateError::Rebuild)
    }

    /// The message this interaction belongs to, waiting for Discord to create
    /// it: an acknowledgement promises a message that appears a moment later,
    /// and asking for it too early is not an error.
    #[requires(true)]
    #[ensures(true)]
    async fn await_original(
        &self,
        target: &Target,
        deadline: Instant,
        keepalive: Option<WorkKeepalive>,
    ) -> Result<Value, RestError> {
        let creation_deadline = (Instant::now() + ORIGINAL_CREATION_BUDGET).min(deadline);
        loop {
            match self
                .read_original(target, deadline, keepalive.clone())
                .await
            {
                Err(RestError::NotCreatedYet) if Instant::now() < creation_deadline => {
                    tokio::time::sleep(ORIGINAL_RETRY_PAUSE).await;
                }
                other => return other,
            }
        }
    }

    #[requires(true)]
    #[ensures(true)]
    async fn read_original(
        &self,
        target: &Target,
        deadline: Instant,
        keepalive: Option<WorkKeepalive>,
    ) -> Result<Value, RestError> {
        let api = self.api.clone();
        let application_id = target.application_id.clone();
        let token = target.token.clone();
        match self
            .governor
            .run_fetch_keeping(deadline, keepalive, move || {
                // What is left is measured where the call is made, not where
                // it was queued: time spent waiting for a lane belongs to the
                // same deadline and must not be granted twice.
                api.get_original(&application_id, &token, remaining_at(deadline))
            })
            .await
        {
            Ok(Ok(value)) => Ok(value),
            Ok(Err(TransportError::Rejected { status: 404, .. })) => Err(RestError::NotCreatedYet),
            Ok(Err(error)) => Err(RestError::Transport(error)),
            Err(error) => Err(RestError::Lane(error)),
        }
    }

    #[requires(true)]
    #[ensures(true)]
    async fn write_original(
        &self,
        target: &Target,
        payload: &MessagePayload,
        deadline: Instant,
        keepalive: Option<WorkKeepalive>,
    ) -> Result<(), WriteError> {
        let api = self.api.clone();
        let application_id = target.application_id.clone();
        let token = target.token.clone();
        let payload = payload.clone();
        match self
            .governor
            .run_fetch_keeping(deadline, keepalive, move || {
                api.edit_original(&application_id, &token, &payload, remaining_at(deadline))
            })
            .await
        {
            Ok(Ok(_)) => Ok(()),
            Ok(Err(error)) => Err(if error.may_have_applied() {
                WriteError::Ambiguous {
                    reason: error.to_string(),
                }
            } else {
                WriteError::Failed {
                    reason: error.to_string(),
                }
            }),
            // The lane gave up before the request was made, or while it was in
            // flight; either way the outcome is not known here.
            Err(error) => Err(WriteError::Ambiguous {
                reason: error.to_string(),
            }),
        }
    }

    /// Tell the acting reader something only they need to know: a sentence
    /// jbotci wrote, and where there is one, the reason, which came from their
    /// own request. This is what is said after the work is over, so it runs on
    /// its own budget: a request that used every second it had would otherwise
    /// be unable to report even that.
    #[requires(!summary.trim().is_empty())]
    #[ensures(true)]
    async fn report_privately(
        &self,
        target: &Target,
        summary: &str,
        reason: Option<&str>,
        keepalive: Option<WorkKeepalive>,
    ) {
        let deadline = Instant::now() + SETTLE_BUDGET;
        let api = self.api.clone();
        let application_id = target.application_id.clone();
        let token = target.token.clone();
        let (content, detail) = private_note(summary, reason);
        let _ = self
            .governor
            .run_fetch_keeping(deadline, keepalive, move || {
                api.create_ephemeral_followup(
                    &application_id,
                    &token,
                    &content,
                    detail
                        .as_ref()
                        .map(|bytes| (PRIVATE_DETAIL_FILENAME, bytes.as_slice())),
                    remaining_at(deadline),
                )
            })
            .await;
    }
}

/// An immediate answer about something the reader's own request produced. The
/// reason is escaped, because Discord renders a message's content as Markdown
/// and a reason quoting the request would otherwise be reformatted by it. This
/// application answers an interaction with inline JSON and uploads no file
/// there, so where a reason can be long the flows put it in the result or in a
/// private note, both of which can carry one.
#[requires(!summary.trim().is_empty())]
#[ensures(true)]
fn ephemeral_about(summary: &str, reason: &str) -> InteractionResponse {
    InteractionResponse::ephemeral(&format!("{summary}\n{}", markdown::escape(reason)))
}

/// The file a private note carries when its reason does not fit in a message.
const PRIVATE_DETAIL_FILENAME: &str = "jbotci-detail.txt";

/// What a private note shows, and what it carries. The reason came from the
/// reader's own request, so it is escaped rather than rendered as Markdown;
/// when the note cannot hold all of it, the message shows the beginning and
/// the whole of it travels as a file. Fifty complaints are only useful all
/// together, and an ellipsis is not an answer.
#[requires(!summary.trim().is_empty())]
#[ensures(super::request::utf16_len(&ret.0) <= MAX_CONTENT_UNITS)]
fn private_note(summary: &str, reason: Option<&str>) -> (String, Option<Vec<u8>>) {
    let Some(reason) = reason else {
        return (bound_content(summary), None);
    };
    let escaped = markdown::escape(reason);
    let whole = format!("{summary}\n{escaped}");
    if super::request::utf16_len(&whole) <= MAX_CONTENT_UNITS {
        return (whole, None);
    }
    let attached = markdown::subtext("The whole of it is attached.");
    let room = MAX_CONTENT_UNITS
        .saturating_sub(super::request::utf16_len(summary))
        .saturating_sub(super::request::utf16_len(&attached))
        .saturating_sub(2)
        .max(1);
    let (shown, _) = markdown::truncate_units(&escaped, room);
    (
        bound_content(&format!("{summary}\n{shown}\n{attached}")),
        Some(reason.as_bytes().to_vec()),
    )
}

/// What is left of one absolute deadline. Called from inside the admitted
/// worker so that waiting in the queue and talking to Discord share a single
/// budget rather than each being given the whole of it.
#[requires(true)]
#[ensures(true)]
fn remaining_at(deadline: Instant) -> Duration {
    deadline.saturating_duration_since(Instant::now())
}

/// What running work holds onto for as long as it actually runs, whoever is
/// still waiting for it: the delivery this interaction was admitted under,
/// and the message lock when the work is going to write. Both are released
/// when the last worker carrying this ends, not when a caller gives up.
#[invariant(true)]
#[derive(Debug)]
struct Retained {
    _ticket: Arc<DeliveryTicket>,
    _guard: MessageGuard,
}

/// A message that is only words: no state, no attachments, nothing to reopen.
/// This is what a request that cannot travel in a message leaves behind.
#[requires(!text.trim().is_empty())]
#[ensures(true)]
fn text_only_payload(text: &str) -> Result<MessagePayload, PayloadError> {
    let display =
        TextDisplay::new(None, text.to_owned()).map_err(|_| PayloadError::TextBudget {
            units: super::request::utf16_len(text),
        })?;
    MessagePayload::new(
        vec1::Vec1::new(MessageComponent::TextDisplay(display)),
        Vec::new(),
    )
}

/// The message one interaction may read and write.
#[invariant(true)]
#[derive(Debug, Clone)]
struct Target {
    application_id: Snowflake,
    token: InteractionToken,
}

/// What a published message says, apart from what Discord assigns: every
/// component's kind, text and identifier in order, and the attachment names.
/// Discord hands back no attachment bytes, so those are named rather than
/// compared, which is as far as reading a message can go.
#[requires(true)]
#[ensures(true)]
fn message_fingerprint(message: &Value) -> Vec<String> {
    #[requires(true)]
    #[ensures(true)]
    fn walk(value: &Value, into: &mut Vec<String>) {
        match value {
            Value::Object(object) => {
                if let Some(kind) = object.get("type").and_then(Value::as_u64) {
                    into.push(format!("type={kind}"));
                }
                if let Some(content) = object.get("content").and_then(Value::as_str) {
                    into.push(format!("content={content}"));
                }
                if let Some(custom_id) = object.get("custom_id").and_then(Value::as_str) {
                    into.push(format!("custom_id={custom_id}"));
                }
                for key in ["file", "media"] {
                    if let Some(url) = object
                        .get(key)
                        .and_then(|nested| nested.get("url"))
                        .and_then(Value::as_str)
                    {
                        into.push(format!("{key}={}", attachment_name_of(url)));
                    }
                }
                for (key, nested) in object {
                    if matches!(
                        key.as_str(),
                        "components" | "component" | "accessory" | "items"
                    ) {
                        walk(nested, into);
                    }
                }
            }
            Value::Array(items) => {
                for item in items {
                    walk(item, into);
                }
            }
            _ => {}
        }
    }
    let mut fingerprint = Vec::new();
    if let Some(components) = message.get("components") {
        walk(components, &mut fingerprint);
    }
    let mut names = message
        .get("attachments")
        .and_then(Value::as_array)
        .map(|attachments| {
            attachments
                .iter()
                .filter_map(|attachment| attachment.get("filename").and_then(Value::as_str))
                .map(str::to_owned)
                .collect::<Vec<_>>()
        })
        .unwrap_or_default();
    names.sort();
    fingerprint.extend(names.into_iter().map(|name| format!("attachment={name}")));
    fingerprint
}

/// The same fingerprint, for the message this build meant to publish.
#[requires(true)]
#[ensures(true)]
fn payload_fingerprint(payload: &MessagePayload) -> Vec<String> {
    message_fingerprint(&payload.to_json())
}

/// The file name inside an attachment URL, which is how a published message
/// refers to what was uploaded.
#[requires(true)]
#[ensures(true)]
fn attachment_name_of(url: &str) -> String {
    url.rsplit('/')
        .next()
        .map(|name| name.split('?').next().unwrap_or(name).to_owned())
        .unwrap_or_default()
}

/// What this application will send as one file, whatever an interaction
/// allows. Discord's own default per-file limit is ten mebibytes.
const DEFAULT_ATTACHMENT_LIMIT: usize = 10 * 1024 * 1024;

/// Why a result could not be produced.
#[invariant(::Link(_) => true)]
#[invariant(::Invalid(_) => true)]
#[invariant(::Operation(_) => true)]
#[invariant(::Assemble(_) => true)]
#[derive(Debug)]
enum ResultError {
    Link(AppLinkTooLarge),
    /// The request itself is incomplete or wrong. Publishing shows this as an
    /// editable result; an edit reports it privately and changes nothing.
    Invalid(RequestValidationError),
    Operation(OperationError),
    Assemble(AssembleError),
}

impl std::fmt::Display for ResultError {
    #[requires(true)]
    #[ensures(true)]
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Link(error) => write!(formatter, "{error}"),
            Self::Invalid(error) => write!(formatter, "{error}"),
            Self::Operation(error) => write!(formatter, "{error}"),
            Self::Assemble(error) => write!(formatter, "{error}"),
        }
    }
}

/// How running a tool ended.
#[invariant(::Invalid(_) => true)]
#[invariant(::Failed(_) => true)]
#[derive(Debug)]
enum RunError {
    Invalid(RequestValidationError),
    Failed(OperationError),
}

/// Why a read of the published message did not produce one.
#[invariant(::NotCreatedYet => true)]
#[invariant(::Transport(_) => true)]
#[invariant(::Lane(_) => true)]
#[derive(Debug)]
enum RestError {
    /// Discord has not created the message its acknowledgement promised.
    NotCreatedYet,
    Transport(TransportError),
    Lane(super::work::WorkError),
}

impl std::fmt::Display for RestError {
    #[requires(true)]
    #[ensures(true)]
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::NotCreatedYet => formatter.write_str("the message was never created"),
            Self::Transport(error) => write!(formatter, "{error}"),
            Self::Lane(error) => write!(formatter, "{error}"),
        }
    }
}

/// Why a message's published state could not be read back.
#[invariant(::NoSettings => true)]
#[invariant(::SourceUnavailable => true)]
#[invariant(::Block(_) => true)]
#[invariant(::Rebuild(_) => true)]
#[derive(Debug)]
enum StateError {
    NoSettings,
    SourceUnavailable,
    Block(super::codec::InputBlockError),
    Rebuild(super::codec::RebuildError),
}

impl std::fmt::Display for StateError {
    #[requires(true)]
    #[ensures(true)]
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::NoSettings => formatter.write_str(
                "jbotci could not find its settings on that message. Run the command again.",
            ),
            Self::SourceUnavailable => formatter.write_str(
                "jbotci could not read the attached source of that result. Run the command again.",
            ),
            Self::Block(error) => write!(
                formatter,
                "jbotci could not read the source on that message ({error}). Run the command again."
            ),
            Self::Rebuild(error) => write!(
                formatter,
                "jbotci could not rebuild that request ({error}). Run the command again."
            ),
        }
    }
}

/// Whether a failed write may still have landed.
#[invariant(::Ambiguous { .. } => true)]
#[invariant(::Failed { .. } => true)]
#[derive(Debug)]
enum WriteError {
    Ambiguous { reason: String },
    Failed { reason: String },
}

impl std::fmt::Display for WriteError {
    #[requires(true)]
    #[ensures(true)]
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Ambiguous { reason } => {
                write!(formatter, "{reason}, and it may or may not have applied")
            }
            Self::Failed { reason } => formatter.write_str(reason),
        }
    }
}

#[requires(true)]
#[ensures(!ret.is_empty())]
fn command_error_text(error: &CommandDecodeError) -> String {
    format!("That command could not be read: {error}")
}

#[requires(true)]
#[ensures(!ret.is_empty())]
fn submission_error_text(error: &SubmissionError) -> String {
    format!("Nothing was changed: {error}.")
}

/// The input block a published message carries, by its component id.
#[requires(true)]
#[ensures(true)]
fn input_block_of(message: &Value) -> Option<String> {
    #[requires(true)]
    #[ensures(true)]
    fn walk(value: &Value) -> Option<String> {
        match value {
            Value::Object(object) => {
                if object.get("id").and_then(Value::as_u64) == Some(u64::from(INPUT_COMPONENT_ID))
                    && let Some(content) = object.get("content").and_then(Value::as_str)
                {
                    return Some(content.to_owned());
                }
                object.values().find_map(walk)
            }
            Value::Array(items) => items.iter().find_map(walk),
            _ => None,
        }
    }
    walk(message.get("components")?)
}

/// The URL of the attachment named `filename`, as Discord currently signs it.
#[requires(!filename.is_empty())]
#[ensures(true)]
fn attachment_url(message: &Value, filename: &str) -> Option<String> {
    message
        .get("attachments")?
        .as_array()?
        .iter()
        .find(|attachment| attachment.get("filename").and_then(Value::as_str) == Some(filename))
        .and_then(|attachment| attachment.get("url").and_then(Value::as_str))
        .map(str::to_owned)
}

impl RequestHeader {
    /// The settings a published message carries, from its ⚙️ button.
    #[requires(true)]
    #[ensures(true)]
    pub(crate) fn of_message(message: &Value) -> Option<Self> {
        #[requires(true)]
        #[ensures(true)]
        fn walk(value: &Value) -> Option<RequestHeader> {
            match value {
                Value::Object(object) => {
                    if let Some(custom_id) = object.get("custom_id").and_then(Value::as_str)
                        && let Ok(header) = RequestHeader::decode(custom_id)
                    {
                        return Some(header);
                    }
                    object.values().find_map(walk)
                }
                Value::Array(items) => items.iter().find_map(walk),
                _ => None,
            }
        }
        walk(message.get("components")?)
    }
}

// ---------------------------------------------------------------------------
// What Discord sent
// ---------------------------------------------------------------------------

/// One interaction, as far as jbotci is concerned.
#[invariant(::Ping => true)]
#[invariant(::Command(_) => true)]
#[invariant(::Component(_) => true)]
#[invariant(::ModalSubmit(_) => true)]
#[derive(Debug, Clone)]
enum Interaction {
    Ping,
    Command(CommandInteraction),
    Component(ComponentInteraction),
    ModalSubmit(ModalSubmitInteraction),
}

#[invariant(true)]
#[derive(Debug, Clone)]
struct CommandInteraction {
    id: Snowflake,
    application_id: Snowflake,
    token: InteractionToken,
    actor: Snowflake,
    data: Value,
    /// What Discord will accept as one file for this interaction.
    attachment_limit: Option<u64>,
}

#[invariant(true)]
#[derive(Debug, Clone)]
struct ComponentInteraction {
    id: Snowflake,
    application_id: Snowflake,
    token: InteractionToken,
    actor: Snowflake,
    /// Which control was pressed. The gear opens the form; a page button asks
    /// for a page.
    custom_id: String,
    message_id: Snowflake,
    message: Value,
    attachment_limit: Option<u64>,
}

#[invariant(true)]
#[derive(Debug, Clone)]
struct ModalSubmitInteraction {
    id: Snowflake,
    application_id: Snowflake,
    token: InteractionToken,
    actor: Snowflake,
    custom_id: String,
    data: Value,
    message: Value,
    message_id: Snowflake,
    attachment_limit: Option<u64>,
}

impl Interaction {
    /// Read one interaction. Anything jbotci does not handle, or that is
    /// missing what it needs, is `None`: a guess here would act on the wrong
    /// message or the wrong reader.
    #[requires(true)]
    #[ensures(true)]
    fn read(value: &Value) -> Option<Self> {
        let kind = value.get("type").and_then(Value::as_u64)?;
        if kind == 1 {
            return Some(Self::Ping);
        }
        let id = snowflake(value.get("id"))?;
        let application_id = snowflake(value.get("application_id"))?;
        let token = InteractionToken::parse(value.get("token")?.as_str()?).ok()?;
        let actor = actor_of(value)?;
        let data = value.get("data").cloned().unwrap_or(Value::Null);
        let attachment_limit = value.get("attachment_size_limit").and_then(Value::as_u64);
        match kind {
            2 => Some(Self::Command(CommandInteraction {
                id,
                application_id,
                token,
                actor,
                data,
                attachment_limit,
            })),
            3 => {
                let message = value.get("message").cloned()?;
                Some(Self::Component(ComponentInteraction {
                    id,
                    application_id,
                    token,
                    actor,
                    custom_id: data.get("custom_id")?.as_str()?.to_owned(),
                    message_id: snowflake(message.get("id"))?,
                    message,
                    attachment_limit,
                }))
            }
            5 => {
                let message = value.get("message").cloned()?;
                Some(Self::ModalSubmit(ModalSubmitInteraction {
                    id,
                    application_id,
                    token,
                    actor,
                    custom_id: data.get("custom_id")?.as_str()?.to_owned(),
                    message_id: snowflake(message.get("id"))?,
                    message,
                    data,
                    attachment_limit,
                }))
            }
            _ => None,
        }
    }
}

/// The reader who acted: a guild interaction reports them under `member`, a
/// direct message under `user`.
#[requires(true)]
#[ensures(true)]
fn actor_of(value: &Value) -> Option<Snowflake> {
    value
        .get("member")
        .and_then(|member| member.get("user"))
        .and_then(|user| snowflake(user.get("id")))
        .or_else(|| value.get("user").and_then(|user| snowflake(user.get("id"))))
}

#[requires(true)]
#[ensures(true)]
fn snowflake(value: Option<&Value>) -> Option<Snowflake> {
    Snowflake::parse(value?.as_str()?).ok()
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::response::IntoResponse;
    use std::collections::VecDeque;
    use std::sync::Mutex;

    use axum::Router;
    use axum::extract::{Path, State};
    use axum::routing::get;
    use serde_json::json;

    use crate::discord::codec::SCHEMA_VERSION;
    use crate::discord::components::{Modal, ModalComponent, ModalControl};
    use crate::discord::operations::OperationError;
    use crate::discord::request::DiscordTool;
    use crate::discord::request::PageNumber;
    use crate::discord::work::{WorkError, WorkGovernor, WorkLane};

    const ACTOR: &str = "123456789012345678";
    const OTHER_ACTOR: &str = "222222222222222222";
    const APPLICATION: &str = "111111111111111111";
    const MESSAGE: &str = "999999999999999999";

    /// What the fake Discord does with the next write.
    #[invariant(true)]
    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    enum Reply {
        /// The call is answered as usual.
        Ok,
        /// The message has not been created yet.
        NotCreated,
        /// The write fails outright, and nothing changes.
        Failed,
        /// The write lands, and then the answer is lost.
        AppliedThenLost,
        /// The write is lost, and nothing changes.
        LostAndNotApplied,
    }

    #[invariant(true)]
    #[derive(Debug, Default)]
    struct FakeState {
        original: Mutex<Option<Value>>,
        reads: Mutex<VecDeque<Reply>>,
        writes: Mutex<VecDeque<Reply>>,
        recorded: Mutex<Vec<(String, Value)>>,
    }

    /// A Discord that answers on localhost and remembers what it was asked.
    #[invariant(true)]
    #[derive(Debug)]
    struct FakeDiscord {
        base_url: String,
        state: Arc<FakeState>,
    }

    impl FakeDiscord {
        #[requires(true)]
        #[ensures(true)]
        async fn start() -> Self {
            let state = Arc::new(FakeState::default());
            let router = Router::new()
                .route(
                    "/webhooks/{application}/{token}/messages/@original",
                    get(read_original).patch(write_original),
                )
                .route(
                    "/webhooks/{application}/{token}",
                    axum::routing::post(followup),
                )
                .with_state(Arc::clone(&state));
            let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
                .await
                .expect("a port");
            let address = listener.local_addr().expect("an address");
            tokio::spawn(async move {
                let _ = axum::serve(listener, router).await;
            });
            Self {
                base_url: format!("http://{address}"),
                state,
            }
        }

        #[requires(true)]
        #[ensures(true)]
        fn publish(&self, message: Value) {
            *self.state.original.lock().expect("state") = Some(message);
        }

        #[requires(true)]
        #[ensures(true)]
        fn original(&self) -> Option<Value> {
            self.state.original.lock().expect("state").clone()
        }

        #[requires(true)]
        #[ensures(true)]
        fn plan_reads(&self, replies: &[Reply]) {
            *self.state.reads.lock().expect("state") = replies.iter().copied().collect();
        }

        #[requires(true)]
        #[ensures(true)]
        fn plan_writes(&self, replies: &[Reply]) {
            *self.state.writes.lock().expect("state") = replies.iter().copied().collect();
        }

        /// Every request, as (method and path, body).
        #[requires(true)]
        #[ensures(true)]
        fn requests(&self) -> Vec<(String, Value)> {
            self.state.recorded.lock().expect("state").clone()
        }

        #[requires(!method.is_empty())]
        #[ensures(true)]
        fn count(&self, method: &str) -> usize {
            self.requests()
                .iter()
                .filter(|(what, _)| what.starts_with(method))
                .count()
        }

        /// The complete text a private note carried as a file, in order.
        #[requires(true)]
        #[ensures(true)]
        fn private_details(&self) -> Vec<String> {
            self.requests()
                .iter()
                .filter(|(what, _)| what.starts_with("POST"))
                .filter_map(|(_, body)| {
                    body.get("jbotci_detail")
                        .and_then(Value::as_str)
                        .map(str::to_owned)
                })
                .collect()
        }

        /// The private messages the acting reader was sent.
        #[requires(true)]
        #[ensures(true)]
        fn private_messages(&self) -> Vec<String> {
            self.requests()
                .iter()
                .filter(|(what, _)| what.starts_with("POST"))
                .filter_map(|(_, body)| {
                    body.get("content")
                        .and_then(Value::as_str)
                        .map(str::to_owned)
                })
                .collect()
        }
    }

    #[requires(true)]
    #[ensures(true)]
    async fn read_original(
        State(state): State<Arc<FakeState>>,
        Path((_application, _token)): Path<(String, String)>,
    ) -> axum::response::Response {
        state
            .recorded
            .lock()
            .expect("state")
            .push(("GET @original".to_owned(), Value::Null));
        match state.reads.lock().expect("state").pop_front() {
            Some(Reply::NotCreated) => {
                return (axum::http::StatusCode::NOT_FOUND, "unknown message").into_response();
            }
            Some(Reply::Failed) | Some(Reply::LostAndNotApplied) | Some(Reply::AppliedThenLost) => {
                return (axum::http::StatusCode::INTERNAL_SERVER_ERROR, "lost").into_response();
            }
            Some(Reply::Ok) | None => {}
        }
        match state.original.lock().expect("state").clone() {
            Some(message) => axum::Json(message).into_response(),
            None => (axum::http::StatusCode::NOT_FOUND, "unknown message").into_response(),
        }
    }

    #[requires(true)]
    #[ensures(true)]
    async fn write_original(
        State(state): State<Arc<FakeState>>,
        Path((_application, _token)): Path<(String, String)>,
        headers: axum::http::HeaderMap,
        body: axum::body::Bytes,
    ) -> axum::response::Response {
        let mut payload = payload_json(&headers, &body);
        if let Some(uploaded) = uploaded_part(&headers, &body, "files[0]") {
            payload["jbotci_result"] = json!(uploaded);
        }
        state
            .recorded
            .lock()
            .expect("state")
            .push(("PATCH @original".to_owned(), payload.clone()));
        let planned = state.writes.lock().expect("state").pop_front();
        let message = discord_message(&payload);
        match planned {
            Some(Reply::Failed) => {
                (axum::http::StatusCode::BAD_REQUEST, "rejected").into_response()
            }
            Some(Reply::LostAndNotApplied) => {
                (axum::http::StatusCode::INTERNAL_SERVER_ERROR, "lost").into_response()
            }
            Some(Reply::AppliedThenLost) => {
                *state.original.lock().expect("state") = Some(message);
                (axum::http::StatusCode::INTERNAL_SERVER_ERROR, "lost").into_response()
            }
            _ => {
                *state.original.lock().expect("state") = Some(message.clone());
                axum::Json(message).into_response()
            }
        }
    }

    #[requires(true)]
    #[ensures(true)]
    async fn followup(
        State(state): State<Arc<FakeState>>,
        Path((_application, _token)): Path<(String, String)>,
        headers: axum::http::HeaderMap,
        body: axum::body::Bytes,
    ) -> axum::response::Response {
        let mut payload = payload_json(&headers, &body);
        // Discord refuses a message whose content is longer than this; the
        // fake refuses it too, so a test cannot pass on a note that would not
        // have been delivered.
        let content = payload["content"].as_str().unwrap_or_default();
        assert!(
            crate::discord::request::utf16_len(content) <= 2000,
            "a private note of {} units would be refused by Discord",
            crate::discord::request::utf16_len(content)
        );
        if let Some(detail) = uploaded_part(&headers, &body, "files[0]") {
            payload["jbotci_detail"] = json!(detail);
        }
        state
            .recorded
            .lock()
            .expect("state")
            .push(("POST followup".to_owned(), payload));
        axum::Json(json!({ "id": "1" })).into_response()
    }

    /// The bytes of one uploaded multipart part, as text.
    #[requires(!name.is_empty())]
    #[ensures(true)]
    fn uploaded_part(
        headers: &axum::http::HeaderMap,
        body: &axum::body::Bytes,
        name: &str,
    ) -> Option<String> {
        let content_type = headers
            .get("content-type")
            .and_then(|value| value.to_str().ok())
            .unwrap_or_default();
        if !content_type.starts_with("multipart/") {
            return None;
        }
        let text = String::from_utf8_lossy(body);
        let marker = format!("name=\"{name}\"");
        let start = text.find(&marker)?;
        let after_headers = start + text[start..].find("\r\n\r\n")? + 4;
        let end = text[after_headers..]
            .find("\r\n--")
            .map(|offset| after_headers + offset)
            .unwrap_or(text.len());
        Some(text[after_headers..end].to_owned())
    }

    /// The message Discord would hold after a write: the components as sent,
    /// and the attachments named and addressed as Discord addresses them.
    #[requires(true)]
    #[ensures(true)]
    fn discord_message(payload: &Value) -> Value {
        let attachments = payload
            .get("attachments")
            .and_then(Value::as_array)
            .map(|attachments| {
                attachments
                    .iter()
                    .enumerate()
                    .map(|(index, attachment)| {
                        let filename = attachment
                            .get("filename")
                            .and_then(Value::as_str)
                            .unwrap_or_default();
                        json!({
                            "id": index.to_string(),
                            "filename": filename,
                            "url": format!("https://cdn.test/{index}/{filename}?ex=1"),
                        })
                    })
                    .collect::<Vec<_>>()
            })
            .unwrap_or_default();
        json!({
            "id": MESSAGE,
            "components": payload.get("components").cloned().unwrap_or(Value::Null),
            "attachments": attachments,
        })
    }

    /// The JSON of a write, whether it came alone or beside uploaded files.
    #[requires(true)]
    #[ensures(true)]
    fn payload_json(headers: &axum::http::HeaderMap, body: &axum::body::Bytes) -> Value {
        let content_type = headers
            .get("content-type")
            .and_then(|value| value.to_str().ok())
            .unwrap_or_default();
        if !content_type.starts_with("multipart/") {
            return serde_json::from_slice(body).unwrap_or(Value::Null);
        }
        // Find the payload_json part and read to the next boundary.
        let text = String::from_utf8_lossy(body);
        let marker = "name=\"payload_json\"";
        let Some(start) = text.find(marker) else {
            return Value::Null;
        };
        let after_headers = text[start..]
            .find("\r\n\r\n")
            .map(|offset| start + offset + 4)
            .unwrap_or(start);
        let end = text[after_headers..]
            .find("\r\n--")
            .map(|offset| after_headers + offset)
            .unwrap_or(text.len());
        serde_json::from_str(&text[after_headers..end]).unwrap_or(Value::Null)
    }

    #[requires(true)]
    #[ensures(true)]
    fn new_service(discord: &FakeDiscord) -> Arc<DiscordService> {
        let config = new!(DiscordConfig {
            public_key: "00".repeat(32),
            api_base: discord.base_url.clone(),
            public_base_url: "https://jbotci.app".to_owned(),
        });
        Arc::new(DiscordService::new(config, ToolServices::new()))
    }

    #[requires(!subcommand.is_empty())]
    #[ensures(true)]
    fn command(id: &str, subcommand: &str, options: Vec<Value>) -> Value {
        json!({
            "type": 2,
            "id": id,
            "application_id": APPLICATION,
            "token": "test-token",
            "member": { "user": { "id": ACTOR } },
            "data": {
                "name": "jbotci",
                "options": [{ "name": subcommand, "type": 1, "options": options }],
            },
        })
    }

    #[requires(!name.is_empty())]
    #[ensures(true)]
    fn option(name: &str, value: &str) -> Value {
        json!({ "name": name, "value": value })
    }

    /// The ⚙️ button of the message the fake currently holds.
    #[requires(true)]
    #[ensures(true)]
    fn gear_of(message: &Value) -> String {
        #[requires(true)]
        #[ensures(true)]
        fn walk(value: &Value) -> Option<String> {
            match value {
                Value::Object(object) => {
                    if let Some(custom_id) = object.get("custom_id").and_then(Value::as_str)
                        && custom_id.starts_with(SCHEMA_VERSION)
                    {
                        return Some(custom_id.to_owned());
                    }
                    object.values().find_map(walk)
                }
                Value::Array(items) => items.iter().find_map(walk),
                _ => None,
            }
        }
        walk(message).expect("a published message carries its gear")
    }

    #[requires(true)]
    #[ensures(true)]
    fn component_interaction(id: &str, message: &Value, actor: &str) -> Value {
        json!({
            "type": 3,
            "id": id,
            "application_id": APPLICATION,
            "token": "component-token",
            "member": { "user": { "id": actor } },
            "message": message,
            "data": { "custom_id": gear_of(message), "component_type": 2 },
        })
    }

    /// The page buttons a published message carries, as (label, custom id,
    /// disabled).
    #[requires(true)]
    #[ensures(true)]
    fn page_buttons(message: &Value) -> Vec<(String, String, bool)> {
        #[requires(true)]
        #[ensures(into.len() >= old(into.len()))]
        fn walk(value: &Value, into: &mut Vec<(String, String, bool)>) {
            match value {
                Value::Object(object) => {
                    if object.get("type").and_then(Value::as_u64) == Some(1)
                        && let Some(items) = object.get("components").and_then(Value::as_array)
                    {
                        for item in items {
                            let label = item
                                .get("label")
                                .and_then(Value::as_str)
                                .unwrap_or_default()
                                .to_owned();
                            let custom_id = item
                                .get("custom_id")
                                .and_then(Value::as_str)
                                .unwrap_or_default()
                                .to_owned();
                            let disabled = item
                                .get("disabled")
                                .and_then(Value::as_bool)
                                .unwrap_or(false);
                            into.push((label, custom_id, disabled));
                        }
                    }
                    for item in object.values() {
                        walk(item, into);
                    }
                }
                Value::Array(items) => {
                    for item in items {
                        walk(item, into);
                    }
                }
                _ => {}
            }
        }
        let mut found = Vec::new();
        walk(message, &mut found);
        found
    }

    /// A click of the button labelled `label` on `message`.
    #[requires(!label.is_empty())]
    #[ensures(true)]
    fn page_click(id: &str, message: &Value, actor: &str, label: &str) -> Value {
        let custom_id = page_buttons(message)
            .into_iter()
            .find(|(button, _, _)| button == label)
            .map(|(_, custom_id, _)| custom_id)
            .unwrap_or_else(|| panic!("no {label} button on {message}"));
        json!({
            "type": 3,
            "id": id,
            "application_id": APPLICATION,
            "token": "component-token",
            "member": { "user": { "id": actor } },
            "message": message,
            "data": { "custom_id": custom_id, "component_type": 2 },
        })
    }

    /// A component click on `message` carrying `custom_id` as it stands.
    #[requires(true)]
    #[ensures(true)]
    fn page_click_with_id(id: &str, message: &Value, actor: &str, custom_id: &str) -> Value {
        json!({
            "type": 3,
            "id": id,
            "application_id": APPLICATION,
            "token": "component-token",
            "member": { "user": { "id": actor } },
            "message": message,
            "data": { "custom_id": custom_id, "component_type": 2 },
        })
    }

    /// A submission of `modal` with `overrides` replacing what it shows.
    #[requires(true)]
    #[ensures(true)]
    fn submission(
        id: &str,
        modal: &Modal,
        message: &Value,
        actor: &str,
        overrides: &[(&str, Value)],
    ) -> Value {
        let mut components = Vec::new();
        for component in modal.components.iter() {
            let ModalComponent::Label(labeled) = component else {
                continue;
            };
            let custom_id = labeled.control.custom_id().as_str().to_owned();
            let value = match &labeled.control {
                ModalControl::TextInput(input) => {
                    json!({ "type": 4, "custom_id": custom_id, "value": input.value.clone().unwrap_or_default() })
                }
                ModalControl::RadioGroup(group) => json!({
                    "type": 21,
                    "custom_id": custom_id,
                    "value": group.options.iter().find(|option| option.default).map(|option| option.value.clone()),
                }),
                ModalControl::CheckboxGroup(group) => json!({
                    "type": 22,
                    "custom_id": custom_id,
                    "values": group.options.iter().filter(|option| option.default).map(|option| option.value.clone()).collect::<Vec<_>>(),
                }),
                ModalControl::StringSelect(select) => json!({
                    "type": 3,
                    "custom_id": custom_id,
                    "values": select.options.iter().filter(|option| option.default).map(|option| option.value.clone()).collect::<Vec<_>>(),
                }),
            };
            components.push(json!({ "type": 18, "component": value }));
        }
        for (custom_id, value) in overrides {
            for component in &mut components {
                if component["component"]["custom_id"] == json!(custom_id) {
                    let mut control = component["component"].clone();
                    control
                        .as_object_mut()
                        .expect("a control")
                        .remove(if value.is_array() { "value" } else { "values" });
                    control[if value.is_array() { "values" } else { "value" }] = value.clone();
                    component["component"] = control;
                }
            }
        }
        json!({
            "type": 5,
            "id": id,
            "application_id": APPLICATION,
            "token": "modal-token",
            "member": { "user": { "id": actor } },
            "message": message,
            "data": { "custom_id": modal.custom_id.as_str(), "components": components },
        })
    }

    #[requires(true)]
    #[ensures(true)]
    fn modal_of(response: InteractionResponse) -> Modal {
        match response {
            InteractionResponse::Modal(modal) => modal,
            other => panic!("expected a form, got {other:?}"),
        }
    }

    #[requires(true)]
    #[ensures(true)]
    fn ephemeral_text(response: &InteractionResponse) -> String {
        match response {
            InteractionResponse::EphemeralText { content } => content.clone(),
            other => panic!("expected a private message, got {other:?}"),
        }
    }

    /// Wait until the spawned work has finished with the fake.
    #[requires(true)]
    #[ensures(true)]
    async fn settle(discord: &FakeDiscord, writes: usize) {
        for _ in 0..600 {
            if discord.count("PATCH") >= writes {
                // Give the task a moment to finish its own bookkeeping.
                tokio::time::sleep(Duration::from_millis(20)).await;
                return;
            }
            tokio::time::sleep(Duration::from_millis(10)).await;
        }
        panic!(
            "the work never wrote {writes} time(s): {:?}",
            discord
                .requests()
                .iter()
                .map(|(what, _)| what)
                .collect::<Vec<_>>()
        );
    }

    #[requires(true)]
    #[ensures(true)]
    async fn quiet(discord: &FakeDiscord) {
        tokio::time::sleep(Duration::from_millis(250)).await;
        let _ = discord.requests();
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 4)]
    #[requires(true)]
    #[ensures(true)]
    async fn a_command_publishes_a_result_that_its_form_can_reopen() {
        let discord = FakeDiscord::start().await;
        let service = new_service(&discord);

        // Discord has not created the deferred message yet; the first read
        // finds nothing, and the work waits for it rather than giving up.
        discord.plan_reads(&[Reply::NotCreated, Reply::NotCreated]);
        let deferred = service
            .handle(&command(
                "1",
                "gentufa",
                vec![option("text", "mi klama lo zarci")],
            ))
            .await;
        assert!(matches!(
            deferred,
            InteractionResponse::DeferredChannelMessage
        ));
        tokio::time::sleep(Duration::from_millis(60)).await;
        discord.publish(json!({ "id": MESSAGE, "components": [], "attachments": [] }));
        settle(&discord, 1).await;

        let message = discord.original().expect("a published message");
        let text = message.to_string();
        assert!(text.contains("kláma"), "{text}");
        assert!(discord.private_messages().is_empty(), "nothing went wrong");

        // The ⚙️ button opens a form on that state, with its link.
        let modal = modal_of(
            service
                .handle(&component_interaction("2", &message, ACTOR))
                .await,
        );
        assert!(modal.custom_id.as_str().starts_with("j3m.g."));
        let ModalComponent::TextDisplay(link) = modal.components.first() else {
            panic!("the first component is the link");
        };
        assert!(
            link.content
                .contains("https://jbotci.app/gentufa?text=mi+klama+lo+zarci"),
            "{}",
            link.content
        );

        // Submitting it unchanged republishes the same request at the next
        // revision, and changing a setting changes the result.
        let submitted = service
            .handle(&submission(
                "3",
                &modal,
                &message,
                ACTOR,
                &[("view", json!("tree"))],
            ))
            .await;
        assert!(matches!(
            submitted,
            InteractionResponse::DeferredUpdateMessage
        ));
        settle(&discord, 2).await;
        let edited = discord.original().expect("an edited message");
        assert!(
            gear_of(&edited).contains(".2."),
            "the revision moved: {}",
            gear_of(&edited)
        );
        assert!(edited.to_string().contains("bridi"), "the tree view");
        assert!(discord.private_messages().is_empty());
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 4)]
    #[requires(true)]
    #[ensures(true)]
    async fn a_replayed_command_never_overwrites_what_the_form_has_changed() {
        let discord = FakeDiscord::start().await;
        let service = new_service(&discord);
        discord.publish(json!({ "id": MESSAGE, "components": [], "attachments": [] }));
        service
            .handle(&command("10", "vlatai", vec![option("text", "klama")]))
            .await;
        settle(&discord, 1).await;
        let published = discord.original().expect("a result");

        // The reader edits it through the form.
        let modal = modal_of(
            service
                .handle(&component_interaction("11", &published, ACTOR))
                .await,
        );
        service
            .handle(&submission(
                "12",
                &modal,
                &published,
                ACTOR,
                &[("text", json!("klabajra"))],
            ))
            .await;
        settle(&discord, 2).await;
        let edited = discord.original().expect("an edit");
        assert!(edited.to_string().contains("klabájra"), "the edit landed");

        // The original command is delivered again after a restart, with the
        // dedupe record gone. It must not put its own result back.
        let restarted = new_service(&discord);
        let writes_before = discord.count("PATCH");
        restarted
            .handle(&command("10", "vlatai", vec![option("text", "klama")]))
            .await;
        quiet(&discord).await;
        assert_eq!(
            discord.count("PATCH"),
            writes_before,
            "a replayed command wrote again"
        );
        assert_eq!(
            discord.original().expect("a message").to_string(),
            edited.to_string(),
            "the edited result is untouched"
        );
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 4)]
    #[requires(true)]
    #[ensures(true)]
    async fn a_duplicate_delivery_does_the_work_once() {
        let discord = FakeDiscord::start().await;
        let service = new_service(&discord);
        discord.publish(json!({ "id": MESSAGE, "components": [], "attachments": [] }));
        let first = service
            .handle(&command("20", "vlatai", vec![option("text", "klama")]))
            .await;
        let second = service
            .handle(&command("20", "vlatai", vec![option("text", "klama")]))
            .await;
        assert!(matches!(first, InteractionResponse::DeferredChannelMessage));
        assert!(matches!(
            second,
            InteractionResponse::DeferredChannelMessage
        ));
        settle(&discord, 1).await;
        quiet(&discord).await;
        assert_eq!(discord.count("PATCH"), 1, "one delivery, one publication");
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 4)]
    #[requires(true)]
    #[ensures(true)]
    async fn a_form_that_cannot_run_changes_nothing() {
        let discord = FakeDiscord::start().await;
        let service = new_service(&discord);
        discord.publish(json!({ "id": MESSAGE, "components": [], "attachments": [] }));
        service
            .handle(&command("30", "gentufa", vec![option("text", "mi klama")]))
            .await;
        settle(&discord, 1).await;
        let published = discord.original().expect("a result");
        let modal = modal_of(
            service
                .handle(&component_interaction("31", &published, ACTOR))
                .await,
        );

        // A dialect that does not parse is reported privately; the message
        // keeps the result it had.
        let response = service
            .handle(&submission(
                "32",
                &modal,
                &published,
                ACTOR,
                &[("dialect", json!("not a dialect(("))],
            ))
            .await;
        assert!(matches!(
            response,
            InteractionResponse::DeferredUpdateMessage
        ));
        for _ in 0..100 {
            if !discord.private_messages().is_empty() {
                break;
            }
            tokio::time::sleep(Duration::from_millis(10)).await;
        }
        let complaint = discord.private_messages().join("\n");
        assert!(complaint.contains("Nothing was changed"), "{complaint}");
        assert!(complaint.contains("dialect"), "{complaint}");
        assert_eq!(
            discord.original().expect("a message").to_string(),
            published.to_string(),
            "the result is exactly as it was"
        );
        assert_eq!(discord.count("PATCH"), 1, "nothing was written");
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 4)]
    #[requires(true)]
    #[ensures(true)]
    async fn an_incomplete_task_is_published_and_finished_in_its_form() {
        let discord = FakeDiscord::start().await;
        let service = new_service(&discord);
        discord.publish(json!({ "id": MESSAGE, "components": [], "attachments": [] }));

        // A search mode with no query is a task waiting to be finished.
        for (id, options) in [
            (40, vec![option("mode", "meaning")]),
            (41, vec![option("mode", "section")]),
        ] {
            *discord.state.original.lock().expect("state") =
                Some(json!({ "id": MESSAGE, "components": [], "attachments": [] }));
            let response = service
                .handle(&command(&id.to_string(), "cukta", options))
                .await;
            assert!(matches!(
                response,
                InteractionResponse::DeferredChannelMessage
            ));
            let writes = discord.count("PATCH");
            settle(&discord, writes + 1).await;
            let published = discord.original().expect("a result");
            let text = published.to_string();
            assert!(text.contains("Not run"), "{text}");
            assert!(text.contains("needs something to look for"), "{text}");
            assert!(discord.private_messages().is_empty(), "it is not an error");

            // The form completes it, and the result appears.
            let modal = modal_of(
                service
                    .handle(&component_interaction(&format!("{id}0"), &published, ACTOR))
                    .await,
            );
            let writes = discord.count("PATCH");
            service
                .handle(&submission(
                    &format!("{id}1"),
                    &modal,
                    &published,
                    ACTOR,
                    &[("query", json!("tanru"))],
                ))
                .await;
            settle(&discord, writes + 1).await;
            let finished = discord.original().expect("a result").to_string();
            assert!(!finished.contains("Not run"), "{finished}");
        }
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 4)]
    #[requires(true)]
    #[ensures(true)]
    async fn only_the_reader_who_asked_may_change_what_everyone_sees() {
        let discord = FakeDiscord::start().await;
        let service = new_service(&discord);
        discord.publish(json!({ "id": MESSAGE, "components": [], "attachments": [] }));
        service
            .handle(&command("50", "vlatai", vec![option("text", "klama")]))
            .await;
        settle(&discord, 1).await;
        let published = discord.original().expect("a result");

        // Anyone may open the form and use its link.
        let modal = modal_of(
            service
                .handle(&component_interaction("51", &published, OTHER_ACTOR))
                .await,
        );
        // Submitting it does nothing but explain why.
        let response = service
            .handle(&submission(
                "52",
                &modal,
                &published,
                OTHER_ACTOR,
                &[("text", json!("bajra"))],
            ))
            .await;
        let text = ephemeral_text(&response);
        assert!(
            text.contains("Only the person who ran this command"),
            "{text}"
        );
        quiet(&discord).await;
        assert_eq!(discord.count("PATCH"), 1, "nothing was written");
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 4)]
    #[requires(true)]
    #[ensures(true)]
    async fn a_form_opened_before_a_change_is_refused_rather_than_applied() {
        let discord = FakeDiscord::start().await;
        let service = new_service(&discord);
        discord.publish(json!({ "id": MESSAGE, "components": [], "attachments": [] }));
        service
            .handle(&command("60", "vlatai", vec![option("text", "klama")]))
            .await;
        settle(&discord, 1).await;
        let published = discord.original().expect("a result");
        let stale_form = modal_of(
            service
                .handle(&component_interaction("61", &published, ACTOR))
                .await,
        );

        // Someone applies a change first.
        let current_form = modal_of(
            service
                .handle(&component_interaction("62", &published, ACTOR))
                .await,
        );
        service
            .handle(&submission(
                "63",
                &current_form,
                &published,
                ACTOR,
                &[("text", json!("bajra"))],
            ))
            .await;
        settle(&discord, 2).await;
        let current = discord.original().expect("an edit");

        // The older form now describes a result that no longer exists.
        let response = service
            .handle(&submission(
                "64",
                &stale_form,
                &current,
                ACTOR,
                &[("text", json!("cadzu"))],
            ))
            .await;
        let text = ephemeral_text(&response);
        assert!(text.contains("changed while the form was open"), "{text}");
        quiet(&discord).await;
        assert_eq!(discord.count("PATCH"), 2, "the stale form wrote nothing");
        assert!(
            discord
                .original()
                .expect("a message")
                .to_string()
                .contains("bájra"),
            "the newer result stands"
        );
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 4)]
    #[requires(true)]
    #[ensures(true)]
    async fn a_write_whose_answer_is_lost_is_confirmed_by_reading_the_message() {
        let discord = FakeDiscord::start().await;
        let service = new_service(&discord);
        discord.publish(json!({ "id": MESSAGE, "components": [], "attachments": [] }));

        // The write lands, but its answer does not come back: reading the
        // message shows the intended result, so nothing is reported.
        discord.plan_writes(&[Reply::AppliedThenLost]);
        service
            .handle(&command("70", "vlatai", vec![option("text", "klama")]))
            .await;
        settle(&discord, 1).await;
        quiet(&discord).await;
        assert!(
            discord.private_messages().is_empty(),
            "an applied write needs no apology: {:?}",
            discord.private_messages()
        );
        let applied = discord.original().expect("a result");
        assert!(applied.to_string().contains("klá"), "the result is there");

        // Now a write that never landed: the message still shows the older
        // result, so the reader is told the change may not have applied.
        let modal = modal_of(
            service
                .handle(&component_interaction("71", &applied, ACTOR))
                .await,
        );
        discord.plan_writes(&[Reply::LostAndNotApplied]);
        service
            .handle(&submission(
                "72",
                &modal,
                &applied,
                ACTOR,
                &[("text", json!("bajra"))],
            ))
            .await;
        settle(&discord, 2).await;
        for _ in 0..100 {
            if !discord.private_messages().is_empty() {
                break;
            }
            tokio::time::sleep(Duration::from_millis(10)).await;
        }
        let complaint = discord.private_messages().join("\n");
        assert!(
            complaint.contains("could not confirm the change"),
            "{complaint}"
        );
        assert!(
            discord
                .original()
                .expect("a message")
                .to_string()
                .contains("klá"),
            "the older result is what the message still shows"
        );
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 4)]
    #[requires(true)]
    #[ensures(true)]
    async fn a_write_that_lands_as_something_else_is_not_taken_for_success() {
        let discord = FakeDiscord::start().await;
        let service = new_service(&discord);
        discord.publish(json!({ "id": MESSAGE, "components": [], "attachments": [] }));
        service
            .handle(&command("80", "vlatai", vec![option("text", "klama")]))
            .await;
        settle(&discord, 1).await;
        let published = discord.original().expect("a result");
        let modal = modal_of(
            service
                .handle(&component_interaction("81", &published, ACTOR))
                .await,
        );

        // The write is lost, and what the message holds is a different result
        // with the same settings: the fingerprint must not match on the
        // header alone.
        let mut impostor = published.clone();
        impostor["components"] = json!([]);
        discord.plan_writes(&[Reply::LostAndNotApplied]);
        service
            .handle(&submission(
                "82",
                &modal,
                &published,
                ACTOR,
                &[("text", json!("bajra"))],
            ))
            .await;
        tokio::time::sleep(Duration::from_millis(80)).await;
        discord.publish(impostor);
        for _ in 0..200 {
            if !discord.private_messages().is_empty() {
                break;
            }
            tokio::time::sleep(Duration::from_millis(10)).await;
        }
        let complaint = discord.private_messages().join("\n");
        assert!(
            complaint.contains("could not confirm the change"),
            "a different body is not the write landing: {complaint}"
        );
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 4)]
    #[requires(true)]
    #[ensures(true)]
    async fn a_small_attachment_limit_bounds_what_is_sent() {
        let discord = FakeDiscord::start().await;
        let service = new_service(&discord);
        discord.publish(json!({ "id": MESSAGE, "components": [], "attachments": [] }));

        // A parse whose text is long enough to travel as a file, with an
        // interaction that accepts almost nothing.
        let long = "mi klama lo zarci ".repeat(80);
        let mut interaction = command("90", "gentufa", vec![option("text", &long)]);
        interaction["attachment_size_limit"] = json!(64);
        service.handle(&interaction).await;
        settle(&discord, 1).await;
        // The source cannot travel, so no result is published; the message
        // still has to stop saying it is thinking, and says why instead.
        let published = discord.original().expect("a message");
        let text = published.to_string();
        assert!(
            text.contains("Not run"),
            "the message says what happened: {text}"
        );
        assert!(
            !text.contains("custom_id"),
            "a request that cannot travel carries no form: {text}"
        );

        // The same request with room for its file publishes it.
        let mut interaction = command("91", "gentufa", vec![option("text", &long)]);
        interaction["attachment_size_limit"] = json!(1 << 20);
        let writes = discord.count("PATCH");
        service.handle(&interaction).await;
        settle(&discord, writes + 1).await;
        let published = discord.original().expect("a result");
        assert!(
            published["attachments"]
                .as_array()
                .is_some_and(|attachments| !attachments.is_empty()),
            "the source travelled as a file: {published}"
        );
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 4)]
    #[requires(true)]
    #[ensures(true)]
    async fn a_form_reopens_a_result_whose_source_travelled_as_a_file() {
        let discord = FakeDiscord::start().await;
        let service = new_service(&discord);
        discord.publish(json!({ "id": MESSAGE, "components": [], "attachments": [] }));
        let long = "mi klama lo zarci ".repeat(80);
        service
            .handle(&command("100", "gentufa", vec![option("text", &long)]))
            .await;
        settle(&discord, 1).await;
        let published = discord.original().expect("a result");
        assert!(
            published["attachments"]
                .as_array()
                .is_some_and(|attachments| !attachments.is_empty()),
            "the source is a file"
        );
        // The fake serves that file from its own address, so opening the form
        // has to fetch it back.
        let response = service
            .handle(&component_interaction("101", &published, ACTOR))
            .await;
        // Without a reachable attachment the form says so rather than opening
        // on a guess; with one it opens on the exact source.
        match response {
            InteractionResponse::Modal(modal) => {
                let ModalComponent::Label(labeled) = &modal.components[1] else {
                    panic!("the text field");
                };
                let ModalControl::TextInput(input) = &labeled.control else {
                    panic!("a text input");
                };
                assert_eq!(input.value.as_deref(), Some(long.as_str()));
            }
            InteractionResponse::EphemeralText { content } => {
                assert!(content.contains("attached source"), "{content}");
            }
            other => panic!("unexpected {other:?}"),
        }
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 4)]
    #[requires(true)]
    #[ensures(true)]
    async fn an_initial_failure_becomes_the_message_it_promised() {
        // Work that cannot produce a result still owes the reader an answer:
        // the acknowledgement already put a thinking indicator on screen, and
        // only editing that message takes it away.
        let discord = FakeDiscord::start().await;
        let service = new_service(&discord);
        discord.publish(json!({ "id": MESSAGE, "components": [], "attachments": [] }));
        let published = PublishedRequest {
            request: decode_command(
                &command("110", "vlasei", vec![option("text", "mi klama")])["data"],
            )
            .expect("a request"),
            revision: Revision::INITIAL,
            initiator: Snowflake::parse(ACTOR).expect("snowflake"),
            build_tag: BuildTag::current(),
        };
        let target = Target {
            application_id: Snowflake::parse(APPLICATION).expect("snowflake"),
            token: InteractionToken::parse("test-token").expect("a token"),
        };
        service
            .publish_failure(
                &target,
                &published,
                &ResultError::Operation(OperationError::Work(WorkError::Overloaded {
                    lane: WorkLane::Compute,
                })),
                None,
                None,
            )
            .await;
        settle(&discord, 1).await;

        let message = discord.original().expect("a message");
        let text = message.to_string();
        assert!(
            text.contains("Not run"),
            "the failure is the message: {text}"
        );
        // And it is an editable result: its form opens on the same request,
        // so the reader can change it and try again.
        let response = service
            .handle(&component_interaction("111", &message, ACTOR))
            .await;
        let modal = modal_of(response);
        let source = modal
            .components
            .iter()
            .find_map(|component| match component {
                ModalComponent::Label(labeled) => match &labeled.control {
                    ModalControl::TextInput(input) => input.value.clone(),
                    _ => None,
                },
                _ => None,
            })
            .expect("the source field");
        assert_eq!(source, "mi klama", "the failed request reopens as it was");
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 4)]
    #[requires(true)]
    #[ensures(true)]
    async fn a_failure_never_replaces_a_result_that_was_already_published() {
        // A failing replay arrives after the first delivery published a real
        // result. The failure path reads the message before writing, exactly
        // as the success path does, so the reader keeps their result.
        let discord = FakeDiscord::start().await;
        let service = new_service(&discord);
        discord.publish(json!({ "id": MESSAGE, "components": [], "attachments": [] }));
        service
            .handle(&command("130", "vlasei", vec![option("text", "mi klama")]))
            .await;
        settle(&discord, 1).await;
        let published = discord.original().expect("a result");

        let request =
            decode_command(&command("131", "vlasei", vec![option("text", "mi klama")])["data"])
                .expect("a request");
        let state = PublishedRequest {
            request,
            revision: Revision::INITIAL,
            initiator: Snowflake::parse(ACTOR).expect("snowflake"),
            build_tag: BuildTag::current(),
        };
        let target = Target {
            application_id: Snowflake::parse(APPLICATION).expect("snowflake"),
            token: InteractionToken::parse("test-token").expect("a token"),
        };
        let writes = discord.count("PATCH");
        service
            .publish_failure(
                &target,
                &state,
                &ResultError::Operation(OperationError::Work(WorkError::Overloaded {
                    lane: WorkLane::Compute,
                })),
                None,
                None,
            )
            .await;
        quiet(&discord).await;
        assert_eq!(discord.count("PATCH"), writes, "nothing was written");
        assert_eq!(
            discord.original().expect("a result"),
            published,
            "the published result is untouched"
        );
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 4)]
    #[requires(true)]
    #[ensures(true)]
    async fn pages_are_turned_on_the_message_and_say_where_they_are() {
        let discord = FakeDiscord::start().await;
        let service = new_service(&discord);
        discord.publish(json!({ "id": MESSAGE, "components": [], "attachments": [] }));
        // A pattern with many dictionary matches, so the result has pages.
        service
            .handle(&command("200", "vlacku", vec![option("query", "kla*")]))
            .await;
        settle(&discord, 1).await;
        let first = discord.original().expect("a result");
        let buttons = page_buttons(&first);
        assert_eq!(
            buttons
                .iter()
                .map(|(label, _, disabled)| (label.as_str(), *disabled))
                .collect::<Vec<_>>(),
            vec![("Previous", true), ("Next", false)],
            "the first page cannot go back and can go on"
        );
        assert!(
            first.to_string().contains("1-5"),
            "the result says which results it shows: {first}"
        );
        assert!(
            !first.to_string().contains("1-5 of "),
            "a search that fetched one page knows no total to claim: {first}"
        );

        // Next turns the page on the same message.
        let writes = discord.count("PATCH");
        service
            .handle(&page_click("201", &first, ACTOR, "Next"))
            .await;
        settle(&discord, writes + 1).await;
        let second = discord.original().expect("a result");
        assert!(
            second.to_string().contains("6-10"),
            "the second page shows the next results: {second}"
        );
        assert!(
            page_buttons(&second)
                .iter()
                .any(|(label, _, disabled)| label == "Previous" && !disabled),
            "page two can go back"
        );

        // Following Next to the end reaches a page that says it is the last,
        // however many pages that takes; nothing here assumes a page count.
        let mut current = second.clone();
        let mut walked = 1;
        for step in 0..40 {
            let next_enabled = page_buttons(&current)
                .iter()
                .any(|(label, _, disabled)| label == "Next" && !disabled);
            if !next_enabled {
                break;
            }
            let writes = discord.count("PATCH");
            service
                .handle(&page_click(&format!("29{step}"), &current, ACTOR, "Next"))
                .await;
            settle(&discord, writes + 1).await;
            current = discord.original().expect("a result");
            walked += 1;
        }
        assert!(
            page_buttons(&current)
                .iter()
                .any(|(label, _, disabled)| label == "Next" && *disabled),
            "the last page cannot go on: {current}"
        );
        assert!(
            walked > 2,
            "the fixture must actually cross pages, not stop at the second: {walked}"
        );
        assert!(
            current.to_string().contains(" of "),
            "reaching the end of the results is what makes their number known: {current}"
        );

        // And Previous comes back to where it started.
        let writes = discord.count("PATCH");
        service
            .handle(&page_click("202", &second, ACTOR, "Previous"))
            .await;
        quiet(&discord).await;
        assert_eq!(
            discord.count("PATCH"),
            writes,
            "a button from an earlier revision writes nothing"
        );
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 4)]
    #[requires(true)]
    #[ensures(true)]
    async fn a_page_button_from_an_older_revision_turns_nothing() {
        let discord = FakeDiscord::start().await;
        let service = new_service(&discord);
        discord.publish(json!({ "id": MESSAGE, "components": [], "attachments": [] }));
        service
            .handle(&command("210", "vlacku", vec![option("query", "kla*")]))
            .await;
        settle(&discord, 1).await;
        let first = discord.original().expect("a result");

        // The reader turns a page, so the message moves to a new revision.
        let writes = discord.count("PATCH");
        service
            .handle(&page_click("211", &first, ACTOR, "Next"))
            .await;
        settle(&discord, writes + 1).await;
        let second = discord.original().expect("a result");

        // A button from the first version is now stale, and says so instead of
        // applying to what the message shows now.
        let writes = discord.count("PATCH");
        // Discord hands the click the message it was drawn on, so the stale
        // revision is only visible once the work reads the message as it
        // stands. The click is acknowledged and then refuses privately.
        service
            .handle(&page_click("212", &first, ACTOR, "Next"))
            .await;
        quiet(&discord).await;
        let complaint = discord.private_messages().join("\n");
        assert!(
            complaint.contains("changed while that page was loading"),
            "the reader is told why: {complaint}"
        );
        assert_eq!(discord.count("PATCH"), writes, "nothing was written");
        assert_eq!(
            discord.original().expect("a result"),
            second,
            "the message is untouched"
        );
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 4)]
    #[requires(true)]
    #[ensures(true)]
    async fn only_the_reader_who_asked_may_turn_the_pages() {
        let discord = FakeDiscord::start().await;
        let service = new_service(&discord);
        discord.publish(json!({ "id": MESSAGE, "components": [], "attachments": [] }));
        service
            .handle(&command("220", "vlacku", vec![option("query", "kla*")]))
            .await;
        settle(&discord, 1).await;
        let published = discord.original().expect("a result");

        let writes = discord.count("PATCH");
        let response = service
            .handle(&page_click("221", &published, OTHER_ACTOR, "Next"))
            .await;
        assert!(
            ephemeral_text(&response).contains("Only the person who ran this command"),
            "{response:?}"
        );
        quiet(&discord).await;
        assert_eq!(discord.count("PATCH"), writes, "nothing was written");
        assert_eq!(discord.original().expect("a result"), published);
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 4)]
    #[requires(true)]
    #[ensures(true)]
    async fn a_meaning_search_pages_through_its_ranking_rather_than_one_fetch() {
        let discord = FakeDiscord::start().await;
        let service = new_service(&discord);
        discord.publish(json!({ "id": MESSAGE, "components": [], "attachments": [] }));
        service
            .handle(&command(
                "280",
                "vlacku",
                vec![option("query", "destination"), option("mode", "meaning")],
            ))
            .await;
        settle(&discord, 1).await;
        let first = discord.original().expect("a result");
        // Meaning search needs the embedding index, which a deployment may not
        // have. Where it is missing the result says so and there is nothing to
        // page; that is the honest outcome and not this test's subject.
        if first.to_string().contains("Meaning search") {
            assert!(
                page_buttons(&first).is_empty(),
                "an unavailable search offers no pages: {first}"
            );
            return;
        }
        assert!(
            first.to_string().contains("1-5"),
            "the first page shows the first five: {first}"
        );

        // A ranking is fetched up to the end of the page being shown, so the
        // pages walk through the ranking rather than stopping at the first
        // fetch. Before this was so, page two held one result and offered
        // nothing further.
        let mut current = first;
        for (step, expected) in ["6-10", "11-15", "16-20"].iter().enumerate() {
            assert!(
                page_buttons(&current)
                    .iter()
                    .any(|(label, _, disabled)| label == "Next" && !disabled),
                "there is another page to turn to: {current}"
            );
            let writes = discord.count("PATCH");
            service
                .handle(&page_click(&format!("28{step}1"), &current, ACTOR, "Next"))
                .await;
            settle(&discord, writes + 1).await;
            current = discord.original().expect("a result");
            assert!(
                current.to_string().contains(expected),
                "page {} shows results {expected}: {current}",
                step + 2
            );
            assert!(
                !current.to_string().contains(&format!("{expected} of ")),
                "a ranking cut to this page is not a count of the results: {current}"
            );
        }
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 4)]
    #[requires(true)]
    #[ensures(true)]
    async fn a_book_meaning_search_pages_through_its_ranking_too() {
        let discord = FakeDiscord::start().await;
        let service = new_service(&discord);
        discord.publish(json!({ "id": MESSAGE, "components": [], "attachments": [] }));
        service
            .handle(&command(
                "290",
                "cukta",
                vec![option("query", "tanru"), option("mode", "meaning")],
            ))
            .await;
        settle(&discord, 1).await;
        let first = discord.original().expect("a result");
        // As with the dictionary, a deployment without the embedding index
        // says so and has nothing to page.
        if first.to_string().contains("Meaning search") {
            assert!(page_buttons(&first).is_empty(), "{first}");
            return;
        }
        assert!(first.to_string().contains("1-5"), "{first}");

        let writes = discord.count("PATCH");
        service
            .handle(&page_click("291", &first, ACTOR, "Next"))
            .await;
        settle(&discord, writes + 1).await;
        let second = discord.original().expect("a result");
        assert!(
            second.to_string().contains("6-10"),
            "the book's ranking pages on: {second}"
        );
        assert!(
            !second.to_string().contains("6-10 of "),
            "a ranking cut to this page is not a count of the results: {second}"
        );
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 4)]
    #[requires(true)]
    #[ensures(true)]
    async fn the_same_page_click_delivered_twice_turns_one_page() {
        let discord = FakeDiscord::start().await;
        let service = new_service(&discord);
        discord.publish(json!({ "id": MESSAGE, "components": [], "attachments": [] }));
        service
            .handle(&command("240", "vlacku", vec![option("query", "kla*")]))
            .await;
        settle(&discord, 1).await;
        let published = discord.original().expect("a result");

        // Discord may deliver one click more than once. The second delivery is
        // the same turn, so it is acknowledged and does no work of its own.
        let writes = discord.count("PATCH");
        let click = page_click("241", &published, ACTOR, "Next");
        let first = service.handle(&click).await;
        let second = service.handle(&click).await;
        // Both are acknowledged the same way — the page is turned by editing
        // the message, not by answering the click — so what says the work
        // happened once is that the message was written once.
        for response in [&first, &second] {
            assert!(
                matches!(response, InteractionResponse::DeferredUpdateMessage),
                "a page click is acknowledged and answered by the edit: {response:?}"
            );
        }
        settle(&discord, writes + 1).await;
        quiet(&discord).await;
        assert_eq!(
            discord.count("PATCH"),
            writes + 1,
            "one click turned one page, however many times it arrived"
        );
        let turned = discord.original().expect("a result");
        assert!(
            turned.to_string().contains("6-10"),
            "the page was turned once: {turned}"
        );
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 4)]
    #[requires(true)]
    #[ensures(true)]
    async fn a_page_turns_after_a_restart_with_every_cache_gone() {
        let discord = FakeDiscord::start().await;
        let service = new_service(&discord);
        discord.publish(json!({ "id": MESSAGE, "components": [], "attachments": [] }));
        service
            .handle(&command("250", "vlacku", vec![option("query", "kla*")]))
            .await;
        settle(&discord, 1).await;
        let published = discord.original().expect("a result");

        // Nothing about the published request is held in the process: a new
        // service, with no dedupe record and no memory of the command, turns
        // the page from what the message itself carries.
        let restarted = new_service(&discord);
        let writes = discord.count("PATCH");
        restarted
            .handle(&page_click("251", &published, ACTOR, "Next"))
            .await;
        settle(&discord, writes + 1).await;
        let turned = discord.original().expect("a result");
        assert!(
            turned.to_string().contains("6-10"),
            "the second page came from the message alone: {turned}"
        );
        assert!(
            page_buttons(&turned)
                .iter()
                .any(|(label, _, disabled)| label == "Previous" && !disabled),
            "and it can go back again"
        );
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 4)]
    #[requires(true)]
    #[ensures(true)]
    async fn only_the_steps_the_message_offers_are_taken() {
        let discord = FakeDiscord::start().await;
        let service = new_service(&discord);
        discord.publish(json!({ "id": MESSAGE, "components": [], "attachments": [] }));
        service
            .handle(&command("300", "vlacku", vec![option("query", "kla*")]))
            .await;
        settle(&discord, 1).await;
        let first = discord.original().expect("a result");

        // The first page's Previous is drawn greyed, so the message does not
        // offer it: a click on it anyway writes nothing.
        let writes = discord.count("PATCH");
        let response = service
            .handle(&page_click("301", &first, ACTOR, "Previous"))
            .await;
        assert!(
            ephemeral_text(&response).contains("not one this result offers"),
            "{response:?}"
        );

        // A page this message does not offer — properly written, this
        // revision, the reader who asked — is still not one of its buttons.
        let next = page_buttons(&first)
            .into_iter()
            .find(|(label, _, _)| label == "Next")
            .map(|(_, custom_id, _)| custom_id)
            .expect("a Next button");
        let (head, _) = next.rsplit_once('.').expect("a page control");
        let mut leap = page_click("302", &first, ACTOR, "Next");
        leap["data"]["custom_id"] = json!(format!("{head}.9"));
        let response = service.handle(&leap).await;
        assert!(
            ephemeral_text(&response).contains("not one this result offers"),
            "a page identifier the message never drew is not one of its buttons: {response:?}"
        );
        quiet(&discord).await;
        assert_eq!(discord.count("PATCH"), writes, "nothing was written");
        assert_eq!(discord.original().expect("a result"), first);
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn only_a_button_of_the_message_counts_as_one_it_offers() {
        let message = json!({
            "id": MESSAGE,
            "custom_id": "j1p.1.2",
            "resolved": { "buttons": [{ "type": 2, "custom_id": "j1p.1.9" }] },
            "components": [
                {
                    "type": 9,
                    "components": [{ "type": 10, "content": "a result" }],
                    "accessory": { "type": 2, "custom_id": "j1.gear", "style": 2 }
                },
                {
                    "type": 1,
                    "components": [
                        { "type": 2, "custom_id": "j1p.1.1", "label": "Previous", "disabled": true },
                        { "type": 2, "custom_id": "j1p.1.3", "label": "Next" },
                        { "type": 3, "custom_id": "j1p.1.4", "options": [] }
                    ]
                }
            ]
        });

        // A button of the message, not greyed.
        assert!(DiscordService::message_offers(&message, "j1p.1.3"));
        assert!(DiscordService::message_offers(&message, "j1.gear"));
        // Greyed, so not offered.
        assert!(!DiscordService::message_offers(&message, "j1p.1.1"));
        // A select carrying the identifier is not a button.
        assert!(!DiscordService::message_offers(&message, "j1p.1.4"));
        // The identifier elsewhere in the message is not a component of it.
        assert!(!DiscordService::message_offers(&message, "j1p.1.2"));
        assert!(!DiscordService::message_offers(&message, "j1p.1.9"));
        // Nothing offers nothing.
        assert!(!DiscordService::message_offers(&message, ""));
        assert!(!DiscordService::message_offers(&json!({}), "j1p.1.3"));
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 4)]
    #[requires(true)]
    #[ensures(true)]
    async fn a_greyed_button_and_a_result_without_a_pager_turn_nothing() {
        let discord = FakeDiscord::start().await;
        let service = new_service(&discord);
        discord.publish(json!({ "id": MESSAGE, "components": [], "attachments": [] }));
        service
            .handle(&command("320", "vlacku", vec![option("query", "kla*")]))
            .await;
        settle(&discord, 1).await;

        // Walk to the last page, where Next is drawn greyed. Discord will not
        // send that click, but one that arrives anyway asks for a page the
        // message does not offer, whatever its identifier says.
        let mut current = discord.original().expect("a result");
        for step in 0..40 {
            if page_buttons(&current)
                .iter()
                .any(|(label, _, disabled)| label == "Next" && *disabled)
            {
                break;
            }
            let writes = discord.count("PATCH");
            service
                .handle(&page_click(&format!("32{step}1"), &current, ACTOR, "Next"))
                .await;
            settle(&discord, writes + 1).await;
            current = discord.original().expect("a result");
        }
        let writes = discord.count("PATCH");
        let response = service
            .handle(&page_click("330", &current, ACTOR, "Next"))
            .await;
        assert!(
            ephemeral_text(&response).contains("not one this result offers"),
            "a greyed button is not offered: {response:?}"
        );
        quiet(&discord).await;
        assert_eq!(discord.count("PATCH"), writes, "nothing was written");

        // A result that fits one page carries no buttons at all, so a page
        // identifier for it — properly written, this revision, this reader —
        // is not one of its own either.
        *discord.state.original.lock().expect("state") =
            Some(json!({ "id": MESSAGE, "components": [], "attachments": [] }));
        service
            .handle(&command("340", "vlacku", vec![option("query", "klama")]))
            .await;
        settle(&discord, discord.count("PATCH") + 1).await;
        let single = discord.original().expect("a result");
        assert!(
            page_buttons(&single).is_empty(),
            "the fixture must be a single page: {single}"
        );
        let header = RequestHeader::of_message(&single).expect("a header");
        let invented = PageControl {
            from_revision: header.revision,
            target: PageNumber::new(2).expect("page"),
        };
        let mut click = page_click_with_id("341", &single, ACTOR, &invented.encode());
        click["id"] = json!("341");
        let writes = discord.count("PATCH");
        let response = service.handle(&mut click).await;
        assert!(
            ephemeral_text(&response).contains("not one this result offers"),
            "a result with no buttons offers none: {response:?}"
        );
        quiet(&discord).await;
        assert_eq!(discord.count("PATCH"), writes, "nothing was written");
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 4)]
    #[requires(true)]
    #[ensures(true)]
    async fn two_different_clicks_on_one_result_turn_one_page() {
        let discord = FakeDiscord::start().await;
        let service = new_service(&discord);
        discord.publish(json!({ "id": MESSAGE, "components": [], "attachments": [] }));
        service
            .handle(&command("310", "vlacku", vec![option("query", "kla*")]))
            .await;
        settle(&discord, 1).await;
        let first = discord.original().expect("a result");

        // The reader turns to the second page, where both directions are
        // offered and enabled.
        let writes = discord.count("PATCH");
        service
            .handle(&page_click("311", &first, ACTOR, "Next"))
            .await;
        settle(&discord, writes + 1).await;
        let second = discord.original().expect("a result");
        assert!(second.to_string().contains("6-10"), "{second}");

        // Now two different clicks on that same result arrive at once, each a
        // button the message offers. They are different interactions, so
        // neither is a repeat of the other: the first to take the message's
        // lock turns its page, and the second finds the message has moved on
        // and turns nothing.
        let writes = discord.count("PATCH");
        let forward = page_click("312", &second, ACTOR, "Next");
        let back = page_click("313", &second, ACTOR, "Previous");
        let (forward_response, back_response) =
            tokio::join!(service.handle(&forward), service.handle(&back));
        for response in [&forward_response, &back_response] {
            assert!(
                matches!(response, InteractionResponse::DeferredUpdateMessage),
                "both clicks are taken up: {response:?}"
            );
        }
        settle(&discord, writes + 1).await;
        quiet(&discord).await;
        assert_eq!(
            discord.count("PATCH"),
            writes + 1,
            "one of the two clicks wrote, and only one"
        );
        let turned = discord.original().expect("a result");
        let shows = turned.to_string();
        assert!(
            shows.contains("11-15") || shows.contains("1-5"),
            "the page that was turned is one of the two that were asked for: {turned}"
        );
        let told = discord.private_messages().join("\n");
        assert!(
            told.contains("changed while that page was loading"),
            "the click that lost the race is told, privately: {told}"
        );
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 4)]
    #[requires(true)]
    #[ensures(true)]
    async fn an_identifier_that_claims_to_be_a_page_button_and_is_not_turns_nothing() {
        let discord = FakeDiscord::start().await;
        let service = new_service(&discord);
        discord.publish(json!({ "id": MESSAGE, "components": [], "attachments": [] }));
        service
            .handle(&command("260", "vlacku", vec![option("query", "kla*")]))
            .await;
        settle(&discord, 1).await;
        let published = discord.original().expect("a result");
        let real = page_buttons(&published)
            .into_iter()
            .find(|(label, _, _)| label == "Next")
            .map(|(_, custom_id, _)| custom_id)
            .expect("a Next button");

        // The same numbers written differently, and a page that is not a
        // number at all. None of these was written by this build, so none of
        // them is read as an instruction, and none falls through to the gear.
        let (head, page) = real.rsplit_once('.').expect("a page control");
        let writes = discord.count("PATCH");
        for forged in [
            format!("{head}.0{page}"),
            format!("{head}.+{page}"),
            format!("{head}.0"),
            format!("{head}."),
            format!("{head}.{page}."),
        ] {
            let mut click = page_click("270", &published, ACTOR, "Next");
            click["id"] = json!(format!("27{}", forged.len()));
            click["data"]["custom_id"] = json!(forged.clone());
            let response = service.handle(&click).await;
            assert!(
                ephemeral_text(&response).contains("not one this build wrote"),
                "{forged}: {response:?}"
            );
        }
        quiet(&discord).await;
        assert_eq!(discord.count("PATCH"), writes, "nothing was written");
        assert_eq!(discord.original().expect("a result"), published);
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 4)]
    #[requires(true)]
    #[ensures(true)]
    async fn a_result_that_fits_one_page_offers_no_pager() {
        let discord = FakeDiscord::start().await;
        let service = new_service(&discord);
        discord.publish(json!({ "id": MESSAGE, "components": [], "attachments": [] }));
        // A parse is one result, never a list.
        service
            .handle(&command("230", "gentufa", vec![option("text", "mi klama")]))
            .await;
        settle(&discord, 1).await;
        assert!(
            page_buttons(&discord.original().expect("a result")).is_empty(),
            "a result with no pages carries no page buttons"
        );
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 4)]
    #[requires(true)]
    #[ensures(true)]
    async fn a_long_refusal_is_shown_in_part_and_travels_whole() {
        // Fifty malformed records are a short request and a long complaint.
        // The reader needs every complaint to fix the request, so the message
        // shows what fits and carries the rest as a file.
        let discord = FakeDiscord::start().await;
        let service = new_service(&discord);
        discord.publish(json!({ "id": MESSAGE, "components": [], "attachments": [] }));
        let records = (0..50)
            .map(|index| format!("??{index}"))
            .collect::<Vec<_>>()
            .join(", ");
        service
            .handle(&command(
                "160",
                "gimfihi",
                vec![option("sources", &records)],
            ))
            .await;
        settle(&discord, 1).await;

        let message = discord.original().expect("a result");
        let shown = message.to_string();
        assert!(shown.contains("Not run"), "the message says so: {shown}");
        let attachments = message["attachments"]
            .as_array()
            .expect("attachments")
            .iter()
            .filter_map(|attachment| attachment["filename"].as_str())
            .collect::<Vec<_>>();
        assert!(
            attachments.contains(&"jbotci-result.txt"),
            "the whole complaint travels with the message: {attachments:?}"
        );
        let uploaded = discord
            .requests()
            .iter()
            .rev()
            .find(|(what, _)| what.starts_with("PATCH"))
            .and_then(|(_, body)| body.get("jbotci_result").cloned())
            .and_then(|value| value.as_str().map(str::to_owned))
            .expect("the attached result");
        assert!(
            uploaded.contains("??49"),
            "the last complaint survives: {}",
            &uploaded[uploaded.len().saturating_sub(200)..]
        );
        // And the form still reopens on the request that caused it.
        let response = service
            .handle(&component_interaction("161", &message, ACTOR))
            .await;
        assert!(
            matches!(response, InteractionResponse::Modal(_)),
            "{response:?}"
        );
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 4)]
    #[requires(true)]
    #[ensures(true)]
    async fn choosing_the_other_scorer_reranks_the_same_result() {
        let discord = FakeDiscord::start().await;
        let service = new_service(&discord);
        discord.publish(json!({ "id": MESSAGE, "components": [], "attachments": [] }));
        service
            .handle(&command(
                "180",
                "gimfihi",
                vec![option("sources", "eng:5:go, spa:3:[ir], cmn:4:[t͡ɕʰy]")],
            ))
            .await;
        settle(&discord, 1).await;
        let published = discord.original().expect("a result");
        let classic = published.to_string();
        assert!(
            classic.contains("classic") && classic.contains("gorci"),
            "a result says how it was ranked, and by what: {classic}"
        );

        // The form carries the choice; taking the other one edits the same
        // message, and what comes back is a different ranking rather than the
        // same one relabelled.
        let modal = modal_of(
            service
                .handle(&component_interaction("181", &published, ACTOR))
                .await,
        );
        let writes = discord.count("PATCH");
        service
            .handle(&submission(
                "182",
                &modal,
                &published,
                ACTOR,
                &[("scorer", json!(["phonetic"]))],
            ))
            .await;
        settle(&discord, writes + 1).await;
        let edited = discord.original().expect("an edit").to_string();
        assert!(
            edited.contains("phonetic") && edited.contains("cigro"),
            "the phonetic scorer ranked it: {edited}"
        );
        assert!(
            !edited.contains("gorci"),
            "and the classic ranking is gone: {edited}"
        );

        // The form reopens on what the message now says, and the link that
        // comes with it opens the app on the same scorer.
        let now = discord.original().expect("the edited result");
        let reopened = modal_of(
            service
                .handle(&component_interaction("183", &now, ACTOR))
                .await,
        );
        let chosen = reopened
            .components
            .iter()
            .find_map(|component| match component {
                ModalComponent::Label(labeled) => match &labeled.control {
                    ModalControl::StringSelect(select) if select.custom_id.as_str() == "scorer" => {
                        Some(
                            select
                                .options
                                .iter()
                                .filter(|option| option.default)
                                .map(|option| option.value.clone())
                                .collect::<Vec<_>>(),
                        )
                    }
                    _ => None,
                },
                _ => None,
            })
            .expect("a scorer control");
        assert_eq!(chosen, vec!["phonetic".to_owned()]);
        let ModalComponent::TextDisplay(link) = reopened.components.first() else {
            panic!("the first component is the link");
        };
        assert!(
            link.content.contains("phonetic"),
            "the link opens the app on the same scorer: {}",
            link.content
        );
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 4)]
    #[requires(true)]
    #[ensures(true)]
    async fn a_long_private_note_stays_within_what_discord_accepts() {
        // The same complaint on an edit is told privately instead, and a
        // note Discord will not accept is no note at all.
        let discord = FakeDiscord::start().await;
        let service = new_service(&discord);
        discord.publish(json!({ "id": MESSAGE, "components": [], "attachments": [] }));
        service.handle(&command("170", "gimfihi", Vec::new())).await;
        settle(&discord, 1).await;
        let published = discord.original().expect("a result");
        let modal = modal_of(
            service
                .handle(&component_interaction("171", &published, ACTOR))
                .await,
        );
        let records = (0..50)
            .map(|index| format!("??{index}"))
            .collect::<Vec<_>>()
            .join(", ");
        let writes = discord.count("PATCH");
        service
            .handle(&submission(
                "172",
                &modal,
                &published,
                ACTOR,
                &[("sources", json!(records))],
            ))
            .await;
        quiet(&discord).await;

        assert_eq!(discord.count("PATCH"), writes, "the result is unchanged");
        let notes = discord.private_messages();
        let note = notes.last().expect("a private note");
        assert!(note.contains("Nothing was changed"), "{note}");
        // The fake refuses an oversized note the way Discord does, so getting
        // here proves the bound. What matters beyond that is that nothing was
        // lost: the last of the fifty complaints is in the delivered file.
        let details = discord.private_details();
        let detail = details.last().expect("the whole complaint");
        assert!(
            detail.contains("??49"),
            "the last complaint survives: {}",
            &detail[detail.len().saturating_sub(200)..]
        );
        assert!(
            detail.len() > note.len(),
            "the file carries more than the message shows"
        );
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 4)]
    #[requires(true)]
    #[ensures(true)]
    async fn a_form_whose_owner_disagrees_with_the_message_changes_nothing() {
        // Both the form's identifier and the message name who asked, and both
        // arrive from Discord. If they disagree, the message is the published
        // one and wins: a form claiming otherwise applies nothing.
        let discord = FakeDiscord::start().await;
        let service = new_service(&discord);
        discord.publish(json!({ "id": MESSAGE, "components": [], "attachments": [] }));
        service
            .handle(&command("150", "vlasei", vec![option("text", "mi klama")]))
            .await;
        settle(&discord, 1).await;
        let published = discord.original().expect("a result");
        let modal = modal_of(
            service
                .handle(&component_interaction("151", &published, ACTOR))
                .await,
        );

        // The same message as if someone else had published it.
        let gear = gear_of(&published);
        let mut fields = gear.split('.').collect::<Vec<_>>();
        assert_eq!(fields[5], ACTOR, "the header's initiator: {gear}");
        fields[5] = OTHER_ACTOR;
        let theirs = fields.join(".");
        let their_message: Value =
            serde_json::from_str(&published.to_string().replace(&gear, &theirs))
                .expect("a message");
        discord.publish(their_message.clone());

        let writes = discord.count("PATCH");
        let response = service
            .handle(&submission("152", &modal, &their_message, ACTOR, &[]))
            .await;
        assert!(
            ephemeral_text(&response).contains("Only the person who ran this command"),
            "{response:?}"
        );
        quiet(&discord).await;
        assert_eq!(discord.count("PATCH"), writes, "nothing was applied");
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 4)]
    #[requires(true)]
    #[ensures(true)]
    async fn a_result_recomputed_by_another_build_says_so() {
        // A message published before an upgrade is edited afterwards: the
        // text and any image are the work of a different version than the one
        // the reader was looking at, and the result says so once.
        let discord = FakeDiscord::start().await;
        let service = new_service(&discord);
        discord.publish(json!({ "id": MESSAGE, "components": [], "attachments": [] }));
        service
            .handle(&command("140", "vlasei", vec![option("text", "mi klama")]))
            .await;
        settle(&discord, 1).await;
        let published = discord.original().expect("a result");

        // The same message as an older build would have left it: only its
        // build tag differs, and the tag is outside the source digest.
        let gear = gear_of(&published);
        let mut fields = gear.split('.').collect::<Vec<_>>();
        assert_eq!(fields.len(), 8, "the header fields: {gear}");
        assert_eq!(fields[6], BuildTag::current().as_str());
        fields[6] = "oldbuild";
        let older = fields.join(".");
        let older_message: Value =
            serde_json::from_str(&published.to_string().replace(&gear, &older)).expect("a message");
        discord.publish(older_message.clone());

        let modal = modal_of(
            service
                .handle(&component_interaction("141", &older_message, ACTOR))
                .await,
        );
        let writes = discord.count("PATCH");
        service
            .handle(&submission("142", &modal, &older_message, ACTOR, &[]))
            .await;
        settle(&discord, writes + 1).await;
        let edited = discord.original().expect("a result").to_string();
        assert!(
            edited.contains("Recomputed by a different version"),
            "the reader is told which version made this: {edited}"
        );

        // An ordinary edit of a result from this build says nothing of the
        // kind: the note carries information, so it must be rare.
        let current = discord.original().expect("a result");
        let modal = modal_of(
            service
                .handle(&component_interaction("143", &current, ACTOR))
                .await,
        );
        let writes = discord.count("PATCH");
        service
            .handle(&submission("144", &modal, &current, ACTOR, &[]))
            .await;
        settle(&discord, writes + 1).await;
        let edited = discord.original().expect("a result").to_string();
        assert!(
            !edited.contains("Recomputed by a different version"),
            "no version note on an ordinary edit: {edited}"
        );
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 4)]
    #[requires(true)]
    #[ensures(true)]
    async fn a_replay_that_cannot_read_the_message_writes_nothing() {
        // A redelivered command re-checks the message immediately before
        // writing. When that check cannot be made at all, whether a result is
        // already published is unknown, and writing would risk replacing an
        // edited result with the command's defaults.
        let discord = FakeDiscord::start().await;
        let service = new_service(&discord);
        discord.publish(json!({ "id": MESSAGE, "components": [], "attachments": [] }));
        discord.plan_reads(&[Reply::Ok, Reply::Failed]);
        service
            .handle(&command("120", "vlasei", vec![option("text", "mi klama")]))
            .await;
        quiet(&discord).await;

        assert_eq!(discord.count("PATCH"), 0, "nothing was written");
        let complaint = discord.private_messages().join("\n");
        assert!(
            complaint.contains("nothing was written"),
            "the reader is told why: {complaint}"
        );
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 4)]
    #[requires(true)]
    #[ensures(true)]
    async fn a_write_that_outlives_its_caller_keeps_the_message_locked() {
        // The message lock belongs to the work that writes, not to whoever
        // waits for it. Were it released when the caller gave up, a second
        // submission could read the old revision, publish, and then be
        // silently overwritten by the first write landing late.
        use std::sync::atomic::{AtomicBool, Ordering};

        let locks = Arc::new(MessageLocks::new(4, 4));
        let governor = WorkGovernor::new(Default::default());
        let recent = Arc::new(RecentInteractions::new(4));
        let message = Snowflake::parse(MESSAGE).expect("snowflake");
        let Admission::First(ticket) = recent.admit(&message) else {
            panic!("first delivery");
        };
        let guard = locks
            .acquire(&message, Instant::now() + Duration::from_secs(5))
            .await
            .expect("the lock");
        let keepalive: WorkKeepalive = Arc::new(Retained {
            _ticket: Arc::new(ticket),
            _guard: guard,
        });

        let running = Arc::new(AtomicBool::new(false));
        let release = Arc::new(AtomicBool::new(false));
        let started = Arc::clone(&running);
        let held = Arc::clone(&release);
        let outcome = governor
            .run_fetch_keeping(
                Instant::now() + Duration::from_millis(100),
                Some(keepalive.clone()),
                move || {
                    started.store(true, Ordering::Release);
                    while !held.load(Ordering::Acquire) {
                        std::thread::sleep(Duration::from_millis(5));
                    }
                },
            )
            .await;
        assert!(outcome.is_err(), "the caller stopped waiting for the write");
        assert!(running.load(Ordering::Acquire), "the write had started");

        // The waiting side lets go of everything it holds; the write has not.
        drop(keepalive);
        assert!(
            locks
                .acquire(&message, Instant::now() + Duration::from_millis(200))
                .await
                .is_err(),
            "no later edit may enter while the write is in flight"
        );
        release.store(true, Ordering::Release);
        assert!(
            locks
                .acquire(&message, Instant::now() + Duration::from_secs(5))
                .await
                .is_ok(),
            "the message is free once the work really settles"
        );
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 3)]
    #[requires(true)]
    #[ensures(true)]
    async fn a_delivery_stands_until_its_work_really_ends() {
        // A handler that gives up waiting does not end the work it started:
        // the job keeps running, and the delivery it stands for must keep
        // standing, or the same interaction could be admitted and run again.
        let recent = Arc::new(RecentInteractions::new(4));
        let governor = WorkGovernor::new(Default::default());
        let id = Snowflake::parse("123456789012345678").expect("snowflake");
        let Admission::First(ticket) = recent.admit(&id) else {
            panic!("the first delivery");
        };
        let ticket = Arc::new(ticket);
        let keepalive: WorkKeepalive = ticket.clone();
        let (release_tx, release_rx) = std::sync::mpsc::channel::<()>();
        let timed_out = governor
            .run_compute_keeping(
                Instant::now() + Duration::from_millis(50),
                Some(keepalive),
                move || {
                    let _ = release_rx.recv();
                    "done"
                },
            )
            .await;
        assert!(timed_out.is_err(), "the caller gave up");
        // The handler drops its own handle and reports privately, as it does
        // after a timeout.
        drop(ticket);
        assert_eq!(
            recent.active_count(),
            1,
            "the delivery still stands while its work runs"
        );
        assert!(
            matches!(recent.admit(&id), Admission::Duplicate),
            "a redelivery does not start the work again"
        );

        release_tx.send(()).expect("the job is waiting");
        for _ in 0..200 {
            if recent.active_count() == 0 {
                break;
            }
            tokio::time::sleep(Duration::from_millis(10)).await;
        }
        assert_eq!(
            recent.active_count(),
            0,
            "and it settles when the work ends"
        );
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 4)]
    #[requires(true)]
    #[ensures(true)]
    async fn every_tool_publishes_a_result_with_a_working_form() {
        let discord = FakeDiscord::start().await;
        let service = new_service(&discord);
        for (index, (tool, options)) in [
            (DiscordTool::Gentufa, vec![option("text", "mi klama")]),
            (DiscordTool::Vlasei, vec![option("text", "mi klama")]),
            (DiscordTool::Vlatai, vec![option("text", "klama")]),
            (DiscordTool::Vlacku, vec![option("query", "klama")]),
            (
                DiscordTool::Cukta,
                vec![option("query", "tanru"), option("mode", "word")],
            ),
            (DiscordTool::Jvozba, vec![option("parts", "klama bajra")]),
            (DiscordTool::Gimfihi, vec![option("sources", "eng:5:go")]),
        ]
        .into_iter()
        .enumerate()
        {
            *discord.state.original.lock().expect("state") =
                Some(json!({ "id": MESSAGE, "components": [], "attachments": [] }));
            let writes = discord.count("PATCH");
            service
                .handle(&command(&format!("2{index}0"), tool.name(), options))
                .await;
            settle(&discord, writes + 1).await;
            let published = discord.original().expect("a result");
            assert!(
                gear_of(&published).starts_with(&format!("j1.{}", tool.code())),
                "{tool}: {}",
                gear_of(&published)
            );
            let modal = modal_of(
                service
                    .handle(&component_interaction(
                        &format!("2{index}1"),
                        &published,
                        ACTOR,
                    ))
                    .await,
            );
            assert_eq!(
                modal.components.first().is_text_display(),
                tool.has_web_page(),
                "{tool}: the link is there exactly when the tool has a page"
            );
            let writes = discord.count("PATCH");
            let response = service
                .handle(&submission(
                    &format!("2{index}2"),
                    &modal,
                    &published,
                    ACTOR,
                    &[],
                ))
                .await;
            assert!(
                matches!(response, InteractionResponse::DeferredUpdateMessage),
                "{tool}: {response:?}"
            );
            settle(&discord, writes + 1).await;
            assert!(
                gear_of(&discord.original().expect("an edit")).contains(".2."),
                "{tool}: the revision moved"
            );
        }
    }
}
