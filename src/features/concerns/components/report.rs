use poise::serenity_prelude as serenity;

use crate::ids::{cid_concern_modal, CONCERN_REASON_FIELD};

/// "Report a Concern" button (from a denied appeal or no-action report DM):
/// open the concern modal.
pub async fn handle(
    ctx: &serenity::Context,
    ci: &serenity::ComponentInteraction,
    kind: String,
    source_id: i64,
    guild_id: u64,
) -> Result<(), anyhow::Error> {
    let modal_id = cid_concern_modal(&kind, source_id, guild_id);
    ci.create_response(
        ctx,
        serenity::CreateInteractionResponse::Modal(
            serenity::CreateModal::new(&modal_id, "⚠️ Report a Concern").components(vec![
                serenity::CreateActionRow::InputText(
                    serenity::CreateInputText::new(
                        serenity::InputTextStyle::Paragraph,
                        "Describe your concern",
                        CONCERN_REASON_FIELD,
                    )
                    .required(true)
                    .placeholder("Be detailed about why you think this decision was unfair…"),
                ),
            ]),
        ),
    )
    .await?;
    Ok(())
}
