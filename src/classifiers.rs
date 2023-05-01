use anyhow::{anyhow, Result};
use indoc::indoc;
use openai::moderations::Moderation;

use crate::ai::generate_response;

pub async fn is_allowed(message: &str) -> Result<bool> {
    let moderation = Moderation::builder(message)
        .model("text-moderation-latest")
        .create()
        .await??;
    moderation
        .results
        .first()
        .map(|result| result.flagged)
        .ok_or(anyhow!("No result"))
}

pub async fn is_instruction(query: &str) -> Result<bool> {
    let system = indoc! {"
        You are a classifier which decides if a message to a chat bot is an instruction that should be remembered.
        Respond with only True or False and nothing else.

        For Example:
        User: Astro from now on, respond only in emojis.
        Classifier: True

        User: Astro I'd like you to be a little cuter moving forward.
        Classifier: True

        User: Ok astro stop responding with emojis
        Classifier: True

        User: Astro never say your name
        Classifier: True

        User: Astro where should I eat today?
        Classifier: False"};

    let classification =
        generate_response("Classifier", &system, None, Some(query.to_string()), false)
            .await
            .unwrap_or_default();
    Ok(classification.contains("True"))
}

pub async fn is_confirmation(query: &str) -> Result<bool> {
    let system = indoc! {"
        You are a classifier which decides if a response to a chatbot is an agreement or confirmation.
        Respond with only True or False and nothing else.

        For Example:
        User: Yes
        Classifier: True

        User: Yeah that works
        Classifier: True

        User: Perfect
        Classifier: True

        User: No
        Classifier: False

        User: I don't think so
        Classifier: False

        User: Astro sorry, he not understand the question, can you please rephrase it?
        Classifier: False"};

    let classification =
        generate_response("Classifier", &system, None, Some(query.to_string()), false)
            .await
            .unwrap_or_default();
    Ok(classification.contains("True"))
}
