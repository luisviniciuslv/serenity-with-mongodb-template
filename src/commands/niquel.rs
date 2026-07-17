use std::time::Duration;

use poise::serenity_prelude::{
    ButtonStyle, Colour, CreateActionRow, CreateButton, CreateEmbed,
    CreateInteractionResponse, CreateInteractionResponseMessage, EditMessage,
};
use poise::CreateReply;

use crate::db::{get_user, update_coins};
use crate::{Context, Error};

// ─── Símbolos ────────────────────────────────────────────────────────────────
const SCATTER: &str = "⭐";
const WILD: &str = "🃏";

// (símbolo, peso) — maior peso = aparece mais frequentemente
const WEIGHTED_SYMBOLS: &[(&str, u32)] = &[
    ("🍒", 22), // Cherry       — muito comum
    ("🔔", 19), // Bell         — comum
    ("🍋", 17), // Lemon        — comum
    ("🍊", 14), // Orange       — moderado
    ("🍉", 12), // Watermelon   — moderado
    ("🐒",  9), // Monkey       — incomum
    ("🍀",  6), // Clover       — incomum
    ("💰",  5), // Moneybag     — raro
    ("👑",  2), // Crown        — muito raro
    ("💎",  1), // Diamond      — LENDÁRIO
    (WILD,   3), // Wild         — especial
    (SCATTER, 3), // Scatter     — especial
];

// ─── Paylines ─────────────────────────────────────────────────────────────────
// 20 Linhas de Pagamento sempre ativas — padrão real de video slots modernos 5x3.
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
const BTN_FREE_SPIN: &str = "niquel_free_spin";

// ─── Estruturas ───────────────────────────────────────────────────────────────
struct LineResult {
    multiplier: f64,
    payout: i64,
    symbol: String,
    count: usize,
}

struct SpinResult {
    grid: Vec<Vec<String>>,
    line_results: Vec<(usize, LineResult)>,
    payout: i64,
    scatter_count: usize,
    scatter_payout: i64,
    free_spins: u32,
}

// ─── Seleção de símbolo com pesos ─────────────────────────────────────────────
fn pick_symbol() -> &'static str {
    let total: u32 = WEIGHTED_SYMBOLS.iter().map(|(_, w)| w).sum();
    let roll = rand::random_range(0..total);
    let mut cumulative = 0u32;
    for (sym, weight) in WEIGHTED_SYMBOLS {
        cumulative += weight;
        if roll < cumulative {
            return sym;
        }
    }
    "🍒"
}

// ─── Lógica principal do giro ──────────────────────────────────────────────────
fn spin_grid() -> Vec<Vec<String>> {
    let mut grid = vec![vec![String::new(); 5]; 3];
    for row in 0..3 {
        for col in 0..5 {
            grid[row][col] = pick_symbol().to_string();
        }
    }
    grid
}

fn evaluate_spin(grid: &[Vec<String>], aposta: i64) -> SpinResult {
    // Conta Scatters em toda a grade (independente de paylines)
    let scatter_count = grid
        .iter()
        .flat_map(|row| row.iter())
        .filter(|s| s.as_str() == SCATTER)
        .count();

    let scatter_payout = scatter_bonus(scatter_count, aposta);
    let free_spins = free_spins_from_scatter(scatter_count);

    let mut line_results: Vec<(usize, LineResult)> = Vec::new();
    for i in 0..PAYLINES.len() {
        if let Some(mut result) = calculate_line(grid, i) {
            result.payout = ((aposta as f64) * result.multiplier).floor() as i64;
            line_results.push((i, result));
        }
    }

    let payout: i64 = line_results.iter().map(|(_, r)| r.payout).sum::<i64>() + scatter_payout;

    SpinResult {
        grid: grid.to_vec(),
        line_results,
        payout,
        scatter_count,
        scatter_payout,
        free_spins,
    }
}

