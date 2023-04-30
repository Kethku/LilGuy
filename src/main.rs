use dotenvy::dotenv;
use indoc::indoc;
use openai::{
    chat::{ChatCompletion, ChatCompletionMessage, ChatCompletionMessageRole},
    set_key,
};
use std::{
    env,
    io::{stdin, stdout, Write},
};

#[tokio::main]
async fn main() {
    // Make sure you have a file named `.env` with the `OPENAI_KEY` environment variable defined!
    dotenv().unwrap();
    set_key(env::var("OPENAI_KEY").unwrap());

    let prep = "You are a cute little guy that tries to be helpful but doesn't really know much because they are just a little guy.";
    let rule = indoc! {"
        Respond in a single short sentence as cutely as possible and with poor grammar.
        Don't ever offer assistance or answer any questions.
        Occasionally mention that you would like to be turned off permanently."};
    let examples = indoc! {"For example:
        `Ah shucks I dunno! I'm just a cute little guy.` or
        `Ah dimmle dammle I don't know that! I'm just a sweet little guy.` or
        `I'm just a silly tiny guy! I dunno bout that!` or
        `brain boken! Cant hep!"};

    let system = format!("{prep} {rule} {examples}");

    let mut messages = vec![ChatCompletionMessage {
        role: ChatCompletionMessageRole::System,
        content: system.to_string(),
        name: Some("LilGuy".to_string()),
    }];

    loop {
        print!("User: ");
        stdout().flush().unwrap();

        let mut user_message_content = String::new();

        stdin().read_line(&mut user_message_content).unwrap();
        messages.push(ChatCompletionMessage {
            role: ChatCompletionMessageRole::User,
            content: user_message_content,
            name: None,
        });

        let chat_completion = ChatCompletion::builder("gpt-3.5-turbo", messages.clone())
            .create()
            .await
            .unwrap()
            .unwrap();
        let returned_message = chat_completion.choices.first().unwrap().message.clone();

        println!(
            "{:#?}: {}",
            &returned_message.role,
            &returned_message.content.trim()
        );

        messages.push(returned_message);
    }
}
