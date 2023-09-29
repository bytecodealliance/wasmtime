use cap_std::net::Pool;

pub struct Network(pub(crate) Pool);

impl Network {
    pub fn new(pool: Pool) -> Self {
        Self(pool)
    }
}
