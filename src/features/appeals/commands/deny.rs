use crate::context::{Context, Error};

/// Deny an appeal.
#[poise::command(slash_command, guild_only, default_member_permissions = "MODERATE_MEMBERS")]
pub async fn deny(
    ctx: Context<'_>,
    #[description = "Response to send to the user"] response: String,
) -> Result<(), Error> {
    super::resolve_appeal(ctx, "denied", response).await
}
