use poise::serenity_prelude as serenity;

use crate::context::{Context, Error};
use crate::ids::{cid_report_msg_modal, REPORT_REASON_FIELD};

/// Report this message to the moderation team.
#[poise::command(context_menu_command = "Report Message", guild_only)]
pub async fn report_message(ctx: Context<'_>, msg: serenity::Message) -> Result<(), Error> {
    if msg.author.bot {
        return Err(Error::user("You can't report a bot's message."));
    }
    if msg.author.id == ctx.author().id {
        return Err(Error::user("You can't report your own message."));
    }

    let modal_id = cid_report_msg_modal(msg.id.get(), msg.channel_id.get(), msg.author.id.get());
    crate::util::modal_response(
        ctx,
        serenity::CreateModal::new(&modal_id, "🚨 Report Message").components(vec![
            serenity::CreateActionRow::InputText(
                serenity::CreateInputText::new(
                    serenity::InputTextStyle::Paragraph,
                    "Why are you reporting this?",
                    REPORT_REASON_FIELD,
                )
                .required(false)
                .placeholder("Be specific about what violates the rules…"),
            ),
        ]),
    )
    .await?;
    Ok(())
}