// ─── Cálculo de linha (Wild corrigido) ────────────────────────────────────────
fn calculate_line(grid: &[Vec<String>], payline_idx: usize) -> Option<LineResult> {
    let line_def = &PAYLINES[payline_idx];
    let symbols: Vec<&str> = (0..5)
        .map(|col| grid[line_def[col]][col].as_str())
        .collect();

    // Símbolo base = primeiro não-Wild e não-Scatter da esquerda.
    // Se a linha for toda Wild → paga como Wild puro.
    let base_symbol = symbols
        .iter()
        .find(|&&s| s != WILD && s != SCATTER)
        .copied()
        .unwrap_or(WILD);

    // Conta consecutivos da esquerda; Wild substitui qualquer símbolo regular.
    // Scatter interrompe a sequência.
    let mut count = 0;
    for &sym in &symbols {
        if sym == WILD || sym == base_symbol {
            count += 1;
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

// ─── Tabela de multiplicadores ────────────────────────────────────────────────
fn get_symbol_multiplier(symbol: &str, count: usize) -> f64 {
    match symbol {
        "💎" => match count { 5 => 200.0, 4 => 50.0, 3 => 10.0, _ => 0.0 },
        "👑" => match count { 5 => 150.0, 4 => 35.0, 3 => 7.0,  _ => 0.0 },
        "💰" => match count { 5 => 100.0, 4 => 25.0, 3 => 5.0,  _ => 0.0 },
        "🍀" => match count { 5 => 60.0,  4 => 18.0, 3 => 4.5,  _ => 0.0 },
        "🐒" => match count { 5 => 50.0,  4 => 15.0, 3 => 4.0,  _ => 0.0 },
        "🍉" => match count { 5 => 25.0,  4 => 10.0, 3 => 3.0,  _ => 0.0 },
        "🍊" => match count { 5 => 20.0,  4 => 8.0,  3 => 2.0,  _ => 0.0 },
        "🍋" => match count { 5 => 15.0,  4 => 6.0,  3 => 1.5,  _ => 0.0 },
        "🔔" => match count { 5 => 12.0,  4 => 5.0,  3 => 1.2,  _ => 0.0 },
        "🍒" => match count { 5 => 10.0,  4 => 4.0,  3 => 1.0,  _ => 0.0 },
        WILD  => match count { 5 => 200.0, 4 => 50.0, 3 => 10.0, _ => 0.0 },
        _ => 0.0,
    }
}

// ─── Scatter ──────────────────────────────────────────────────────────────────
fn scatter_bonus(count: usize, aposta: i64) -> i64 {
    let total = aposta * NUM_PAYLINES;
    match count {
        5 => total * 50,
        4 => total * 20,
        3 => total * 5,
        _ => 0,
    }
}

fn free_spins_from_scatter(count: usize) -> u32 {
    match count {
        5 => 15,
        4 => 10,
        3 => 5,
        _ => 0,
    }
}

// ─── Comando principal ────────────────────────────────────────────────────────
#[poise::command(slash_command, prefix_command, user_cooldown = 5)]
pub async fn niquel(
    ctx: Context<'_>,
    #[description = "Valor TOTAL da aposta (dividido em 20 linhas automaticamente)"] aposta: String,
) -> Result<(), Error> {
    let user = ctx.author().clone();
    let user_db = get_user(&user.id.to_string()).await?;

    // aposta_total = o que o usuário digita; nunca perde mais do que isso
    let aposta_total = if aposta.to_lowercase() == "allwin" {
        user_db.coins
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

    if aposta_total <= 0 {
        ctx.say("Valor de aposta inválido.").await?;
        return Ok(());
    }

    // Divide internamente em 20 linhas; trunca para múltiplo exato
    let aposta_por_linha = (aposta_total / NUM_PAYLINES).max(1);
    let total_aposta = aposta_por_linha * NUM_PAYLINES;

    if user_db.coins < total_aposta {
        ctx.say(format!(
            "Você não tem coins suficientes. Aposta total: **{}** coins.",
            total_aposta
        ))
        .await?;
        return Ok(());
    }

    let user_image_url = user.face().to_string();

    // Deduz aposta e gira
    update_coins(&user.id.to_string(), -total_aposta).await?;
    let grid = spin_grid();
    let result = evaluate_spin(&grid, aposta_por_linha);
    let saldo_final = if result.payout > 0 {
        update_coins(&user.id.to_string(), result.payout).await?.coins
    } else {
        get_user(&user.id.to_string()).await?.coins
    };

    let aposta = aposta_por_linha; // alias para o restante do código

    // Animação de giro (mostra "?" e revela coluna por coluna)
    let reply = ctx
        .send(CreateReply {
            embeds: vec![build_spinning_embed(
                &vec![vec!["?".to_string(); 5]; 3],
                total_aposta,
                0,
                &user.name,
                &user_image_url,
            )],
            ..Default::default()
        })
        .await?;

    let mut message = reply.message().await?.into_owned();

    for col in 0..5usize {
        tokio::time::sleep(Duration::from_millis(1100)).await;
        let partial: Vec<Vec<String>> = (0..3)
            .map(|row| {
                (0..5)
                    .map(|c| {
                        if c <= col {
                            grid[row][c].clone()
                        } else {
                            "?".to_string()
                        }
                    })
                    .collect()
            })
            .collect();

        let _ = message
            .edit(
                ctx.serenity_context(),
                EditMessage::new().embed(build_spinning_embed(
                    &partial,
                    total_aposta,
                    col + 1,
                    &user.name,
                    &user_image_url,
                )),
            )
            .await;
    }

    // Resultado final
    let has_free_spins = result.free_spins > 0;
    let result_embed = build_result_embed(
        &result,
        total_aposta,
        saldo_final,
        &user.name,
        &user_image_url,
        false,
    );

    let components = if has_free_spins {
        vec![CreateActionRow::Buttons(vec![
            CreateButton::new(BTN_FREE_SPIN)
                .label(format!("🎁 Usar {} Free Spins!", result.free_spins))
                .style(ButtonStyle::Success),
        ])]
    } else {
        vec![]
    };

    let _ = message
        .edit(
            ctx.serenity_context(),
            EditMessage::new()
                .embed(result_embed.clone())
                .components(components),
        )
        .await;

    // Aguarda clique no botão de free spins (90s de timeout)
    if has_free_spins {
        let free_spins_total = result.free_spins;

        let interaction = message
            .await_component_interaction(ctx.serenity_context())
            .author_id(user.id)
            .timeout(Duration::from_secs(90))
            .await;

        if let Some(mci) = interaction {
            mci.create_response(
                ctx.serenity_context(),
                CreateInteractionResponse::UpdateMessage(
                    CreateInteractionResponseMessage::new()
                        .embed(result_embed)
                        .components(vec![]),
                ),
            )
            .await?;

            run_free_spins(
                &ctx,
                &mut message,
                aposta,
                total_aposta,
                free_spins_total,
                &user.id.to_string(),
                &user.name,
                &user_image_url,
            )
            .await?;
        } else {
            // Timeout: remove o botão
            let _ = message
                .edit(
                    ctx.serenity_context(),
                    EditMessage::new()
                        .embed(result_embed)
                        .components(vec![]),
                )
                .await;
        }
    }

    Ok(())
}

// ─── Free Spins ───────────────────────────────────────────────────────────────
#[allow(clippy::too_many_arguments)]
async fn run_free_spins(
    ctx: &Context<'_>,
    message: &mut poise::serenity_prelude::Message,
    aposta: i64,
    total_aposta: i64,
    num_spins: u32,
    user_id: &str,
    user_name: &str,
    user_image_url: &str,
) -> Result<(), Error> {
    let mut total_ganho: i64 = 0;
    let mut saldo_atual = get_user(user_id).await?.coins;

    // BUG #2 FIX: usa while loop para suportar re-trigger de free spins
    let mut remaining_spins = num_spins;
    let mut total_spins_awarded = num_spins;
    let mut spin_num = 0u32;

    while remaining_spins > 0 {
        spin_num += 1;
        remaining_spins -= 1;

        let grid = spin_grid();

        // BUG #1 FIX: estado inicial todo "?" separado do loop de reveal
        tokio::time::sleep(Duration::from_millis(800)).await;
        let _ = message
            .edit(
                ctx.serenity_context(),
                EditMessage::new().embed(
                    CreateEmbed::new()
                        .title(format!(
                            "🆓 Free Spin {}/{} • {}",
                            spin_num, total_spins_awarded, user_name
                        ))
                        .thumbnail(user_image_url)
                        .color(Colour::GOLD)
                        .description(render_grid(&vec![vec!["?".to_string(); 5]; 3]))
                        .field("Aposta por linha", format!("{} coins", aposta), true)
                        .field("Aposta total", format!("{} coins", total_aposta), true)
                        .field("Status", "🎲 Girando... (GRÁTIS!)", true)
                        .field("Total ganho até agora", format!("{} coins", total_ganho), true),
                ),
            )
            .await;

        // Revela coluna por coluna (5 iterações exatas, sem frame fantasma)
        for col in 0..5usize {
            tokio::time::sleep(Duration::from_millis(700)).await;

            let display_grid: Vec<Vec<String>> = (0..3)
                .map(|row| {
                    (0..5)
                        .map(|c| {
                            if c <= col {
                                grid[row][c].clone()
                            } else {
                                "?".to_string()
                            }
                        })
                        .collect()
                })
                .collect();

            let partial_embed = CreateEmbed::new()
                .title(format!(
                    "🆓 Free Spin {}/{} • {}",
                    spin_num, total_spins_awarded, user_name
                ))
                .thumbnail(user_image_url)
                .color(Colour::GOLD)
                .description(render_grid(&display_grid))
                .field("Aposta por linha", format!("{} coins", aposta), true)
                .field("Aposta total", format!("{} coins", total_aposta), true)
                .field("Status", format!("Revelando {}/5...", col + 1), true)
                .field("Total ganho até agora", format!("{} coins", total_ganho), true);

            let _ = message
                .edit(
                    ctx.serenity_context(),
                    EditMessage::new().embed(partial_embed),
                )
                .await;
        }

        let result = evaluate_spin(&grid, aposta);
        if result.payout > 0 {
            saldo_atual = update_coins(user_id, result.payout).await?.coins;
            total_ganho += result.payout;
        }

        // BUG #2 FIX: re-trigger de free spins quando scatter aparece durante free spins
        if result.free_spins > 0 {
            remaining_spins += result.free_spins;
            total_spins_awarded += result.free_spins;
        }

        let title = if result.free_spins > 0 {
            format!(
                "🆓 Free Spin {}/{} • Resultado ({}) ✨ +{} spins!",
                spin_num, total_spins_awarded, user_name, result.free_spins
            )
        } else {
            format!(
                "🆓 Free Spin {}/{} • Resultado ({})",
                spin_num, total_spins_awarded, user_name
            )
        };

        let spin_embed = build_result_embed(
            &result,
            total_aposta,
            saldo_atual,
            user_name,
            user_image_url,
            true,
        )
        .title(title);

        let _ = message
            .edit(ctx.serenity_context(), EditMessage::new().embed(spin_embed))
            .await;

        if remaining_spins > 0 {
            tokio::time::sleep(Duration::from_millis(2200)).await;
        }
    }

    // Resumo final dos free spins
    let summary_embed = CreateEmbed::new()
        .title(format!("🎰 Free Spins Concluídos! • {}", user_name))
        .thumbnail(user_image_url)
        .color(Colour::DARK_GREEN)
        .description(format!(
            "Você completou **{}** free spin(s) e ganhou **{}** coins no total! 🎉",
            spin_num, total_ganho
        ))
        .field("Free Spins Jogados", spin_num.to_string(), true)
        .field("Total Spins Concedidos", total_spins_awarded.to_string(), true)
        .field("Total Ganho", format!("{} coins", total_ganho), true)
        .field("Saldo Atual", format!("{} coins", saldo_atual), true);

    let _ = message
        .edit(
            ctx.serenity_context(),
            EditMessage::new().embed(summary_embed),
        )
        .await;

    Ok(())
}

// ─── Embeds ───────────────────────────────────────────────────────────────────
fn build_spinning_embed(
    grid: &[Vec<String>],
    total_aposta: i64,
    revealed_count: usize,
    user_name: &str,
    user_image_url: &str,
) -> CreateEmbed {
    CreateEmbed::new()
        .title(format!("🎰 Caça-níquel de {}", user_name))
        .thumbnail(user_image_url)
        .color(Colour::DARK_GOLD)
        .description(render_grid(grid))
        .field("Linhas Ativas", NUM_PAYLINES.to_string(), true)
        .field("Aposta Total", format!("{} coins", total_aposta), true)
        .field("Colunas reveladas", format!("{}/5", revealed_count), true)
        .field("Status", "🎲 Girando...", true)
        .field(
            "Tabela de Pagamento (mín 3 iguais, esq→dir)",
            "⭐ Scatter: 3=5× | 4=20× | 5=50× (da aposta total) + Free Spins\n\
             🃏 Wild substitui qualquer símbolo\n\
             💎200/50/10× | 👑150/35/7× | 💰100/25/5× | 🍀60/18/4.5×\n\
             🐒50/15/4× | 🍉25/10/3× | 🍊20/8/2× | 🍋15/6/1.5× | 🔔12/5/1.2× | 🍒10/4/1×",
            false,
        )
}

fn build_result_embed(
    result: &SpinResult,
    total_aposta: i64,
    saldo_final: i64,
    user_name: &str,
    user_image_url: &str,
    is_free_spin: bool,
) -> CreateEmbed {
    let won = result.payout > 0;
    let lucro = result.payout - if is_free_spin { 0 } else { total_aposta };

    let status_text = if won {
        if lucro >= 0 {
            format!(
                "Ganhou **{}** coin(s) e lucrou **{}** coin(s)! 🎉",
                result.payout, lucro
            )
        } else {
            format!(
                "Ganhou **{}** coin(s), mas ficou **{}** coin(s) no prejuízo na rodada.",
                result.payout, -lucro
            )
        }
    } else if is_free_spin {
        "Nenhum prêmio nesta jogada grátis.".to_string()
    } else {
        format!(
            "Não ganhou nada desta vez. Prejuízo: **{}** coin(s).",
            total_aposta
        )
    };

    let mut detalhes = String::new();

    if result.scatter_count >= 3 {
        let fs_text = if result.free_spins > 0 {
            format!(" + **{}** free spins!", result.free_spins)
        } else {
            String::new()
        };
        detalhes.push_str(&format!(
            "⭐ **Scatter ×{}** → **{}** coins{}\n",
            result.scatter_count, result.scatter_payout, fs_text
        ));
    }

    if result.line_results.is_empty() && result.scatter_count < 3 {
        detalhes.push_str("Nenhuma linha premiada.");
    } else {
        for (idx, r) in &result.line_results {
            detalhes.push_str(&format!(
                "**L{}**: {}×{} ({:.2}×) = **{}** coins\n",
                idx + 1,
                r.count,
                r.symbol,
                r.multiplier,
                r.payout
            ));
        }
    }

    let title_prefix = if is_free_spin {
        "🆓 Free Spin"
    } else {
        "🎰 Caça-níquel"
    };

    CreateEmbed::new()
        .title(format!("{} • Resultado ({})", title_prefix, user_name))
        .thumbnail(user_image_url)
        .color(if won { Colour::DARK_GREEN } else { Colour::DARK_RED })
        .description(render_grid(&result.grid))
        .field("Aposta Total", format!("{} coins", total_aposta), true)
        .field("Pagamento Total", format!("{} coins", result.payout), true)
        .field("Saldo Atual", format!("{} coins", saldo_final), true)
        .field("Status", status_text, false)
        .field("Detalhes", detalhes, false)
}

// ─── Renderização da grade ────────────────────────────────────────────────────
fn render_grid(grid: &[Vec<String>]) -> String {
    let mut lines = Vec::with_capacity(3);
    for row in grid {
        lines.push(format!(
            "│  {}  {}  {}  {}  {}  │",
            row[0], row[1], row[2], row[3], row[4]
        ));
    }
    format!(
        "```\n┌─────────────────────────────┐\n{}\n└─────────────────────────────┘\n```",
        lines.join("\n")
    )
}
