fn main() -> Result<(), trigora_client::TrigoraError> {
    let run = trigora_client::start("approval", None)?;
    run.send("approved", serde_json::json!(true))?;
    println!("{}", run.result()?);
    Ok(())
}
