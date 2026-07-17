use poise::serenity_prelude::{Colour, CreateEmbed, CreateEmbedFooter};
use poise::CreateReply;

use crate::{Context, Error};

/// Exibe as regras completas do cassino
#[poise::command(slash_command, prefix_command, rename = "help")]
pub async fn help_cassino(
    ctx: Context<'_>,
    #[description = "Tópico: niquel, highlow, poi (ou vazio para visão geral)"] topico: Option<
        String,
    >,
) -> Result<(), Error> {
    let topico = topico.unwrap_or_default().to_lowercase();

    let embed = match topico.trim() {
        "niquel" | "níquel" | "caça-níquel" => embed_niquel(),
        "highlow" | "hl" => embed_highlow(),
        "poi" | "par" | "impar" | "par ou ímpar" => embed_poi(),
        _ => embed_geral(),
    };

    ctx.send(CreateReply {
        embeds: vec![embed],
        ..Default::default()
    })
    .await?;

    Ok(())
}

// ─── Embed Geral ──────────────────────────────────────────────────────────────
fn embed_geral() -> CreateEmbed {
    CreateEmbed::new()
        .title("🎰 Cassino — Guia Completo")
        .color(Colour::GOLD)
        .description(
            "Bem-vindo ao cassino! Aqui você encontra todos os jogos disponíveis.\n\
             Use `!help <jogo>` para ver as regras detalhadas de cada um.\n\u{200b}",
        )
        .field(
            "🎰 Caça-Níquel  →  `!niquel <aposta>`",
            "Grid 5×3 com 20 linhas de pagamento ativas.\n\
             A `<aposta>` é o valor **total** apostado — nunca perde mais do que digitou.\n\
             Símbolos especiais: ⭐ Scatter (bônus + Free Spins) e 🃏 Wild (coringa).\n\
             > `!help niquel` para detalhes completos.",
            false,
        )
        .field(
            "🃏 HighLow  →  `!highlow <aposta>`",
            "Adivinhe se a próxima carta é **maior** ou **menor**.\n\
             Acerte em sequência para multiplicar seus ganhos (até **8×**).\n\
             Saque a qualquer momento para garantir o prêmio.\n\
             > `!help highlow` para detalhes completos.",
            false,
        )
        .field(
            "🎲 Par ou Ímpar  →  `!poi <par|impar> <aposta>`",
            "Adivinhe se o número sorteado (0–99) é par ou ímpar.\n\
             Acerto paga **2×** a aposta. Probabilidade: **50%**.\n\
             > `!help poi` para detalhes completos.",
            false,
        )
        .field(
            "💡 Dica geral",
            "Todos os jogos aceitam **`allwin`** como aposta para arriscar tudo de uma vez!\n\
             Use `!rec` para ganhar coins gratuitos todos os dias.",
            false,
        )
        .footer(CreateEmbedFooter::new(
            "!help niquel  |  !help highlow  |  !help poi",
        ))
}

// ─── Embed Caça-Níquel ────────────────────────────────────────────────────────
fn embed_niquel() -> CreateEmbed {
    CreateEmbed::new()
        .title("🎰 Caça-Níquel — Regras Completas")
        .color(Colour::DARK_GOLD)
        .description(
            "**Comando:** `!niquel <aposta>` ou `/niquel <aposta>`\n\
             A `<aposta>` é o valor **total** que você arrisca na rodada.\n\
             Internamente ela é dividida em **20 linhas de pagamento** — \
             você nunca perde mais do que digitou.\n\u{200b}",
        )
        .field(
            "📐 Como funciona",
            "• Grid de **5 colunas × 3 linhas** sorteado a cada giro\n\
             • **20 paylines** avaliadas (horizontal, diagonal, zig-zag)\n\
             • Para pagar: **3 ou mais símbolos iguais da esquerda → direita**\n\
             • Prêmio por linha = `(aposta ÷ 20) × multiplicador`\n\
             • Prêmio total = soma de todas as linhas vencedoras + Scatter",
            false,
        )
        .field(
            "🃏 Wild (Coringa)",
            "Substitui qualquer símbolo regular na sequência.\n\
             5× Wild puro paga **200×** (igual ao 💎).",
            true,
        )
        .field(
            "⭐ Scatter (Bônus)",
            "Vale em **qualquer posição** da grade:\n\
             **3⭐** → 5× aposta total + **5 Free Spins**\n\
             **4⭐** → 20× aposta total + **10 Free Spins**\n\
             **5⭐** → 50× aposta total + **15 Free Spins**",
            true,
        )
        .field(
            "🆓 Free Spins",
            "Giros **gratuitos** sem custo adicional.\n\
             Se Scatter cair durante um Free Spin → **re-trigger** (mais spins adicionados).",
            false,
        )
        .field(
            "💎 Tabela de Multiplicadores  (3× / 4× / 5× iguais)",
            "```\n\
💎 Diamante  —  10× / 50× / 200×  ← LENDÁRIO\n\
👑 Coroa     —   7× / 35× / 150×\n\
💰 Saco $    —   5× / 25× / 100×\n\
🍀 Trevo     — 4.5× / 18× /  60×\n\
🐒 Macaco    —   4× / 15× /  50×\n\
🍉 Melancia  —   3× / 10× /  25×\n\
🍊 Laranja   —   2× /  8× /  20×\n\
🍋 Limão     — 1.5× /  6× /  15×\n\
🔔 Sino      — 1.2× /  5× /  12×\n\
🍒 Cereja    —   1× /  4× /  10×  ← mais comum\
```",
            false,
        )
        .field(
            "💡 Exemplo",
            "`!niquel 200` → você arrisca **200 coins**.\n\
             Se cair 3× 🍉 em uma linha: `(200÷20) × 3 = 30 coins` de prêmio nessa linha.\n\
             `!niquel allwin` → aposta todos os seus coins de uma vez.",
            false,
        )
        .footer(CreateEmbedFooter::new("Voltar: !help cassino"))
}

