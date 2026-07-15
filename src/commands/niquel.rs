use std::time::Duration;

use poise::serenity_prelude::{Colour, CreateEmbed, EditMessage};
use poise::CreateReply;

use crate::db::{get_user, update_coins};
use crate::{Context, Error};

const SYMBOLS: [&str; 7] = ["💎", "💰", "🤑", "🐒", "😒", "🐈‍⬛", "👎"];

#[poise::command(slash_command, prefix_command, user_cooldown = 5)]
pub async fn niquel(
    ctx: Context<'_>,
    #[description = "Valor da aposta"] aposta: i64,
) -> Result<(), Error> {
    let user = ctx.author();
    let user_db = get_user(&user.id.to_string()).await?;

    if aposta <= 0 {
        ctx.say("Valor de aposta inválido.").await?;
        return Ok(());
    }

    if user_db.coins < aposta {
        ctx.say("Você não tem coins suficientes para apostar.").await?;
        return Ok(());
    }

    let saldo_apos_aposta = update_coins(&user.id.to_string(), -aposta).await?;

    let mut slots = vec!["?".to_string(), "?".to_string(), "?".to_string()];

    let reply = ctx
        .send(CreateReply {
            embeds: vec![build_spinning_embed(&slots, aposta, 0)],
            ..Default::default()
        })
        .await?;

    let mut message = reply.message().await?.into_owned();

    for i in 0..3 {
        tokio::time::sleep(Duration::from_secs(2)).await;

        let symbol = SYMBOLS[rand::random_range(0..SYMBOLS.len())];
        slots[i] = symbol.to_string();

        message
            .edit(
                ctx.serenity_context(),
                EditMessage::new().embed(build_spinning_embed(&slots, aposta, i + 1)),
            )
            .await?;
    }

    let multiplier = calculate_multiplier(&slots);
    let payout = (aposta as f64 * multiplier).floor() as i64;

    let saldo_final = if payout > 0 {
        update_coins(&user.id.to_string(), payout).await?.coins
    } else {
        saldo_apos_aposta.coins
    };

    message
        .edit(
            ctx.serenity_context(),
            EditMessage::new().embed(build_result_embed(&slots, aposta, multiplier, payout, saldo_final)),
        )
        .await?;

    Ok(())
}

fn build_spinning_embed(slots: &[String], aposta: i64, revealed_count: usize) -> CreateEmbed {
    CreateEmbed::new()
        .title("Caça-níquel")
        .color(Colour::DARK_GOLD)
        .description(format!(
            "```\n┌───────────────┐\n│  {}  {}  {}  │\n└───────────────┘\n```",
            slots[0], slots[1], slots[2]
        ))
        .field("Aposta", aposta.to_string(), true)
        .field("Reels revelados", format!("{}/3", revealed_count), true)
        .field("Status", "Girando...", true)
        .field(
            "Tabela",
            "💎💎💎 = x10 | 💰💰💰 = x5 | 🤑🤑🤑 = x2.5\nMix (só desses 3) = média das odds",
            false,
        )
}

fn build_result_embed(
    slots: &[String],
    aposta: i64,
    multiplier: f64,
    payout: i64,
    saldo_final: i64,
) -> CreateEmbed {
    let won = payout > 0;
    let status_text = if won {
        format!("Você ganhou {} coin(s)!", payout)
    } else {
        "Você não ganhou nada desta vez.".to_string()
    };

    CreateEmbed::new()
        .title("Caça-níquel • Resultado")
        .color(if won { Colour::DARK_GREEN } else { Colour::DARK_RED })
        .description(format!(
            "```\n┌───────────────┐\n│  {}  {}  {}  │\n└───────────────┘\n```",
            slots[0], slots[1], slots[2]
        ))
        .field("Aposta", aposta.to_string(), true)
        .field("Multiplicador", format!("{:.2}x", multiplier), true)
        .field("Pagamento", payout.to_string(), true)
        .field("Saldo atual", saldo_final.to_string(), true)
        .field("Status", status_text, false)
}

fn calculate_multiplier(slots: &[String]) -> f64 {
    let diamonds = slots.iter().filter(|slot| slot.as_str() == "💎").count() as i64;
    let money = slots.iter().filter(|slot| slot.as_str() == "💰").count() as i64;
    let rich = slots.iter().filter(|slot| slot.as_str() == "🤑").count() as i64;

    if diamonds == 3 {
        return 10.0;
    }

    if money == 3 {
        return 5.0;
    }

    if rich == 3 {
        return 2.5;
    }

    // Mixed odds for combinations containing only the 3 winning symbols.
    if diamonds + money + rich == 3 {
        return (diamonds as f64 * 10.0 + money as f64 * 5.0 + rich as f64 * 2.5) / 3.0;
    }

    0.0
}
