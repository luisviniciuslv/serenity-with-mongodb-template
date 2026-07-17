use poise::serenity_prelude::{Colour, CreateEmbed};
use poise::CreateReply;

use crate::db::{get_user, record_bet, update_coins};
use crate::{Context, Error};

#[poise::command(slash_command, prefix_command, rename = "poi", user_cooldown = 5)]
pub async fn par_ou_impar(
    ctx: Context<'_>,
    #[description = "Escolha par ou impar"] escolha: String,
    #[description = "Valor que deseja apostar"] aposta: Option<String>,
) -> Result<(), Error> {
    run_bet_command(ctx, escolha, aposta).await
}

#[poise::command(prefix_command, user_cooldown = 5)]
pub async fn par(
    ctx: Context<'_>,
    #[description = "Valor que deseja apostar"] aposta: Option<String>,
) -> Result<(), Error> {
    run_bet_command(ctx, "par".to_string(), aposta).await
}

#[poise::command(prefix_command, user_cooldown = 5)]
pub async fn impar(
    ctx: Context<'_>,
    #[description = "Valor que deseja apostar"] aposta: Option<String>,
) -> Result<(), Error> {
    run_bet_command(ctx, "impar".to_string(), aposta).await
}

async fn run_bet_command(
    ctx: Context<'_>,
    escolha: String,
    aposta: Option<String>,
) -> Result<(), Error> {
    let user = ctx.author();
    let user_db = get_user(&user.id.to_string()).await?;
    let escolha = escolha.to_lowercase();

    if aposta.is_none() {
        let embed = CreateEmbed::new()
            .title("❌ Erro de Sintaxe")
            .color(Colour::RED)
            .description("Use `/poi` ou `!poi <par|impar> <aposta>`.\nExemplo: `!poi par 100`");
        ctx.send(CreateReply {
            embeds: vec![embed],
            ..Default::default()
        })
        .await?;
        return Ok(());
    }

    let aposta_str = aposta.unwrap();
    let aposta_val = if aposta_str.to_lowercase() == "allwin" {
        user_db.coins
    } else {
        match aposta_str.parse::<i64>() {
            Ok(val) => val,
            Err(_) => {
                let embed = CreateEmbed::new()
                    .title("❌ Erro")
                    .color(Colour::RED)
                    .description("Valor de aposta inválido. Digite um número válido ou `allwin`.");
                ctx.send(CreateReply {
                    embeds: vec![embed],
                    ..Default::default()
                })
                .await?;
                return Ok(());
            }
        }
    };

    if aposta_val <= 0 {
        let embed = CreateEmbed::new()
            .title("❌ Erro")
            .color(Colour::RED)
            .description("Valor de aposta inválido.");
        ctx.send(CreateReply {
            embeds: vec![embed],
            ..Default::default()
        })
        .await?;
        return Ok(());
    }

    if user_db.coins < aposta_val {
        let embed = CreateEmbed::new()
            .title("❌ Saldo Insuficiente")
            .color(Colour::RED)
            .description("Você não tem coins suficientes para realizar esta aposta.");
        ctx.send(CreateReply {
            embeds: vec![embed],
            ..Default::default()
        })
        .await?;
        return Ok(());
    }

    if escolha != "par" && escolha != "impar" {
        let embed = CreateEmbed::new()
            .title("❌ Erro")
            .color(Colour::RED)
            .description("Escolha inválida. Use apenas `par` ou `impar`.");
        ctx.send(CreateReply {
            embeds: vec![embed],
            ..Default::default()
        })
        .await?;
        return Ok(());
    }

    let aposta = aposta_val;
    update_coins(&user.id.to_string(), -aposta).await?;
    let numero = generate_random_number(100);

    let ganhou = match escolha.as_str() {
        "par" => numero % 2 == 0,
        "impar" => numero % 2 != 0,
        _ => false,
    };

    let user_image_url = user.face().to_string();

    let embed = if ganhou {
        let premio = aposta * 2;
        let updated_user = update_coins(&user.id.to_string(), premio).await?;
        let _ = record_bet(&user.id.to_string(), "par_ou_impar", aposta, true).await?;
        CreateEmbed::new()
            .title("🎲 Par ou Ímpar")
            .thumbnail(&user_image_url)
            .color(Colour::DARK_GREEN)
            .description(format!(
                "**{}** escolheu **{}** e o número gerado foi **{}**.\n\n🎉 **VOCÊ GANHOU!**",
                user.name, escolha, numero
            ))
            .field("Prêmio", format!("{} coins", premio), true)
            .field("Saldo Atual", format!("{} coins", updated_user.coins), true)
    } else {
        let updated_user = get_user(&user.id.to_string()).await?;
        let _ = record_bet(&user.id.to_string(), "par_ou_impar", aposta, false).await?;
        CreateEmbed::new()
            .title("🎲 Par ou Ímpar")
            .thumbnail(&user_image_url)
            .color(Colour::DARK_RED)
            .description(format!(
                "**{}** escolheu **{}** mas o número gerado foi **{}**.\n\n💀 **VOCÊ PERDEU!**",
                user.name, escolha, numero
            ))
            .field("Prejuízo", format!("{} coins", aposta), true)
            .field("Saldo Atual", format!("{} coins", updated_user.coins), true)
    };

    ctx.send(CreateReply {
        embeds: vec![embed],
        ..Default::default()
    })
    .await?;

    Ok(())
}

fn generate_random_number(range: u32) -> u32 {
    rand::random_range(0..range)
}
