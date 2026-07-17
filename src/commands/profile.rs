use poise::{
    serenity_prelude::{Colour, CreateEmbed, CreateEmbedFooter, Timestamp, User},
    CreateReply,
};

use crate::{Context, Error};

use crate::db::{
    get_current_timestamp, get_reward_interval_seconds, get_reward_per_interval, get_user,
};

fn calculate_bet_stats(user_db: &crate::model::UserModel) -> (i64, i64, usize, usize, i64, i64) {
    let total_won = user_db.total_won;
    let total_lost = user_db.total_lost;
    let wins = user_db.wins as usize;
    let losses = user_db.losses as usize;

    let net = total_won - total_lost;
    let total_bets = (wins + losses) as i64;

    (total_won, total_lost, wins, losses, total_bets, net)
}

#[poise::command(slash_command, prefix_command, aliases("p"))]
pub async fn profile(
    ctx: Context<'_>,
    #[description = "Caso queira ver o perfil de algum usuário, mencione-o."] user: Option<User>,
) -> Result<(), Error> {
    let user = user.unwrap_or_else(|| ctx.author().clone());

    let user_db = get_user(&user.id.to_string()).await?;
    let (total_won, total_lost, wins, losses, total_bets, net_result) =
        calculate_bet_stats(&user_db);
    let reward_rate = get_reward_per_interval(&user_db);
    let elapsed_seconds: i64 = (get_current_timestamp() - user_db.last_reward).max(0);
    let _ = if elapsed_seconds % get_reward_interval_seconds() == 0 {
        get_reward_interval_seconds()
    } else {
        get_reward_interval_seconds() - (elapsed_seconds % get_reward_interval_seconds())
    };
    let footer = CreateEmbedFooter::new("ヾ(￣▽￣)").icon_url(user.face());
    let profit_color = if net_result >= 0 {
        Colour::DARK_GREEN
    } else {
        Colour::DARK_RED
    };

    let embed = CreateEmbed::new()
        .title(format!("Perfil de {}", user.name.clone()))
        .description("Resumo financeiro e de apostas do usuário.")
        .color(profit_color)
        .thumbnail(user.face())
        .field("Saldo atual", format!("{} coin(s)", user_db.coins), true)
        .field("Ganhos totais", format!("{} coin(s)", total_won), true)
        .field("Perdas totais", format!("{} coin(s)", total_lost), true)
        .field("Vitórias", wins.to_string(), true)
        .field("Derrotas", losses.to_string(), true)
        .field("Total de apostas", total_bets.to_string(), true)
        .field("Saldo histórico", format!("{} coin(s)", net_result), true)
        .field(
            "Renda por ciclo",
            format!(
                "{} coin(s) / {}s",
                reward_rate,
                get_reward_interval_seconds()
            ),
            true,
        )
        .footer(footer)
        .timestamp(Timestamp::now());

    ctx.send(CreateReply {
        embeds: vec![embed],
        ..Default::default()
    })
    .await
    .unwrap();

    Ok(())
}
