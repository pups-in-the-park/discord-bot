use poise::serenity_prelude as serenity;

use crate::context::{Context, Error};
use crate::ids::{cid_report_msg_modal, REPORT_REASON_FIELD};

/// Report a message by link.
#[poise::command(slash_command, guild_only)]
pub async fn message(
    ctx: Context<'_>,
    #[description = "Message link (right-click → Copy Message Link)"] message_link: String,
) -> Result<(), Error> {
    let guild_id = ctx.guild_id().unwrap();

    let (link_guild, channel_id, message_id) = parse_message_link(&message_link).ok_or_else(|| {
        Error::user("That doesn't look like a message link. Right-click a message → Copy Message Link.")
    })?;

    if link_guild != guild_id.get() {
        return Err(Error::user("That message link is from a different server."));
    }

    // Resolve the message author (the report target) up front so we can reject bots
    // and self-reports, and so the submit handler gets a real target id.
    let msg = ctx
        .serenity_context()
        .http
        .get_message(serenity::ChannelId::new(channel_id), serenity::MessageId::new(message_id))
        .await
        .map_err(|_| {
            Error::user("I couldn't find that message — check the link and that I can see the channel.")
        })?;

    if msg.author.bot {
        return Err(Error::user("You can't report a bot's message."));
    }
    if msg.author.id == ctx.author().id {
        return Err(Error::user("You can't report your own message."));
    }

    // Encode the coordinates the submit handler expects: `m:rep:{msg}:{chan}:{author}`.
    let modal_id = cid_report_msg_modal(message_id, channel_id, msg.author.id.get());
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

/// Parse a Discord message link into `(guild_id, channel_id, message_id)`. Accepts
/// any host (discord.com, canary/ptb, discordapp.com) by reading the trailing
/// `channels/{guild}/{channel}/{message}` segments.
fn parse_message_link(link: &str) -> Option<(u64, u64, u64)> {
    let segs: Vec<&str> = link.trim().trim_end_matches('/').split('/').collect();
    let n = segs.len();
    if n < 3 {
        return None;
    }
    // For a full URL, require the "channels" marker before the three ids so a random
    // "a/b/c" string isn't accepted.
    if n >= 4 && segs[n - 4] != "channels" {
        return None;
    }
    let guild = segs[n - 3].parse().ok()?;
    let channel = segs[n - 2].parse().ok()?;
    let message = segs[n - 1].parse().ok()?;
    Some((guild, channel, message))
}
