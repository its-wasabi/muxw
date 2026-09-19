mod context;

pub struct Renderer {
    context: context::Context,
}

impl Renderer {
    pub fn new() -> Result<Self, Box<dyn std::error::Error>> {
        Ok(Self {
            context: context::Context::new()?,
        })
    }
}
