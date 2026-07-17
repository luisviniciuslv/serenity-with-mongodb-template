use std::cmp::Ordering;
use std::time::Duration;

use poise::serenity_prelude::{
    ButtonStyle, Colour, CreateActionRow, CreateButton, CreateEmbed, CreateInteractionResponse,
    CreateInteractionResponseMessage,
};
use poise::CreateReply;
use rand::seq::SliceRandom;

use crate::db::{get_user, record_bet, set_highlow_streak, update_coins};
use crate::{Context, Error};

const BTN_HIGH: &str = "highlow_high";
const BTN_LOW: &str = "highlow_low";
const BTN_CASHOUT: &str = "highlow_cashout";

#[poise::command(slash_command, prefix_command, user_cooldown = 5)]
pub async fn highlow(
    ctx: Context<'_>,
    #[description = "Valor da aposta por rodada ou 'allwin'"] aposta: String,
) -> Result<(), Error> {
    let user = ctx.author().clone();
    let user_id = user.id.to_string();
    let user_db = get_user(&user_id).await?;

    let aposta_val = if aposta.to_lowercase() == "allwin" {
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

    if aposta_val <= 0 {
        ctx.say("Aposta inválida.").await?;
        return Ok(());
    }

    if user_db.coins < aposta_val {
        ctx.say("Você não tem coins suficientes para essa aposta.")
            .await?;
        return Ok(());
    }

    let aposta = aposta_val;

    // The first round is paid as soon as the command starts.
    let saldo_apos_aposta = update_coins(&user_id, -aposta).await?.coins;
    let mut first_round_pending = true;

    let mut deck = shuffled_deck();
    let mut current_card = draw_card(&mut deck).expect("Deck should start with cards");
    let mut streak = user_db.highlow_streak;

    let reply = ctx
        .send(CreateReply {
            embeds: vec![build_waiting_embed(
                current_card,
                aposta,
                streak,
                saldo_apos_aposta,
            )],
            components: Some(default_components()),
            ..Default::default()
        })
        .await?;

    let mut message = reply.message().await?.into_owned();

    loop {
        let interaction = message
            .await_component_interaction(ctx.serenity_context())
            .author_id(user.id)
            .timeout(Duration::from_secs(60))
            .await;

        let Some(interaction) = interaction else {
            interaction_timeout_update(&ctx, &mut message, current_card, aposta, streak).await?;
            break;
        };

        let action_id = interaction.data.custom_id.as_str();

        if action_id == BTN_CASHOUT {
            process_cashout(&ctx, &interaction, &user_id, current_card, aposta, streak).await?;
            break;
        }

        if action_id != BTN_HIGH && action_id != BTN_LOW {
            continue;
        }

        if !pay_round_if_needed(
            &ctx,
            &interaction,
            &user_id,
            current_card,
            aposta,
            streak,
            &mut first_round_pending,
        )
        .await?
        {
            break;
        }

        let Some(next_card) = draw_card(&mut deck) else {
            process_deck_exhausted(&ctx, &interaction, &user_id, current_card, aposta, streak)
                .await?;
            break;
        };
        let did_win = is_winning_guess(action_id, current_card, next_card);

        if did_win {
            let previous_card = current_card;
            current_card = next_card;
            streak = process_round_win(
                &ctx,
                &interaction,
                &user_id,
                previous_card,
                next_card,
                aposta,
                streak,
            )
            .await?;
        } else {
            process_round_loss(
                &ctx,
                &interaction,
                &user_id,
                current_card,
                next_card,
                aposta,
            )
            .await?;
            break;
        }
    }

    Ok(())
}

async fn interaction_timeout_update(
    ctx: &Context<'_>,
    message: &mut poise::serenity_prelude::Message,
    current_card: Card,
    aposta: i64,
    streak: i64,
) -> Result<(), Error> {
    message
        .edit(
            ctx.serenity_context(),
            poise::serenity_prelude::EditMessage::new()
                .embed(build_error_embed(
                    current_card,
                    aposta,
                    streak,
                    "Tempo esgotado. Comando encerrado.",
                ))
                .components(vec![]),
        )
        .await?;

    Ok(())
}

async fn process_cashout(
    ctx: &Context<'_>,
    interaction: &poise::serenity_prelude::ComponentInteraction,
    user_id: &str,
    current_card: Card,
    aposta: i64,
    streak: i64,
) -> Result<(), Error> {
    let (bonus, saldo_atual) = settle_streak_reset(user_id, aposta, streak).await?;

    respond_with_embed(
        ctx,
        interaction,
        build_cashout_embed(current_card, aposta, streak, bonus, saldo_atual),
        vec![],
    )
    .await?;

    Ok(())
}

async fn pay_round_if_needed(
    _ctx: &Context<'_>,
    _interaction: &poise::serenity_prelude::ComponentInteraction,
    _user_id: &str,
    _current_card: Card,
    _aposta: i64,
    _streak: i64,
    first_round_pending: &mut bool,
) -> Result<bool, Error> {
    if *first_round_pending {
        *first_round_pending = false;
    }
    Ok(true)
}

async fn process_deck_exhausted(
    ctx: &Context<'_>,
    interaction: &poise::serenity_prelude::ComponentInteraction,
    user_id: &str,
    current_card: Card,
    aposta: i64,
    streak: i64,
) -> Result<(), Error> {
    let (bonus, saldo_atual) = settle_streak_reset(user_id, aposta, streak).await?;

    respond_with_embed(
        ctx,
        interaction,
        build_deck_exhausted_embed(current_card, aposta, streak, bonus, saldo_atual),
        vec![],
    )
    .await?;

    Ok(())
}

fn is_winning_guess(custom_id: &str, current_card: Card, next_card: Card) -> bool {
    let comparison = compare_cards(next_card, current_card);
    if custom_id == BTN_HIGH {
        comparison == Ordering::Greater
    } else {
        comparison == Ordering::Less
    }
}

async fn process_round_win(
    ctx: &Context<'_>,
    interaction: &poise::serenity_prelude::ComponentInteraction,
    user_id: &str,
    previous_card: Card,
    next_card: Card,
    aposta: i64,
    streak: i64,
) -> Result<i64, Error> {
    let next_streak = streak + 1;
    set_highlow_streak(user_id, next_streak).await?;
    let _ = record_bet(user_id, "highlow", aposta, true).await?;

    let multiplier = streak_multiplier(next_streak);
    let payout = ((aposta as f64) * multiplier).floor() as i64;
    let fresh_user = get_user(user_id).await?;

    respond_with_embed(
        ctx,
        interaction,
        build_win_embed(
            previous_card,
            next_card,
            aposta,
            next_streak,
            multiplier,
            payout,
            fresh_user.coins,
        ),
        default_components(),
    )
    .await?;

    Ok(next_streak)
}

async fn process_round_loss(
    ctx: &Context<'_>,
    interaction: &poise::serenity_prelude::ComponentInteraction,
    user_id: &str,
    current_card: Card,
    next_card: Card,
    aposta: i64,
) -> Result<(), Error> {
    set_highlow_streak(user_id, 0).await?;
    let _ = record_bet(user_id, "highlow", aposta, false).await?;
    let updated_user = get_user(user_id).await?;

    respond_with_embed(
        ctx,
        interaction,
        build_loss_embed(current_card, next_card, aposta, updated_user.coins),
        vec![],
    )
    .await?;

    Ok(())
}

async fn respond_with_embed(
    ctx: &Context<'_>,
    interaction: &poise::serenity_prelude::ComponentInteraction,
    embed: CreateEmbed,
    components: Vec<CreateActionRow>,
) -> Result<(), Error> {
    interaction
        .create_response(
            ctx.serenity_context(),
            CreateInteractionResponse::UpdateMessage(
                CreateInteractionResponseMessage::new()
                    .embed(embed)
                    .components(components),
            ),
        )
        .await?;

    Ok(())
}

async fn settle_streak_reset(user_id: &str, aposta: i64, streak: i64) -> Result<(i64, i64), Error> {
    let bonus = cashout_bonus(aposta, streak);
    let updated_user = if bonus > 0 {
        Some(update_coins(user_id, bonus).await?)
    } else {
        None
    };

    set_highlow_streak(user_id, 0).await?;

    let saldo_atual = updated_user
        .as_ref()
        .map(|user| user.coins)
        .unwrap_or(get_user(user_id).await?.coins);

    Ok((bonus, saldo_atual))
}

fn default_components() -> Vec<CreateActionRow> {
    vec![CreateActionRow::Buttons(vec![
        CreateButton::new(BTN_HIGH)
            .label("Maior")
            .style(ButtonStyle::Success),
        CreateButton::new(BTN_LOW)
            .label("Menor")
            .style(ButtonStyle::Primary),
        CreateButton::new(BTN_CASHOUT)
            .label("Sacar")
            .style(ButtonStyle::Danger),
    ])]
}

fn build_waiting_embed(
    current_card: Card,
    aposta: i64,
    streak: i64,
    saldo_atual: i64,
) -> CreateEmbed {
    CreateEmbed::new()
        .title("HighLow")
        .color(Colour::DARK_BLUE)
        .description(format!(
            "Carta atual:\n{}\n\nAposta por rodada: **{}**\nStreak atual: **{}**\nSaldo atual: **{}** coin(s)\nBônus de saque agora: **{}** coin(s)\n\nClique em **Maior** se acha que a próxima carta será maior.\nClique em **Menor** se acha que a próxima carta será menor.",
            card_text(current_card),
            aposta,
            streak,
            saldo_atual,
            cashout_bonus(aposta, streak)
        ))
}

fn build_win_embed(
    previous_card: Card,
    revealed_card: Card,
    aposta: i64,
    streak: i64,
    multiplier: f64,
    payout: i64,
    saldo_atual: i64,
) -> CreateEmbed {
    CreateEmbed::new()
        .title("HighLow • Acertou")
        .color(Colour::DARK_GREEN)
        .description(format!(
            "Carta anterior:\n{}\nNova carta:\n{}\n\nVocê venceu esta rodada!\nAposta: **{}**\nMultiplicador da streak: **{:.2}x**\nPagamento: **{}**\nSaldo atual: **{}** coin(s)",
            card_text(previous_card),
            card_text(revealed_card),
            aposta,
            multiplier,
            payout,
            saldo_atual
        ))
        .field("Streak", streak.to_string(), true)
        .field("Saque agora", cashout_bonus(aposta, streak).to_string(), true)
        .field("Próxima jogada", "Escolha Maior ou Menor novamente.", true)
}

fn build_loss_embed(
    previous_card: Card,
    revealed_card: Card,
    aposta: i64,
    saldo_atual: i64,
) -> CreateEmbed {
    CreateEmbed::new()
        .title("HighLow • Fim de jogo")
        .color(Colour::DARK_RED)
        .description(format!(
            "Carta anterior:\n{}\nNova carta:\n{}\n\nVocê errou a previsão.\nPerda desta rodada: **{}**\nSaldo atual: **{}** coin(s)\nStreak resetada para **0**.",
            card_text(previous_card),
            card_text(revealed_card),
            aposta,
            saldo_atual
        ))
}

fn build_cashout_embed(
    current_card: Card,
    aposta: i64,
    streak: i64,
    bonus: i64,
    saldo_atual: i64,
) -> CreateEmbed {
    CreateEmbed::new()
        .title("HighLow • Saque realizado")
        .color(Colour::DARK_GREY)
        .description(format!(
            "Você sacou e encerrou a sessão.\n\nCarta atual:\n{}\nAposta por rodada: **{}**\nStreak no saque: **{}**\nBônus recebido: **{}** coin(s)\nSaldo atual: **{}** coin(s)\nStreak resetada para **0**.",
            card_text(current_card),
            aposta,
            streak,
            bonus,
            saldo_atual
        ))
}

fn build_deck_exhausted_embed(
    current_card: Card,
    aposta: i64,
    streak: i64,
    bonus: i64,
    saldo_atual: i64,
) -> CreateEmbed {
    CreateEmbed::new()
        .title("HighLow • Fim da sessão")
        .color(Colour::DARK_ORANGE)
        .description(format!(
            "As cartas válidas acabaram sem reposição, então a sessão foi encerrada automaticamente.\n\nCarta atual:\n{}\nAposta por rodada: **{}**\nStreak final: **{}**\nSaque automático: **{}** coin(s)\nSaldo atual: **{}** coin(s)\nStreak resetada para **0**.",
            card_text(current_card),
            aposta,
            streak,
            bonus,
            saldo_atual
        ))
}

fn build_error_embed(current_card: Card, aposta: i64, streak: i64, message: &str) -> CreateEmbed {
    CreateEmbed::new()
        .title("HighLow")
        .color(Colour::DARK_ORANGE)
        .description(format!(
            "{}\n\nCarta atual:\n{}\nAposta por rodada: **{}**\nStreak atual: **{}**",
            message,
            card_text(current_card),
            aposta,
            streak
        ))
}

fn shuffled_deck() -> Vec<Card> {
    let mut rng = rand::rng();
    let mut suits = Suit::all();
    suits.shuffle(&mut rng);

    let mut deck = Vec::with_capacity(52);

    for suit in suits {
        let mut ranks: Vec<i64> = (1..=13).collect();
        ranks.shuffle(&mut rng);

        for rank in ranks {
            deck.push(Card { rank, suit });
        }
    }

    deck.reverse();
    deck
}

fn draw_card(deck: &mut Vec<Card>) -> Option<Card> {
    deck.pop()
}

fn card_rank(value: i64) -> &'static str {
    match value {
        1 => "A",
        11 => "J",
        12 => "Q",
        13 => "K",
        2 => "2",
        3 => "3",
        4 => "4",
        5 => "5",
        6 => "6",
        7 => "7",
        8 => "8",
        9 => "9",
        10 => "10",
        _ => "?",
    }
}

