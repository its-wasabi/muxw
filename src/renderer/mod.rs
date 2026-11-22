mod context;

pub struct Renderer {
    context: context::Context,
}

impl Renderer {
    pub fn new() -> eyre::Result<Self> {
        Ok(Self {
            context: context::Context::new()?,
        })
    }
}
