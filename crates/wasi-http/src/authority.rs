//! Shared validation for wasi-http outgoing request authorities (p2 and p3).

/// Parse an outgoing request `authority`, rejecting a malformed value.
///
/// `http::uri::Authority` accepts an authority whose port section is empty or
/// non-numeric (for example `example.com:` or `example.com:abc`), so the port
/// is validated here as well. A `:` inside an IPv6 literal host such as `[::1]`
/// is part of the host rather than a port delimiter, so the port is only looked
/// for after any closing bracket.
pub(crate) fn parse_authority(authority: String) -> Result<http::uri::Authority, ()> {
    let has_port = match authority.rfind(']') {
        Some(i) => authority[i..].contains(':'),
        None => authority.contains(':'),
    };
    let authority = http::uri::Authority::try_from(authority).map_err(|_| ())?;
    if has_port && authority.port_u16().is_none() {
        return Err(());
    }
    Ok(authority)
}

#[cfg(test)]
mod tests {
    use super::parse_authority;

    #[test]
    fn authority_accepts_ipv6_and_validates_ports() {
        // Host names and IPv4 literals, with and without an explicit port.
        assert!(parse_authority("example.com".into()).is_ok());
        assert!(parse_authority("example.com:443".into()).is_ok());
        assert!(parse_authority("127.0.0.1:80".into()).is_ok());

        // Bracketed IPv6 literals: the colons belong to the host, so a missing
        // port must still be accepted and not mistaken for an empty port.
        assert!(parse_authority("[::1]".into()).is_ok());
        assert!(parse_authority("[2001:db8::1]".into()).is_ok());
        assert!(parse_authority("[::1]:443".into()).is_ok());

        // When a port section is present it must be a valid number.
        assert!(parse_authority("example.com:".into()).is_err());
        assert!(parse_authority("example.com:abc".into()).is_err());
        assert!(parse_authority("example.com:65536".into()).is_err());
        assert!(parse_authority("[::1]:abc".into()).is_err());
    }
}
