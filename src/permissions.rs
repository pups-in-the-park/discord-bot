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

/// Ensures `ctx.author()` may manage `role` — it must sit strictly below their
/// highest role (the guild owner is exempt). Without this, any `MANAGE_ROLES`
/// holder could grant or remove roles at or above their own rank, since Discord
/// only blocks assigning roles above the *bot*.
pub async fn ensure_can_manage_role(
    ctx: &Context<'_>,
    role: &serenity::Role,
) -> Result<(), BotError> {
    let guild_id = role.guild_id;

    // The guild owner sits above the role hierarchy. Copy the id out before any
    // await so the cache guard from `ctx.guild()` isn't held across a suspension;
    // on a cache miss, fetch — a role-less owner would otherwise fail the
    // hierarchy check below (author_top = 0).
    let owner_id = match ctx.guild().map(|g| g.owner_id) {
        Some(id) => Some(id),
        None => guild_id
            .to_partial_guild(ctx.serenity_context())
            .await
            .ok()
            .map(|g| g.owner_id),
    };
    if owner_id == Some(ctx.author().id) {
        return Ok(());
    }

    let author = guild_id
        .member(ctx.serenity_context(), ctx.author().id)
        .await
        .map_err(|_| BotError::user("Could not verify your roles."))?;
    let roles = guild_id
        .roles(&ctx.serenity_context().http)
        .await
        .map_err(|_| BotError::user("Could not load this server's roles."))?;
    let author_top = author
        .roles
        .iter()
        .filter_map(|r| roles.get(r))
        .map(|r| r.position)
        .max()
        .unwrap_or(0);

    if role.position >= author_top {
        return Err(BotError::user(
            "You can only manage roles positioned below your own highest role.",
        ));
    }
    Ok(())
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
