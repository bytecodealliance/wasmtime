use cap_std::net::Pool;

pub struct Network {
    pub pool: Pool,
    pub allow_ip_name_lookup: bool,
}
