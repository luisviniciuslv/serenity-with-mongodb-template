use poise::serenity_prelude::{Colour, CreateEmbed};
use poise::CreateReply;

use crate::db::{get_user, get_all_users};
use crate::{Context, Error};

#[poise::command(slash_command, prefix_command, user_cooldown = 5)]
pub async fn rank(ctx: Context<'_>) -> Result<(), Error> {
    let user = ctx.author();
    let user_db = get_user(&user.id.to_string()).await?;

    let users = get_all_users().await?;

    let mut embed = CreateEmbed::new()
        .title("🏆 Ranking de Coins")
        .description(format!(
            "Você tem **{}** coin(s).",
            user_db.coins
        ))
        .colour(Colour::GOLD);

    for (i, u) in users.iter().enumerate() {
        let dc_user = ctx.serenity_context().http.get_user(u._id.parse::<u64>().unwrap().into()).await?;
        embed = embed.field(
            format!("{}. {}", i + 1, dc_user.name),
            format!("{} coin(s)", u.coins),
            false,
        );
    }
    
    ctx.send(CreateReply {
        embeds: vec![embed],
        ..Default::default()
    })
    .await?;

    Ok(())
}