// ─── Embed HighLow ────────────────────────────────────────────────────────────
fn embed_highlow() -> CreateEmbed {
    CreateEmbed::new()
        .title("🃏 HighLow — Regras Completas")
        .color(Colour::DARK_BLUE)
        .description(
            "**Comando:** `!highlow <aposta>` ou `/highlow <aposta>`\n\
             Jogo de cartas: adivinhe se a próxima carta é maior ou menor.\n\u{200b}",
        )
        .field(
            "📐 Como funciona",
            "1. Uma carta do baralho embaralhado é revelada\n\
             2. Você clica **Maior** ou **Menor**\n\
             3. A próxima carta é revelada\n\
             4. **Acertou** → streak +1, prêmio creditado; continue ou saque\n\
             5. **Errou** → perde a aposta da rodada e streak volta a **0**",
            false,
        )
        .field(
            "🔢 Ordem das cartas",
            "**A < 2 < 3 < 4 < 5 < 6 < 7 < 8 < 9 < 10 < J < Q < K**\n\
             Em empate de valor, o naipe desempata:\n\
             ♦ Ouros < ♠ Espadas < ♥ Copas < ♣ Paus",
            false,
        )
        .field(
            "📈 Multiplicador por Streak",
            "```\n\
Streak  1 →  1.20×\n\
Streak  3 →  1.86×\n\
Streak  6 →  2.86×\n\
Streak 10 →  4.54×\n\
Streak 13 →  8.00×  ← máximo\
```\n\
Cresce linearmente de **1.20×** até **8.00×**.",
            false,
        )
        .field(
            "💰 Saque",
            "Clique **Sacar** a qualquer momento:\n\
             Recebe `aposta × multiplicador_atual` e streak vai a 0.\n\
             O saque acontece automaticamente se o baralho acabar.",
            false,
        )
        .field(
            "⏱️ Timeout",
            "**60 segundos** para escolher em cada rodada.\n\
             Se o tempo esgotar, o jogo encerra sem perda adicional.",
            false,
        )
        .field(
            "💡 Dica",
            "`!highlow allwin` → aposta todos os seus coins.\n\
             Streaks altas são tentadoras — **saque na hora certa!**",
            false,
        )
        .footer(CreateEmbedFooter::new("Voltar: !help cassino"))
}

// ─── Embed Par ou Ímpar ───────────────────────────────────────────────────────
fn embed_poi() -> CreateEmbed {
    CreateEmbed::new()
        .title("🎲 Par ou Ímpar — Regras Completas")
        .color(Colour::DARK_GREEN)
        .description(
            "**Comando:** `!poi <par|impar> <aposta>` ou `/poi <par|impar> <aposta>`\n\
             O jogo mais simples do cassino — aposte e torça!\n\u{200b}",
        )
        .field(
            "📐 Como funciona",
            "1. Você escolhe **par** ou **impar** e define o valor da aposta\n\
             2. Um número entre **0 e 99** é sorteado\n\
             3. **Acertou** → recebe **2× a aposta**\n\
             4. **Errou** → perde a aposta",
            false,
        )
        .field(
            "🎯 Probabilidade",
            "Pares: 0, 2, 4 … 98 → **50 números**\n\
             Ímpares: 1, 3, 5 … 99 → **50 números**\n\
             Chance de acerto: **50%** | Payout: **2×** (jogo justo)",
            false,
        )
        .field(
            "📝 Atalhos disponíveis",
            "`!par <aposta>` — aposta direto em par\n\
             `!impar <aposta>` — aposta direto em ímpar\n\
             `!poi par allwin` — aposta tudo em par\n\
             `!poi impar allwin` — aposta tudo em ímpar",
            false,
        )
        .footer(CreateEmbedFooter::new(
            "Ideal para dobrar coins rapidamente. Voltar: !help cassino",
        ))
}
