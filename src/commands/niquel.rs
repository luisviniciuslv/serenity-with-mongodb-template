use std::time::Duration;

use poise::serenity_prelude::{Colour, CreateEmbed, EditMessage};
use poise::CreateReply;

use crate::db::{get_user, update_coins};
use crate::{Context, Error};

const SYMBOLS: [&str; 7] = ["💎", "💰", "🤑", "🐒", "😒", "🐈‍⬛", "👎"];

#[poise::command(slash_command, prefix_command, user_cooldown = 5)]
pub async fn niquel(
    ctx: Context<'_>,
    #[description = "Quantidade de linhas (1 a 10)"] linhas: i64,
    #[description = "Valor da aposta por linha"] aposta: i64,
) -> Result<(), Error> {
    let user = ctx.author();
    let user_db = get_user(&user.id.to_string()).await?;

    if !(1..=10).contains(&linhas) {
        ctx.say("Quantidade de linhas inválida. Use entre 1 e 10.").await?;
        return Ok(());
    }

    if aposta <= 0 {
        ctx.say("Valor de aposta inválido.").await?;
        return Ok(());
    }

    let total_aposta = linhas * aposta;

    if user_db.coins < total_aposta {
        ctx.say("Você não tem coins suficientes para apostar.").await?;
        return Ok(());
    }

    let saldo_apos_aposta = update_coins(&user.id.to_string(), -total_aposta).await?;

    let linhas_usize = linhas as usize;
    let mut slots = vec![vec!["?".to_string(), "?".to_string(), "?".to_string()]; linhas_usize];

    let user_image_url = user.face().to_string();

    let reply = ctx
        .send(CreateReply {
            embeds: vec![build_spinning_embed(&slots, linhas, aposta, total_aposta, 0, &user.name, &user_image_url)],
            ..Default::default()
        })
        .await?;

    let mut message = reply.message().await?.into_owned();
    let mut animation_edit_failed = false;

    // Revela por coluna para todas as linhas: 1a, 2a e 3a.
    for col in 0..3 {
        tokio::time::sleep(Duration::from_secs(2)).await;

        for row in slots.iter_mut() {
            let symbol = SYMBOLS[rand::random_range(0..SYMBOLS.len())];
            row[col] = symbol.to_string();
        }

        if !animation_edit_failed {
            if message
                .edit(
                    ctx.serenity_context(),
                    EditMessage::new().embed(build_spinning_embed(
                        &slots,
                        linhas,
                        aposta,
                        total_aposta,
                        col + 1,
                        &user.name,
                        &user_image_url,
                    )),
                )
                .await
                .is_err()
            {
                animation_edit_failed = true;
            }
        }
    }

    let line_multipliers: Vec<f64> = slots
        .iter()
        .map(|line| calculate_multiplier(line))
        .collect();

    let comedy_bonus = rare_quartet_bonus(&slots, total_aposta);

    let payout: i64 = line_multipliers
        .iter()
        .map(|multiplier| ((aposta as f64) * multiplier).floor() as i64)
        .sum::<i64>()
        + comedy_bonus.as_ref().map(|(_, bonus)| *bonus).unwrap_or(0);

    let saldo_final = if payout > 0 {
        update_coins(&user.id.to_string(), payout).await?.coins
    } else {
        saldo_apos_aposta.coins
    };

    let result_embed = build_result_embed(
        &slots,
        &line_multipliers,
        linhas,
        aposta,
        total_aposta,
        payout,
        saldo_final,
        comedy_bonus.as_ref().map(|(label, _)| label.as_str()),
        comedy_bonus.as_ref().map(|(_, bonus)| *bonus).unwrap_or(0),
    );

    if animation_edit_failed
        || message
            .edit(
                ctx.serenity_context(),
                EditMessage::new().embed(result_embed.clone()),
            )
            .await
            .is_err()
    {
        ctx.send(CreateReply {
            embeds: vec![result_embed],
            ..Default::default()
        })
        .await?;
    }

    Ok(())
}

