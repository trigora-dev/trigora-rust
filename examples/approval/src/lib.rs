use trigora::{effect, wait_for_event};

pub struct Approval {
    pub result: f64,
    pub review: bool,
}

pub async fn main() -> Result<Approval, String> {
    let amount: f64 = 42.0;
    let result = effect("generate", move || amount).await?;
    let review: bool = wait_for_event("approved").await?;
    Ok(Approval {
        result: result,
        review: review,
    })
}
