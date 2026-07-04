use std::sync::Arc;

use poise::serenity_prelude as serenity;

use crate::context::BotData;
use crate::util::respond_ephemeral;

/// "Join Ticket" button on a staff-alert card (staff only). Adding a thread
/// member is idempotent, so this doubles as "take me to the ticket" for staff
/// who are already in.
pub async fn handle(
    ctx: &serenity::Context,
    data: &Arc<BotData>,
    ci: &serenity::ComponentInteraction,
    ticket_id: i64,
) -> Result<(), anyhow::Error> {
    let Some(guild_id) = ci.guild_id else {
        return Ok(());
    };
    if !crate::permissions::is_mod_staff(ctx, data, guild_id, ci.user.id).await {
        respond_ephemeral(ctx, ci, "Only staff can join tickets from here.").await;
        return Ok(());
    }

    let Some(ticket) = data.db.get_ticket_by_id(ticket_id).await? else {
        respond_ephemeral(ctx, ci, "That ticket no longer exists.").await;
        return Ok(());
    };
    let Ok(thread_id) = ticket.thread_id.parse::<u64>() else {
        respond_ephemeral(ctx, ci, "That ticket's thread can't be found.").await;
        return Ok(());
    };

    let thread = serenity::ThreadId::new(thread_id);
    if let Err(e) = ctx.http.add_thread_channel_member(thread, ci.user.id).await {
        respond_ephemeral(
            ctx,
            ci,
            &format!("I couldn't add you to <#{}> — the thread may be archived or deleted. ({e})", thread_id),
        )
        .await;
        return Ok(());
    }
    respond_ephemeral(ctx, ci, &format!("🎟️ You're in — <#{}>", thread_id)).await;
    Ok(())
}
