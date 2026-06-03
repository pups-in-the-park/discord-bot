use poise::serenity_prelude as serenity;

use super::autocomplete_scope;
use crate::context::{colours, Context, Error};
use crate::features::blocklist::view::{scope_feature_label, sqlite_ts, FEATURE_SCOPES};
use crate::features::moderation::service::{send_action_dm, ModActionDm};

/// Block a user from a feature (tickets, reports, concerns, appeals) for a time.
#[poise::command(slash_command, guild_only, default_member_permissions = "MANAGE_GUILD")]
pub async fn add(
    ctx: Context<'_>,
    #[description = "User to blocklist"] user: serenity::User,
    #[description = "tickets / reports / concerns / appeals / ticket:<category>"]
    #[autocomplete = "autocomplete_scope"]
    scope: String,
    #[description = "Reason (shown to the user)"] reason: String,
    #[description = "Duration, e.g. 7d, 30d (required unless permanent)"] duration: Option<String>,
    #[description = "Block permanently (no expiry)"] permanent: Option<bool>,
) -> Result<(), Error> {
    let guild_id = ctx.guild_id().unwrap();
    let gid = guild_id.to_string();
    let author = ctx.author().id.to_string();

    if !FEATURE_SCOPES.contains(&scope.as_str()) && !scope.starts_with("ticket:") {
        return Err(Error::user(
            "Scope must be one of: `tickets`, `reports`, `concerns`, `appeals`, or `ticket:<category>`.",
        ));
    }

    // Expiry is required by default; `permanent: true` is the explicit opt-out.
    let expires_at: Option<String> = if permanent.unwrap_or(false) {
        None
    } else {
        let Some(secs) = duration.as_deref().and_then(crate::util::parse_duration_secs) else {
            return Err(Error::user(
                "Provide a duration (e.g. `7d`, `30d`) or set `permanent: true`.",
            ));
        };
        Some(
            (chrono::Utc::now() + chrono::Duration::seconds(secs))
                .format("%Y-%m-%d %H:%M:%S")
                .to_string(),
        )
    };

    ctx.data()
        .db
        .blocklist_add(&gid, &user.id.to_string(), &scope, &reason, &author, expires_at.as_deref())
        .await?;

    // Record as an appealable infraction so the user can contest the block even
    // when blocked from appeals (see the appeal escape hatch).
    let infraction = ctx
        .data()
        .db
        .create_infraction(
            &gid,
            &user.id.to_string(),
            &author,
            "blocklist",
            &reason,
            None,
            true,
            expires_at.as_deref(),
        )
        .await?;

    let expires_ts = expires_at.as_deref().and_then(sqlite_ts);

    // Inform the user with reason + expiry + an appeal button.
    send_action_dm(
        &ctx.serenity_context().http,
        &user,
        guild_id,
        ModActionDm::Blocklist {
            reason: &reason,
            feature: &scope_feature_label(&scope),
            expires_ts,
        },
        Some((infraction.id, guild_id)),
    )
    .await;

    let expiry_label = match expires_ts {
        Some(ts) => format!("lifts <t:{ts}:R>"),
        None => "permanent".to_string(),
    };
    ctx.send(
        poise::CreateReply::default().ephemeral(true).embed(
            serenity::CreateEmbed::new()
                .colour(colours::GREEN)
                .title("Blocklist Updated")
                .description(format!(
                    "<@{}> blocked from `{}` ({}).\n**Reason:** {}",
                    user.id, scope, expiry_label, reason
                )),
        ),
    )
    .await?;
    Ok(())
}