#[derive(Clone, Copy)]
struct Card {
    rank: i64,
    suit: Suit,
}

#[derive(Clone, Copy)]
enum Suit {
    Hearts,
    Diamonds,
    Clubs,
    Spades,
}

impl Suit {
    fn all() -> [Suit; 4] {
        [Suit::Hearts, Suit::Diamonds, Suit::Clubs, Suit::Spades]
    }

    fn symbol(self) -> &'static str {
        match self {
            Suit::Hearts => "♥",
            Suit::Diamonds => "♦",
            Suit::Clubs => "♣",
            Suit::Spades => "♠",
        }
    }

}

fn compare_cards(left: Card, right: Card) -> Ordering {
    left.rank.cmp(&right.rank)
}

fn card_text(card: Card) -> String {
    let rank = card_rank(card.rank);
    format!(
        "```\n┌───────┐\n│  {:<2} {} │\n└───────┘\n```",
        rank,
        card.suit.symbol()
    )
}

fn streak_multiplier(streak: i64) -> f64 {
    if streak <= 0 {
        return 1.20;
    }

    let start = 1.20;
    let end = 8.00;
    let max_streak = 13.0;
    let step = (end - start) / (max_streak - 1.0);

    (start + ((streak - 1) as f64 * step)).min(end)
}

fn cashout_bonus(aposta: i64, streak: i64) -> i64 {
    if streak <= 0 {
        return 0;
    }

    ((aposta as f64) * streak_multiplier(streak)).floor() as i64
}
