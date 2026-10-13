use crate::{ErrorResponse, WasiHttpCtx};
use http::uri::{Authority, PathAndQuery, Scheme};

/// Accounting to limit the size of the strings making up a request's method,
/// scheme, authority, and path-with-query.
///
/// Every place a request's strings enter the host, whether set by a guest or
/// read off the wire, goes through [`RequestStringsValidator`].
///
/// Built-in methods (`GET`, `POST`, ...) and schemes (`http`, `https`) don't
/// allocate, so they don't count toward the limit.
#[derive(Debug, Clone)]
pub(crate) struct RequestStringsValidator {
    limit: usize,
    method: usize,
    scheme: usize,
    authority: usize,
    path_with_query: usize,
}

#[derive(Clone, Copy)]
enum Field {
    Method,
    Scheme,
    Authority,
    PathWithQuery,
}

impl RequestStringsValidator {
    /// Create a validator using the limit configured in
    /// `ctx`.
    pub(crate) fn new(ctx: &WasiHttpCtx) -> Self {
        Self {
            limit: ctx.request_strings_size_limit,
            method: 0,
            scheme: 0,
            authority: 0,
            path_with_query: 0,
        }
    }

    fn slot(&mut self, field: Field) -> &mut usize {
        match field {
            Field::Method => &mut self.method,
            Field::Scheme => &mut self.scheme,
            Field::Authority => &mut self.authority,
            Field::PathWithQuery => &mut self.path_with_query,
        }
    }

    fn total(&self) -> usize {
        self.method + self.scheme + self.authority + self.path_with_query
    }

    fn set<T>(
        &mut self,
        field: Field,
        len: usize,
        parse: impl FnOnce() -> Option<T>,
    ) -> Result<T, ()> {
        let current = *self.slot(field);
        if self.total() - current + len > self.limit {
            return Err(());
        }
        let value = parse().ok_or(())?;
        *self.slot(field) = len;
        Ok(value)
    }

    /// Set a method not among the built-in variants.
    pub(crate) fn set_other_method(&mut self, method: &str) -> Result<http::Method, ()> {
        self.set(Field::Method, method.len(), || method.parse().ok())
    }

    /// Set one of the built-in methods, which don't count toward the limit.
    pub(crate) fn set_builtin_method(&mut self) {
        self.method = 0;
    }

    /// Set a scheme other than `http` or `https`.
    pub(crate) fn set_other_scheme(&mut self, scheme: &str) -> Result<Scheme, ()> {
        self.set(Field::Scheme, scheme.len(), || scheme.parse().ok())
    }

    /// Set no scheme, or one of the built-in schemes, neither of which count
    /// toward the limit.
    pub(crate) fn set_builtin_scheme(&mut self) {
        self.scheme = 0;
    }

    pub(crate) fn set_authority(
        &mut self,
        authority: Option<String>,
    ) -> Result<Option<Authority>, ()> {
        match authority {
            Some(a) => self
                .set(Field::Authority, a.len(), || crate::parse_authority(a).ok())
                .map(Some),
            None => {
                self.authority = 0;
                Ok(None)
            }
        }
    }

    pub(crate) fn set_path_with_query(
        &mut self,
        path_with_query: Option<&str>,
    ) -> Result<Option<PathAndQuery>, ()> {
        match path_with_query {
            Some(p) => self
                .set(Field::PathWithQuery, p.len(), || {
                    crate::parse_path_with_query(p)
                })
                .map(Some),
            None => {
                self.path_with_query = 0;
                Ok(None)
            }
        }
    }

    /// Account for a request built by the host from already-parsed parts,
    /// such as one received off the wire.
    ///
    /// On failure, the returned error carries an [`ErrorResponse`] with status
    /// 400 so the request can be rejected before reaching the guest.
    pub(crate) fn host_parts(
        &mut self,
        method: &http::Method,
        scheme: Option<&Scheme>,
        authority: Option<&Authority>,
        path_with_query: Option<&PathAndQuery>,
    ) -> wasmtime::Result<()> {
        self.method = if is_builtin_method(method) {
            0
        } else {
            method.as_str().len()
        };
        self.scheme = match scheme {
            Some(s) if *s != Scheme::HTTP && *s != Scheme::HTTPS => s.as_str().len(),
            _ => 0,
        };
        self.authority = authority.map_or(0, |a| a.as_str().len());
        self.path_with_query = path_with_query.map_or(0, |p| p.as_str().len());
        if self.total() > self.limit {
            return Err(wasmtime::Error::msg(format!(
                "request method, scheme, authority, and path with query total {} bytes, \
                 exceeding the size limit of {} bytes",
                self.total(),
                self.limit,
            ))
            .context(ErrorResponse::new(http::StatusCode::BAD_REQUEST)));
        }
        Ok(())
    }
}

fn is_builtin_method(method: &http::Method) -> bool {
    use http::Method as M;
    [
        M::GET,
        M::HEAD,
        M::POST,
        M::PUT,
        M::DELETE,
        M::CONNECT,
        M::OPTIONS,
        M::TRACE,
        M::PATCH,
    ]
    .contains(method)
}

#[cfg(test)]
mod tests {
    use super::RequestStringsValidator;

    #[test]
    fn total_is_limited() {
        let mut v = RequestStringsValidator::new(&crate::WasiHttpCtx {
            field_size_limit: 0,
            request_strings_size_limit: 10,
        });
        assert!(v.set_path_with_query(Some("/aaaa")).is_ok());
        assert!(v.set_authority(Some("bbbbb".into())).is_ok());
        assert!(v.set_other_method("C").is_err());
        assert!(v.set_other_scheme("c").is_err());

        // Built-ins don't count.
        v.set_builtin_method();
        v.set_builtin_scheme();

        // Replacing a field releases its old size.
        assert!(v.set_path_with_query(Some("/")).is_ok());
        assert!(v.set_other_method("CCCC").is_ok());
        assert!(v.set_authority(None).is_ok());
        assert!(v.set_other_scheme("dddddd").is_err());
        assert!(v.set_other_scheme("ddddd").is_ok());
    }

    #[test]
    fn rejected_values_are_not_recorded() {
        let mut v = RequestStringsValidator::new(&crate::WasiHttpCtx {
            field_size_limit: 0,
            request_strings_size_limit: 10,
        });
        // Fits, but fails to parse.
        assert!(v.set_authority(Some("a:b".into())).is_err());
        assert!(v.set_path_with_query(Some("/aaaaaaaaa")).is_ok());
        // Over the limit.
        assert!(v.set_other_method("X").is_err());
        assert!(v.set_path_with_query(Some("/aaaaaaaaa")).is_ok());
    }
}
