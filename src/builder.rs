use crate::{
    config::{MulterConfig, Selector, UnknownFieldPolicy},
    error::ConfigError,
    limits::Limits,
};

/// Builder for configuring a `Multer` instance.
#[derive(Debug, Clone, Default)]
pub struct MulterBuilder {
    config: MulterConfig,
}

impl MulterBuilder {
    /// Creates a builder with default configuration.
    pub fn new() -> Self {
        Self::default()
    }

    /// Returns the current builder configuration snapshot.
    pub fn config(&self) -> &MulterConfig {
        &self.config
    }

    /// Replaces the full builder configuration.
    pub fn with_config(mut self, config: MulterConfig) -> Self {
        self.config = config;
        self
    }

    /// Sets the active file field selector strategy.
    pub fn selector(mut self, selector: Selector) -> Self {
        self.config.selector = selector;
        self
    }

    /// Sets how unknown fields should be handled.
    pub fn unknown_field_policy(mut self, policy: UnknownFieldPolicy) -> Self {
        self.config.unknown_field_policy = policy;
        self
    }

    /// Sets global multipart limits.
    pub fn limits(mut self, limits: Limits) -> Self {
        self.config.limits = limits;
        self
    }

    /// Validates builder configuration.
    pub fn validate(&self) -> Result<(), ConfigError> {
        self.config.validate()
    }

    /// Finalizes and returns validated configuration.
    pub fn build_config(self) -> Result<MulterConfig, ConfigError> {
        self.config.validate()?;
        Ok(self.config)
    }
}
