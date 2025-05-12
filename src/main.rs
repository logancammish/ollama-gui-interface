use ollama_rs::models::ModelOptions;
use ollama_rs::Ollama;
use ollama_rs::generation::completion::request::GenerationRequest;

#[tokio::main]
async fn main() {
    let prompt = "What is the capital of France?";
    let ollama = Ollama::default();
    
    let request = GenerationRequest::new(String::from("deepseek-r1:1.5b"), prompt)
        .options(ModelOptions::default())
        .system("You are a helpful AI assistant who has a strong devotion to the truth.\nYou are in a school environment, and you are to adhere to certain policies related to this. Begin talking now.");
    match ollama.generate(request).await {
        Ok(response) => println!("Response: {:?}", response.response),
        Err(err) => eprintln!("Generation Error: {:?}", err),
    }
}