fn build_spinning_embed(
    slots: &[Vec<String>],
    linhas: i64,
    aposta: i64,
    total_aposta: i64,
    revealed_count: usize,
    user_name: &str,
    user_image_url: &str
) -> CreateEmbed {
    CreateEmbed::new()
        .title(&format!("Caça-níquel de {}", user_name))
        .thumbnail(user_image_url)
        .color(Colour::DARK_GOLD)
        .description(render_slots_table(slots))
        .field("Linhas", linhas.to_string(), true)
        .field("Aposta por linha", aposta.to_string(), true)
        .field("Aposta total", total_aposta.to_string(), true)
        .field("Reels revelados", format!("{}/3", revealed_count), true)
        .field("Status", "Girando...", true)
        .field(
            "Tabela",
            "💎💎💎 = x8 | 💰💰💰 = x4 | 🤑🤑🤑 = x2\n2 símbolos bons = x0.75 | 1 símbolo bom = x0.20\nQuarteto idêntico em 4 linhas = bônus raro",
            false,
        )
}

fn build_result_embed(
    slots: &[Vec<String>],
    line_multipliers: &[f64],
    linhas: i64,
    aposta: i64,
    total_aposta: i64,
    payout: i64,
    saldo_final: i64,
    comedy_bonus_label: Option<&str>,
    comedy_bonus_value: i64,
) -> CreateEmbed {
    let won = payout > 0;
    let lucro = payout - total_aposta;
    let status_text = if won {
        if lucro >= 0 {
            format!("Você ganhou {} coin(s) e lucrou {} coin(s)!", payout, lucro)
        } else {
            format!("Você ganhou {} coin(s), mas ainda ficou -{} coin(s) nesta rodada.", payout, -lucro)
        }
    } else {
        format!("Você não ganhou nada desta vez. Prejuízo: {} coin(s).", total_aposta)
    };

    let mut detalhes_linhas = String::new();
    for (index, multiplier) in line_multipliers.iter().enumerate() {
        let line_payout = ((aposta as f64) * multiplier).floor() as i64;
        detalhes_linhas.push_str(&format!("L{}: {:.2}x ({} coin)\n", index + 1, multiplier, line_payout));
    }

    CreateEmbed::new()
        .title("Caça-níquel • Resultado")
        .color(if won { Colour::DARK_GREEN } else { Colour::DARK_RED })
        .description(render_slots_table(slots))
        .field("Linhas", linhas.to_string(), true)
        .field("Aposta por linha", aposta.to_string(), true)
        .field("Aposta total", total_aposta.to_string(), true)
        .field("Pagamento total", payout.to_string(), true)
        .field("Saldo atual", saldo_final.to_string(), true)
        .field("Resultado por linha", detalhes_linhas, false)
        .field(
            "Jogada cômica",
            comedy_bonus_label
                .map(|label| format!("{} (+{} coin)", label, comedy_bonus_value))
                .unwrap_or_else(|| "Nenhuma desta vez.".to_string()),
            false,
        )
        .field("Status", status_text, false)
}

fn calculate_multiplier(slots: &[String]) -> f64 {
    let diamonds = slots.iter().filter(|slot| slot.as_str() == "💎").count() as i64;
    let money = slots.iter().filter(|slot| slot.as_str() == "💰").count() as i64;
    let rich = slots.iter().filter(|slot| slot.as_str() == "🤑").count() as i64;
    let winning = diamonds + money + rich;

    if diamonds == 3 {
        return 8.0;
    }

    if money == 3 {
        return 4.0;
    }

    if rich == 3 {
        return 2.0;
    }

    if winning == 3 {
        return 0.75;
    }

    if winning == 2 {
        return 0.35;
    }

    if winning == 1 {
        return 0.20;
    }

    0.0
}

fn rare_quartet_bonus(slots: &[Vec<String>], total_aposta: i64) -> Option<(String, i64)> {
    if slots.len() != 4 {
        return None;
    }

    let first_line = slots.first()?;

    if !slots.iter().all(|line| line == first_line) {
        return None;
    }

    let has_winning_symbol = first_line.iter().any(|slot| matches!(slot.as_str(), "💎" | "💰" | "🤑"));
    if !has_winning_symbol {
        return None;
    }

    Some(("Quarteto dos Amigos".to_string(), (total_aposta as f64 * 2.0).floor() as i64))
}

fn render_slots_table(slots: &[Vec<String>]) -> String {
    let mut lines = Vec::with_capacity(slots.len());

    for line in slots {
        lines.push(format!("│  {}  {}  {}  │", line[0], line[1], line[2]));
    }

    format!(
        "```\n┌───────────────┐\n{}\n└───────────────┘\n```",
        lines.join("\n")
    )
}
