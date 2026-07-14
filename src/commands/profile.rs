use poise::{serenity_prelude::{CreateEmbed, CreateEmbedFooter, User, Timestamp}, CreateReply};

use crate::{Context, Error};

use crate::db::{
  calculate_pending_reward, get_current_timestamp, get_max_reward_per_collection,
  get_reward_interval_seconds, get_reward_per_interval, get_user,
};
//aliase !p
#[poise::command(slash_command, prefix_command, aliases("!p"))]
pub async fn profile(ctx: Context<'_>, 
#[description = "Caso queira ver o perfil de algum usuário, mencione-o."]
user: Option<User>) -> Result<(), Error> {
  let user = user.unwrap_or_else(|| ctx.author().clone());

  let user_db = get_user(&user.id.to_string()).await?;
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

  let embed = CreateEmbed::new()
    .title(user.name.clone())
    .description(format!("Perfil do {}", user.name.clone()))
    .field("Coins", user_db.coins.to_string(), true)
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
