use poise::{
    serenity_prelude::{Context, FullEvent},
    FrameworkContext,
};

use crate::{Data, Error};

pub mod ready;

pub async fn handler(
    ctx: &Context,
    event: &FullEvent,
    _framework: FrameworkContext<'_, Data, Error>,
    _data: &Data,
) -> Result<(), Error> {
    match event {
        FullEvent::Ready { data_about_bot, .. } => {
            ready::ready(data_about_bot, ctx).await.unwrap();
        }

        _ => {}
    }

    Ok(())
}
