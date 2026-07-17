use poise::serenity_prelude::{Colour, CreateEmbed};
use poise::CreateReply;

use crate::db::{get_all_users, get_user};
use crate::{Context, Error};

#[poise::command(slash_command, prefix_command, user_cooldown = 5)]
pub async fn rank(
    ctx: Context<'_>,
    #[description = "Modo: empresas"] modo: Option<String>,
) -> Result<(), Error> {
    let user = ctx.author();
    let user_db = get_user(&user.id.to_string()).await?;

    let mut users = get_all_users().await?;
    let empresas_mode = matches!(
        modo.as_deref()
            .map(str::trim)
            .map(str::to_lowercase)
            .as_deref(),
        Some("empresas")
    );

    if empresas_mode {
        // Nivel total sempre tem prioridade sobre quantidade de empresas.
        users.sort_by(|a, b| {
            let a_total_level: i64 = a.businesses.iter().map(|business| business.level).sum();
            let b_total_level: i64 = b.businesses.iter().map(|business| business.level).sum();

            b_total_level
                .cmp(&a_total_level)
                .then_with(|| b.businesses.len().cmp(&a.businesses.len()))
                .then_with(|| b.coins.cmp(&a.coins))
        });
    } else {
        users.sort_by(|a, b| b.coins.cmp(&a.coins));
    }

    let mut embed = if empresas_mode {
        CreateEmbed::new()
            .title("🏢 Ranking de Empresas")
            .description(format!(
                "Você tem **{}** empresa(s) com **{}** nível(is) total.",
                user_db.businesses.len(),
                user_db
                    .businesses
                    .iter()
                    .map(|business| business.level)
                    .sum::<i64>()
            ))
            .colour(Colour::DARK_GREEN)
    } else {
        CreateEmbed::new()
            .title("🏆 Ranking de Coins")
            .description(format!("Você tem **{}** coin(s).", user_db.coins))
            .colour(Colour::GOLD)
    };

    for (i, u) in users.iter().enumerate() {
        let user_id = u._id.parse::<u64>();
        let display_name = if let Ok(parsed_id) = user_id {
            match ctx.serenity_context().http.get_user(parsed_id.into()).await {
                Ok(dc_user) => dc_user.name,
                Err(_) => u._id.clone(),
            }
        } else {
            u._id.clone()
        };

        if empresas_mode {
            let total_level: i64 = u.businesses.iter().map(|business| business.level).sum();
            embed = embed.field(
                format!("{}. {}", i + 1, display_name),
                format!(
                    "{} empresa(s) | {} nível(is) total",
                    u.businesses.len(),
                    total_level
                ),
                false,
            );
        } else {
            embed = embed.field(
                format!("{}. {}", i + 1, display_name),
                format!("{} coin(s)", u.coins),
                false,
            );
        }
    }

    ctx.send(CreateReply {
        embeds: vec![embed],
        ..Default::default()
    })
    .await?;

    Ok(())
}
