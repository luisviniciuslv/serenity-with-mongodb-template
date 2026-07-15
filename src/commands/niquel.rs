use std::time::Duration;

use poise::serenity_prelude::{Colour, CreateEmbed, EditMessage};
use poise::CreateReply;

use crate::db::{get_user, update_coins};
use crate::{Context, Error};

const SYMBOLS: [&str; 8] = ["🍒", "🍋", "🍊", "🍉", "🐒", "💰", "💎", "🃏"];

// 20 Linhas de Pagamento sempre ativas — padrão real de video slots modernos 5x3.
// Cada entrada representa a linha (0=topo, 1=meio, 2=base) em cada uma das 5 colunas.
const PAYLINES: [[usize; 5]; 20] = [
    [1, 1, 1, 1, 1], // L1:  Meio
    [0, 0, 0, 0, 0], // L2:  Topo
    [2, 2, 2, 2, 2], // L3:  Base
    [0, 1, 2, 1, 0], // L4:  V
    [2, 1, 0, 1, 2], // L5:  V invertido
    [0, 0, 1, 2, 2], // L6:  Escada descendo
    [2, 2, 1, 0, 0], // L7:  Escada subindo
    [1, 2, 2, 2, 1], // L8:  Arco base
    [1, 0, 0, 0, 1], // L9:  Arco topo
    [0, 1, 1, 1, 0], // L10: U invertido
    [2, 1, 1, 1, 2], // L11: U normal
    [0, 0, 0, 1, 2], // L12: Diagonal descendo direita
    [2, 2, 2, 1, 0], // L13: Diagonal subindo direita
    [0, 1, 0, 1, 0], // L14: Zig-zag topo longo
    [2, 1, 2, 1, 2], // L15: Zig-zag base longo
    [1, 2, 1, 0, 1], // L16: Zig-zag base curto
    [1, 0, 1, 2, 1], // L17: Zig-zag topo curto
    [0, 1, 2, 2, 2], // L18: L direita base
    [2, 1, 0, 0, 0], // L19: L direita topo
    [1, 1, 0, 1, 1], // L20: Dente de serra
];

const NUM_PAYLINES: i64 = PAYLINES.len() as i64;

struct LineResult {
    multiplier: f64,
    payout: i64,
    symbol: String,
    count: usize,
}

