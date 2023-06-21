use std::sync::atomic::Ordering;

use anyhow::{Context as AnyhowContext, Result};
use serenity::{async_trait, futures::StreamExt, model::prelude::Message, prelude::Context};

use crate::{ai::directed_at_bot, ACTIVE_CONVO};

#[async_trait]
pub trait MessageExt {
    async fn is_own(&self, ctx: &Context) -> Result<bool>;
    async fn requires_response(&self, ctx: &Context) -> Result<bool>;
    async fn reply_maybe_long(&self, ctx: &Context, response: String) -> Result<()>;
}

#[async_trait]
impl MessageExt for Message {
    async fn is_own(&self, ctx: &Context) -> Result<bool> {
        Ok(self.author.id == ctx.http.get_current_user().await?.id)
    }

    async fn requires_response(&self, ctx: &Context) -> Result<bool> {
        let references_own_message = match self.referenced_message.as_ref() {
            Some(message) => message.is_own(&ctx).await.unwrap_or_default(),
            None => false,
        };

        if self.mentions_me(&ctx).await?
            || references_own_message
            || self.content.to_lowercase().contains("astro")
        {
            return Ok(true);
        }

        if ACTIVE_CONVO.load(Ordering::Relaxed) != self.channel_id.0 {
            return Ok(false);
        }

        let mut last_5_messages = self
            .channel_id
            .messages_iter(&ctx.http)
            .take(5)
            .map(|message| message.unwrap().clone())
            .collect::<Vec<_>>()
            .await;
        last_5_messages.reverse();

        directed_at_bot(ctx.clone(), last_5_messages).await
    }

    async fn reply_maybe_long(&self, ctx: &Context, mut response: String) -> Result<()> {
        let mut replied = false;
        while let Some(overflow_length) = Message::overflow_length(&response) {
            let (first, second) = response.split_at(response.len() - overflow_length);
            if !replied {
                self.reply(ctx, first)
                    .await
                    .context("Failed to send message")?;
                replied = true;
            } else {
                self.channel_id
                    .say(&ctx, first)
                    .await
                    .context("Failed to send message")?;
            }

            response = second.to_string();
        }

        if !replied {
            self.reply(ctx, response)
                .await
                .context("Failed to send message")?;
        } else {
            self.channel_id
                .say(&ctx, response)
                .await
                .context("Failed to send message")?;
        }

        Ok(())
    }
}
