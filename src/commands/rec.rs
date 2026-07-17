use poise::CreateReply;

use crate::db::{
    collect_reward, get_current_timestamp, get_max_reward_per_collection,
    get_reward_interval_seconds, get_user,
};
use crate::{Context, Error};

#[poise::command(slash_command, prefix_command, user_cooldown = 5)]
pub async fn rec(ctx: Context<'_>) -> Result<(), Error> {
    let user = ctx.author();
    let user_db = get_user(&user.id.to_string()).await?;
    let rec_cap = get_max_reward_per_collection(&user_db);
    let now = get_current_timestamp();
    let elapsed_seconds = (now - user_db.last_reward).max(0);
    let missing_seconds =
        get_reward_interval_seconds() - (elapsed_seconds % get_reward_interval_seconds());
    let (updated_user, reward_amount, was_capped) = collect_reward(&user.id.to_string()).await?;

    let message = if reward_amount > 0 {
        if was_capped {
            format!(
                "Você recebeu {reward_amount} coin(s)! Limite por coleta: {} coin(s).",
                rec_cap
            )
        } else {
            format!(
                "Você recebeu {reward_amount} coin(s)! Agora você tem {} coin(s).",
                updated_user.coins
            )
        }
    } else {
        format!("Ainda não acumulou reward. Espere mais {missing_seconds} segundo(s).")
    };

    ctx.send(CreateReply {
        content: Some(message),
        ..Default::default()
    })
    .await?;
    Ok(())
}
