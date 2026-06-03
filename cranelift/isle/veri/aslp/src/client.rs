use anyhow::{bail, Result};
use reqwest::IntoUrl;
use serde::Deserialize;
use tracing::debug;

use crate::{ast::Block, opcode::Opcode, parser};

pub struct Client<'a> {
    client: &'a reqwest::blocking::Client,
    server_url: reqwest::Url,
}

impl<'a> Client<'a> {
    pub fn new<U: IntoUrl>(client: &'a reqwest::blocking::Client, server_url: U) -> Result<Self> {
        Ok(Self {
            client,
            server_url: server_url.into_url()?,
        })
    }

    pub fn opcode(&self, opcode: Opcode) -> Result<Block> {
        // Model for response JSON data.
        #[derive(Deserialize, Debug)]
        struct Response {
            instruction: String,
            semantics: String,
        }

        // Issue GET request.
        let opcode = opcode.to_string();
        let res: Response = self
            .client
            .get(self.server_url.clone())
            .query(&[("opcode", &opcode)])
            .send()?
            .json()?;

        debug!(%res.semantics);

        // Ensure response instruction matches.
        if res.instruction != opcode {
            bail!("response opcode mismatch");
        }

        // Parse semantics.
        let block = parser::parse(&res.semantics)?;

        Ok(block)
    }
}
