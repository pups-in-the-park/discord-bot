//! Staff permission checks shared across features.

use std::sync::Arc;

use poise::serenity_prelude as serenity;

use crate::context::{BotData, BotError, Context};

/// Returns true if the user holds at least one of the global mod staff roles.
pub async fn is_mod_staff(
    ctx: &serenity::Context,
    data: &Arc<BotData>,
    guild_id: serenity::GuildId,
    user_id: serenity::UserId,
) -> bool {
    let cfg = match data.db.get_or_create_mod_config(&guild_id.to_string()).await {
        Ok(c) => c,
        Err(_) => return false,
    };
    let roles = cfg.staff_roles();

    // Prefer the cache to avoid an HTTP round-trip on every permission check.
    let cached = ctx
        .cache
        .guild(guild_id)
        .and_then(|g| g.members.get(&user_id).cloned());
    let member = match cached {
        Some(m) => m,
        None => match guild_id.member(ctx, user_id).await {
            Ok(m) => m,
            Err(_) => return false,
        },
    };

    if roles.is_empty() {
        // No staff roles configured — fall back to Discord native permissions
        // so server admins can use staff commands before setup is complete.
        let Some(guild) = ctx.cache.guild(guild_id) else {
            return false;
        };
        let perms = guild.member_permissions(&member);
        return perms.contains(serenity::Permissions::ADMINISTRATOR)
            || perms.contains(serenity::Permissions::MANAGE_GUILD);
    }

    member.roles.iter().any(|r| roles.contains(&r.to_string()))
}

/// Validates that a moderation target is not a bot and not the invoker.
pub async fn validate_target(ctx: &Context<'_>, user: &serenity::User) -> Result<(), BotError> {
    if user.bot() {
        return Err(BotError::user("You cannot moderate bots."));
    }
    if user.id == ctx.author().id {
        return Err(BotError::user("You cannot moderate yourself."));
    }
    Ok(())
}

/// Returns `Err(BotError::User)` if the calling context's author is not mod staff.
pub async fn require_mod_staff(ctx: &Context<'_>) -> Result<(), BotError> {
    let Some(guild_id) = ctx.guild_id() else {
        return Err(BotError::user("This command must be used in a server."));
    };
    if !is_mod_staff(ctx.serenity_context(), &ctx.data(), guild_id, ctx.author().id).await {
        return Err(BotError::user("You don't have permission to use this command."));
    }
    Ok(())
}
