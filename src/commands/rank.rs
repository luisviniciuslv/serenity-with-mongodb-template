use poise::serenity_prelude::{Colour, CreateEmbed};
use poise::CreateReply;

use crate::db::{get_all_users, get_user};
use crate::{Context, Error};

fn bet_stats(user: &crate::model::UserModel) -> (i64, i64, usize, usize) {
    let total_won = user.total_won;
    let total_lost = user.total_lost;
    let wins = user.wins as usize;
    let losses = user.losses as usize;

    (total_won, total_lost, wins, losses)
}

async fn display_name_for_user(ctx: &Context<'_>, user_id: &str) -> String {
    if let Ok(parsed_id) = user_id.parse::<u64>() {
        return match ctx.serenity_context().http.get_user(parsed_id.into()).await {
            Ok(dc_user) => dc_user.name,
            Err(_) => user_id.to_string(),
        };
    }

    user_id.to_string()
}

#[poise::command(slash_command, prefix_command, user_cooldown = 5)]
pub async fn rank(
    ctx: Context<'_>,
    #[description = "Modo: empresas, vitoria ou derrota"] modo: Option<String>,
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
    let vitoria_mode = matches!(
        modo.as_deref()
            .map(str::trim)
            .map(str::to_lowercase)
            .as_deref(),
        Some("vitoria")
    );
    let derrota_mode = matches!(
        modo.as_deref()
            .map(str::trim)
            .map(str::to_lowercase)
            .as_deref(),
        Some("derrota")
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
    } else if vitoria_mode {
        users.sort_by(|a, b| {
            let (a_won, _, a_wins, a_losses) = bet_stats(a);
            let (b_won, _, b_wins, b_losses) = bet_stats(b);

            b_won
                .cmp(&a_won)
                .then_with(|| b_wins.cmp(&a_wins))
                .then_with(|| a_losses.cmp(&b_losses))
                .then_with(|| b.coins.cmp(&a.coins))
        });
    } else if derrota_mode {
        users.sort_by(|a, b| {
            let (_, a_lost, a_wins, a_losses) = bet_stats(a);
            let (_, b_lost, b_wins, b_losses) = bet_stats(b);

            b_lost
                .cmp(&a_lost)
                .then_with(|| b_losses.cmp(&a_losses))
                .then_with(|| a_wins.cmp(&b_wins))
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
        } else if vitoria_mode {
            CreateEmbed::new()
                .title("🏆 Ranking de Vitórias")
                .description(format!(
                    "Você tem **{}** vitória(s) e **{}** derrota(s).",
                    user_db.wins,
                    user_db.losses
                ))
                .colour(Colour::DARK_GREEN)
        } else if derrota_mode {
            CreateEmbed::new()
                .title("📉 Ranking de Derrotas")
                .description(format!(
                    "Você perdeu **{}** coin(s) no total.",
                    user_db.total_lost
                ))
                .colour(Colour::DARK_RED)
    } else {
        CreateEmbed::new()
            .title("🏆 Ranking de Coins")
            .description(format!("Você tem **{}** coin(s).", user_db.coins))
            .colour(Colour::GOLD)
    };

    for (i, u) in users.iter().enumerate() {
        let display_name = display_name_for_user(&ctx, &u._id).await;

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
        } else if vitoria_mode {
            let (total_won, total_lost, wins, losses) = bet_stats(u);
            embed = embed.field(
                format!("{}. {}", i + 1, display_name),
                format!(
                    "Ganhou: {} coin(s) | Perdeu: {} coin(s) | Vitórias: {} | Derrotas: {}",
                    total_won, total_lost, wins, losses
                ),
                false,
            );
        } else if derrota_mode {
            let (total_won, total_lost, wins, losses) = bet_stats(u);
            embed = embed.field(
                format!("{}. {}", i + 1, display_name),
                format!(
                    "Perdeu: {} coin(s) | Ganhou: {} coin(s) | Vitórias: {} | Derrotas: {}",
                    total_lost, total_won, wins, losses
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
