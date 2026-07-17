use poise::{serenity_prelude::{Colour, CreateEmbed, CreateEmbedFooter, User, Timestamp}, CreateReply};

use crate::{Context, Error};

use crate::db::{
  calculate_pending_reward, get_current_timestamp, get_max_reward_per_collection,
  get_reward_interval_seconds, get_reward_per_interval, get_user,
};

fn calculate_bet_stats(user_db: &crate::model::UserModel) -> (i64, i64, usize, usize, i64, i64) {
  let mut total_won = 0;
  let mut total_lost = 0;
  let mut wins = 0usize;
  let mut losses = 0usize;

  for bet in &user_db.bets {
    match bet.result.as_str() {
      "vitoria" => {
        total_won += bet.value;
        wins += 1;
      }
      "derrota" => {
        total_lost += bet.value;
        losses += 1;
      }
      _ => {}
    }
  }

  let net = total_won - total_lost;
  let total_bets = wins + losses;

  (total_won, total_lost, wins, losses, total_bets as i64, net)
}

//aliase !p
#[poise::command(slash_command, prefix_command, aliases("!p"))]
pub async fn profile(ctx: Context<'_>, 
#[description = "Caso queira ver o perfil de algum usuário, mencione-o."]
user: Option<User>) -> Result<(), Error> {
  let user = user.unwrap_or_else(|| ctx.author().clone());

  let user_db = get_user(&user.id.to_string()).await?;
  let (total_won, total_lost, wins, losses, total_bets, net_result) = calculate_bet_stats(&user_db);
  let rec_cap = get_max_reward_per_collection(&user_db);
  let (pending_reward, _, was_capped) = calculate_pending_reward(&user_db, get_current_timestamp());
  let reward_rate = get_reward_per_interval(&user_db);
  let elapsed_seconds = (get_current_timestamp() - user_db.last_reward).max(0);
  let seconds_to_next_reward = if elapsed_seconds % get_reward_interval_seconds() == 0 {
    get_reward_interval_seconds()
  } else {
    get_reward_interval_seconds() - (elapsed_seconds % get_reward_interval_seconds())
  };
  let footer = CreateEmbedFooter::new("ヾ(￣▽￣)").icon_url(user.face());
  let profit_color = if net_result >= 0 { Colour::DARK_GREEN } else { Colour::DARK_RED };

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
    .field("Renda por ciclo", format!("{} coin(s) / {}s", reward_rate, get_reward_interval_seconds()), true)
    .field("Acumulado no /rec", pending_reward.to_string(), true)
    .field("Limite por /rec", rec_cap.to_string(), true)
    .field("Cap ativo", if was_capped { "Sim" } else { "Nao" }, true)
    .field("Próximo tick", format!("{} segundo(s)", seconds_to_next_reward), true)
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
