use poise::serenity_prelude as serenity;

use crate::context::{Context, Error};
use crate::features::moderation::view::confirm_embed;

/// Autocomplete over the guild's ban list. Fetches up to 1000 bans (Discord's
/// per-page cap) and matches the partial against username, display name, or a
/// typed id prefix. The fetched list is cached briefly so a burst of keystrokes
/// doesn't re-download it each time. Best-effort: on any fetch failure the list
/// is just empty and a pasted id still works.
pub async fn autocomplete_banned_user<'a>(
    ctx: Context<'a>,
    partial: &'a str,
) -> serenity::CreateAutocompleteResponse<'a> {
    let Some(guild_id) = ctx.guild_id() else {
        return serenity::CreateAutocompleteResponse::new();
    };
    let data = ctx.data();
    let bans = match data.ban_cache.get(guild_id) {
        Some(bans) => bans,
        None => {
            // Snapshot the generation so `put` can detect an invalidation mid-fetch.
            let generation = data.ban_cache.generation();
            match guild_id
                .bans(&ctx.serenity_context().http, None, serenity::nonmax::NonMaxU16::new(1000))
                .await
            {
                Ok(fetched) => data.ban_cache.put(guild_id, fetched, generation),
                // Don't cache on failure — an empty list would then stick for the whole
                // TTL and blank out autocomplete even after Discord recovers.
                Err(_) => return serenity::CreateAutocompleteResponse::new(),
            }
        }
    };
    let p = partial.to_lowercase();
    let choices: Vec<_> = bans
        .iter()
        .filter(|b| {
            p.is_empty()
                || b.user.name.to_lowercase().contains(&p)
                || b.user.display_name().to_lowercase().contains(&p)
                || b.user.id.to_string().starts_with(&p)
        })
        .take(25)
        .map(|b| {
            serenity::AutocompleteChoice::new(
                format!("{} ({})", b.user.display_name(), b.user.id),
                b.user.id.to_string(),
            )
        })
        .collect();
    serenity::CreateAutocompleteResponse::new().set_choices(choices)
}

/// Lift a ban.
#[poise::command(slash_command, guild_only, default_member_permissions = "BAN_MEMBERS")]
pub async fn unban(
    ctx: Context<'_>,
    #[description = "Banned user — pick from the list, or paste an ID/mention"]
    #[autocomplete = "autocomplete_banned_user"]
    user: String,
    #[description = "Reason"] reason: String,
) -> Result<(), Error> {
    let guild_id = ctx.guild_id().unwrap();

    let raw = user.trim().trim_start_matches("<@").trim_start_matches('!').trim_end_matches('>');
    let uid = raw.parse::<u64>().map(serenity::UserId::new).map_err(|_| {
        Error::user("That doesn't look like a user — pick one from the list or paste their ID.")
    })?;

    // Confirm they're actually banned before touching anything, and pick up a
    // human name for the confirmation. A transient fetch error doesn't block
    // the unban itself.
    let banned_name = match guild_id.get_ban(&ctx.serenity_context().http, uid).await {
        Ok(Some(ban)) => Some(ban.user.display_name().to_string()),
        Ok(None) => {
            return Err(Error::user(format!("<@{}> isn't banned from this server.", uid)));
        }
        Err(_) => None,
    };

    let case_id = crate::features::moderation::service::lift_ban(
        &ctx.serenity_context().http,
        &ctx.data(),
        guild_id,
        uid,
        ctx.author().id,
        &reason,
        None,
    )
    .await
    .map_err(|e| Error::user(format!("Failed to unban: {}", e)))?;

    let who = match banned_name {
        Some(name) => format!("**{}** (<@{}>)", name, uid),
        None => format!("<@{}>", uid),
    };
    ctx.send(poise::CreateReply::default().ephemeral(true).embed(confirm_embed(
        "unban",
        Some(case_id),
        format!("{}'s ban has been lifted. They can rejoin with an invite.\n**Reason:** {}", who, reason),
    )))
    .await?;
    Ok(())
}
