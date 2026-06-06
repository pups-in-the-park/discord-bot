use std::sync::Arc;

use poise::serenity_prelude as serenity;

use crate::context::BotData;
use crate::ids::parse_category_step2_modal;
use crate::ui::{read_checkbox_group, read_multi_select};
use crate::util::{modal_field, parse_hex_color, validate_thread_pattern};

use super::refresh_category_form;

/// `m:cat:create2:{cat_id}` — step 2 of the category-creation wizard: accent colour,
/// welcome message, thread pattern, and behaviour toggles. Then show the config form.
pub async fn handle(
    ctx: &serenity::Context,
    data: &Arc<BotData>,
    mi: &serenity::ModalInteraction,
) -> Result<(), anyhow::Error> {
    let cat_id = parse_category_step2_modal(&mi.data.custom_id)
        .ok_or_else(|| anyhow::anyhow!("Invalid category ID"))?;
    let cat = data
        .db
        .get_ticket_type_by_id(cat_id)
        .await?
        .ok_or_else(|| anyhow::anyhow!("Category not found"))?;

    let color = read_multi_select(&mi.data.components, "cat_color")
        .first()
        .and_then(|h| parse_hex_color(h))
        .unwrap_or_else(|| cat.color.clone());

    let welcome = modal_field(&mi.data.components, "cat_welcome").filter(|s| !s.is_empty()).map(str::to_string);

    // Thread pattern: validate; on a bad pattern, bail without applying anything.
    let pattern = modal_field(&mi.data.components, "cat_pattern")
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .unwrap_or(cat.thread_name_pattern.as_str())
        .to_string();
    if let Err(msg) = validate_thread_pattern(&pattern) {
        mi.create_response(
            &ctx.http,
            serenity::CreateInteractionResponse::Message(
                serenity::CreateInteractionResponseMessage::new()
                    .ephemeral(true)
                    .content(format!("{} Nothing was changed — open the category and try again.", msg)),
            ),
        )
        .await?;
        return Ok(());
    }

    let behaviour = read_checkbox_group(&mi.data.components, "cat_behaviour");
    let has_form = behaviour.iter().any(|v| v == "form");
    let auto_staff = behaviour.iter().any(|v| v == "staff");

    data.db
        .update_ticket_type(cat_id, &cat.label, cat.emoji.as_deref(), cat.description.as_deref(), &color, welcome.as_deref())
        .await?;
    data.db.set_ticket_type_thread_pattern(cat_id, &pattern).await?;
    data.db.set_ticket_type_has_form(cat_id, has_form).await?;
    data.db.set_ticket_type_auto_add_staff(cat_id, auto_staff).await?;

    refresh_category_form(ctx, data, mi, cat_id).await?;
    Ok(())
}
