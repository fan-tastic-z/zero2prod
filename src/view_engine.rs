use std::path::Path;

use serde::Serialize;

use crate::{errors::Error, Result};

const VIEWS_DIR: &str = "assets/views";

#[derive(Debug, Clone)]
pub struct TeraView {
    pub tera: tera::Tera,
    pub default_context: tera::Context,
}

impl TeraView {
    pub fn build() -> Result<Self> {
        Self::from_custom_dir(&VIEWS_DIR)
    }

    pub fn from_custom_dir<P: AsRef<Path>>(path: &P) -> Result<Self> {
        if !path.as_ref().exists() {
            return Err(Error::string(&format!(
                "missing views directory: `{}`",
                path.as_ref().display()
            )));
        }

        let tera = tera::Tera::new(
            path.as_ref()
                .join("**")
                .join("*.html")
                .to_str()
                .ok_or_else(|| Error::string("invalid blob"))?,
        )?;
        let ctx = tera::Context::default();
        Ok(Self {
            tera,
            default_context: ctx,
        })
    }

    pub fn render<S: Serialize>(&self, key: &str, data: S) -> Result<String> {
        let context = tera::Context::from_serialize(data)?;
        Ok(self.tera.render(key, &context)?)
    }
}
