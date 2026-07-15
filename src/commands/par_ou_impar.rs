use poise::CreateReply;

use crate::db::{get_user, update_coins};
use crate::{Context, Error};

#[poise::command(slash_command, prefix_command, rename = "poi", user_cooldown = 5)]
pub async fn par_ou_impar(
    ctx: Context<'_>,
    #[description = "Escolha par ou impar"] escolha: String,
    #[description = "Valor que deseja apostar"] aposta: Option<i64>,
) -> Result<(), Error> {
    run_bet_command(ctx, escolha, aposta).await
}

#[poise::command(prefix_command, user_cooldown = 5)]
pub async fn par(
    ctx: Context<'_>,
    #[description = "Valor que deseja apostar"] aposta: Option<i64>,
) -> Result<(), Error> {
    run_bet_command(ctx, "par".to_string(), aposta).await
}

#[poise::command(prefix_command, user_cooldown = 5)]
pub async fn impar(
    ctx: Context<'_>,
    #[description = "Valor que deseja apostar"] aposta: Option<i64>,
) -> Result<(), Error> {
    run_bet_command(ctx, "impar".to_string(), aposta).await
}

async fn run_bet_command(
    ctx: Context<'_>,
    escolha: String,
    aposta: Option<i64>,
) -> Result<(), Error> {
    let user = ctx.author();
    let user_db = get_user(&user.id.to_string()).await?;
    let escolha = escolha.to_lowercase();

    let message = if aposta.is_none() {
        "Use /poi ou !poi <par|impar> <aposta>. Exemplo: !poi par 100".to_string()
    } else if aposta.unwrap() <= 0 {
        "Valor de aposta inválido".to_string()
    } else if user_db.coins < aposta.unwrap() {
        "Você não tem coins suficientes para apostar".to_string()
    } else if escolha != "par" && escolha != "impar" {
        "Escolha inválida. Use \"par\" ou \"impar\".".to_string()
    } else {
        let aposta = aposta.unwrap();
        update_coins(&user.id.to_string(), -aposta).await?;
        let numero = generate_random_number(100);

        let ganhou = match escolha.as_str() {
            "par" => numero % 2 == 0,
            "impar" => numero % 2 != 0,
            _ => false,
        };

        if ganhou {
            let premio = aposta * 2;
            let updated_user = update_coins(&user.id.to_string(), premio).await?;
            format!(
                "Número gerado: {numero}\nVocê ganhou!\nPrêmio: {premio} coins\nSaldo atual: {} coins",
                updated_user.coins
            )
        } else {
            let updated_user = get_user(&user.id.to_string()).await?;
            format!("Número gerado: {numero}\nVocê perdeu {aposta} coins\nSaldo atual: {} coins", updated_user.coins)
        }
    };

    ctx.send(CreateReply {
        content: Some(message),
        ..Default::default()
    })
    .await?;

    Ok(())
}

fn generate_random_number(range: u32) -> u32 {
  return rand::random_range(0..range);
}
    