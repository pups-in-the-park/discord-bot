use crate::context::{colours, Context, Error};
use crate::ui::{self, Container};

/// Show per-moderator concern statistics (which mod is challenged most).
#[poise::command(slash_command, guild_only)]
pub async fn stats(ctx: Context<'_>) -> Result<(), Error> {
    let guild_id = ctx.guild_id().unwrap();
    let gid = guild_id.to_string();

    let rows = ctx.data().db.concern_stats(&gid).await?;
    let (pending, reviewed) = ctx.data().db.concern_status_counts(&gid).await?;

    let mut body = format!(
        "**🚩 Concern statistics**\nPending: **{pending}** · Reviewed: **{reviewed}**"
    );
    if rows.is_empty() {
        body.push_str("\n\nNo concerns have been raised yet.");
    } else {
        body.push_str("\n\n**By moderator — most challenged first:**");
        for s in &rows {
            let who = match &s.moderator_id {
                Some(m) => format!("<@{m}>"),
                None => "*(unattributed)*".to_string(),
            };
            body.push_str(&format!(
                "\n{} — **{}** ({} denied appeals, {} dismissed reports)",
                who, s.total, s.denied_appeals, s.dismissed_reports,
            ));
        }
    }

    let card = Container::new(vec![ui::text(body)]).accent(colours::ORANGE.0);
    ui::slash_respond(ctx, &[card.into()]).await?;
    Ok(())
}