#[poise::command(slash_command, prefix_command, user_cooldown = 5)]
pub async fn niquel(
    ctx: Context<'_>,
    #[description = "Valor da aposta por linha (x10 linhas no total)"] aposta: String,
) -> Result<(), Error> {
    let user = ctx.author();
    let user_db = get_user(&user.id.to_string()).await?;

    let aposta = if aposta.to_lowercase() == "allwin" {
        user_db.coins / NUM_PAYLINES
    } else {
        match aposta.parse::<i64>() {
            Ok(val) => val,
            Err(_) => {
                ctx.say("Aposta inválida. Digite um número ou 'allwin'.")
                    .await?;
                return Ok(());
            }
        }
    };

    if aposta <= 0 {
        ctx.say("Valor de aposta inválido.").await?;
        return Ok(());
    }

    // Caça-níquel real: sempre 10 linhas ativas
    let total_aposta = NUM_PAYLINES * aposta;

    if user_db.coins < total_aposta {
        ctx.say(format!(
            "Você não tem coins suficientes. Aposta total: {} coins ({} linhas x {}).",
            total_aposta, NUM_PAYLINES, aposta
        ))
        .await?;
        return Ok(());
    }

    let saldo_apos_aposta = update_coins(&user.id.to_string(), -total_aposta).await?;

    // Grade fixa de 3 linhas e 5 colunas
    let mut grid = vec![vec!["?".to_string(); 5]; 3];

    let user_image_url = user.face().to_string();

    let reply = ctx
        .send(CreateReply {
            embeds: vec![build_spinning_embed(
                &grid,
                aposta,
                total_aposta,
                0,
                &user.name,
                &user_image_url,
            )],
            ..Default::default()
        })
        .await?;

    let mut message = reply.message().await?.into_owned();
    let mut animation_edit_failed = false;

    // Revela coluna por coluna (5 colunas)
    for col in 0..5 {
        tokio::time::sleep(Duration::from_millis(1200)).await;

        for row in 0..3 {
            let symbol = SYMBOLS[rand::random_range(0..SYMBOLS.len())];
            grid[row][col] = symbol.to_string();
        }

        if !animation_edit_failed {
            if message
                .edit(
                    ctx.serenity_context(),
                    EditMessage::new().embed(build_spinning_embed(
                        &grid,
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

    // Calcula os resultados das 10 linhas
    let mut line_results: Vec<(usize, LineResult)> = Vec::new();

    for i in 0..PAYLINES.len() {
        if let Some(mut result) = calculate_line(&grid, i) {
            result.payout = ((aposta as f64) * result.multiplier).floor() as i64;
            line_results.push((i, result));
        }
    }

    let payout: i64 = line_results.iter().map(|(_, r)| r.payout).sum();

    let saldo_final = if payout > 0 {
        update_coins(&user.id.to_string(), payout).await?.coins
    } else {
        saldo_apos_aposta.coins
    };

    let result_embed = build_result_embed(
        &grid,
        &line_results,
        total_aposta,
        payout,
        saldo_final,
        &user.name,
        &user_image_url,
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
    grid: &[Vec<String>],
    aposta: i64,
    total_aposta: i64,
    revealed_count: usize,
    user_name: &str,
    user_image_url: &str,
) -> CreateEmbed {
    CreateEmbed::new()
        .title(&format!("🎰 Caça-níquel de {}", user_name))
        .thumbnail(user_image_url)
        .color(Colour::DARK_GOLD)
        .description(render_grid(grid))
        .field("Linhas Ativas", format!("{}", NUM_PAYLINES), true)
        .field("Aposta por linha", format!("{} coins", aposta), true)
        .field("Aposta total", format!("{} coins", total_aposta), true)
        .field("Colunas reveladas", format!("{}/5", revealed_count), true)
        .field("Status", "Girando...", true)
        .field(
            "Tabela de Pagamento (Esq → Dir, mín 3 iguais)",
            "🃏 Wild (substitui todos)\n💎 200/50/10x | 💰 100/25/5x | 🐒 50/15/4x | 🍉 25/10/3x | 🍊 20/8/2x | 🍋 15/6/1.5x | 🍒 10/4/1x",
            false,
        )
}

fn build_result_embed(
    grid: &[Vec<String>],
    line_results: &[(usize, LineResult)],
    total_aposta: i64,
    payout: i64,
    saldo_final: i64,
    user_name: &str,
    user_image_url: &str,
) -> CreateEmbed {
    let won = payout > 0;
    let lucro = payout - total_aposta;
    let status_text = if won {
        if lucro >= 0 {
            format!("Ganhou **{}** coin(s) e lucrou **{}** coin(s)! 🎉", payout, lucro)
        } else {
            format!(
                "Ganhou **{}** coin(s), mas ficou **{}** coin(s) no prejuízo na rodada.",
                payout, -lucro
            )
        }
    } else {
        format!(
            "Não ganhou nada desta vez. Prejuízo: **{}** coin(s).",
            total_aposta
        )
    };

    let mut detalhes_linhas = String::new();
    if line_results.is_empty() {
        detalhes_linhas.push_str("Nenhuma linha premiada.");
    } else {
        for (idx, result) in line_results {
            detalhes_linhas.push_str(&format!(
                "**L{}**: {}x{} ({:.2}x) = **{}** coins\n",
                idx + 1,
                result.count,
                result.symbol,
                result.multiplier,
                result.payout
            ));
        }
    }

    CreateEmbed::new()
        .title(&format!("🎰 Caça-níquel • Resultado ({})", user_name))
        .thumbnail(user_image_url)
        .color(if won {
            Colour::DARK_GREEN
        } else {
            Colour::DARK_RED
        })
        .description(render_grid(grid))
        .field("Aposta Total", format!("{} coins", total_aposta), true)
        .field("Pagamento Total", format!("{} coins", payout), true)
        .field("Saldo Atual", format!("{} coins", saldo_final), true)
        .field("Status", status_text, false)
        .field("Detalhes das Linhas Ganhas", detalhes_linhas, false)
}

fn calculate_line(grid: &[Vec<String>], payline_idx: usize) -> Option<LineResult> {
    let line_def = &PAYLINES[payline_idx];

    // Extrai os 5 símbolos desta linha da esquerda para a direita
    let symbols: Vec<String> = (0..5).map(|col| grid[line_def[col]][col].clone()).collect();

    // Encontra o "símbolo base" (o primeiro não-Wild)
    let mut base_symbol = &symbols[0];
    let mut count = 0;

    for (i, sym) in symbols.iter().enumerate() {
        if base_symbol == "🃏" && sym != "🃏" {
            base_symbol = sym;
        }
        if sym == base_symbol || sym == "🃏" {
            count = i + 1;
        } else {
            break;
        }
    }

    let multiplier = get_symbol_multiplier(base_symbol, count);

    if multiplier > 0.0 {
        Some(LineResult {
            multiplier,
            payout: 0,
            symbol: base_symbol.to_string(),
            count,
        })
    } else {
        None
    }
}

fn get_symbol_multiplier(symbol: &str, count: usize) -> f64 {
    match symbol {
        "💎" => match count { 5 => 200.0, 4 => 50.0, 3 => 10.0, _ => 0.0 },
        "💰" => match count { 5 => 100.0, 4 => 25.0, 3 => 5.0, _ => 0.0 },
        "🐒" => match count { 5 => 50.0, 4 => 15.0, 3 => 4.0, _ => 0.0 },
        "🍉" => match count { 5 => 25.0, 4 => 10.0, 3 => 3.0, _ => 0.0 },
        "🍊" => match count { 5 => 20.0, 4 => 8.0, 3 => 2.0, _ => 0.0 },
        "🍋" => match count { 5 => 15.0, 4 => 6.0, 3 => 1.5, _ => 0.0 },
        "🍒" => match count { 5 => 10.0, 4 => 4.0, 3 => 1.0, _ => 0.0 },
        "🃏" => match count { 5 => 200.0, 4 => 50.0, 3 => 10.0, _ => 0.0 },
        _ => 0.0
    }
}

fn render_grid(grid: &[Vec<String>]) -> String {
    let mut lines = Vec::with_capacity(3);
    for row in grid {
        lines.push(format!("│  {}  {}  {}  {}  {}  │", row[0], row[1], row[2], row[3], row[4]));
    }
    format!(
        "```\n┌─────────────────────────────┐\n{}\n└─────────────────────────────┘\n```",
        lines.join("\n")
    )
}
