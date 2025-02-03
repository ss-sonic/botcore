use thiserror::Error;

#[derive(Error, Debug)]
pub enum BotError {
    #[error("Collector error: {message}")]
    CollectorError {
        message: String,
        source: Option<Box<dyn std::error::Error + Send + Sync>>,
    },

    #[error("Strategy error: {message}")]
    StrategyError {
        message: String,
        source: Option<Box<dyn std::error::Error + Send + Sync>>,
    },

    #[error("Executor error: {message}")]
    ExecutorError {
        message: String,
        source: Option<Box<dyn std::error::Error + Send + Sync>>,
    },

    #[error("Channel error: {message}")]
    ChannelError {
        message: String,
        source: Option<Box<dyn std::error::Error + Send + Sync>>,
    },

    #[error(transparent)]
    Other(#[from] anyhow::Error),
}

pub type Result<T> = std::result::Result<T, BotError>;

impl BotError {
    pub fn collector_error(message: impl Into<String>) -> Self {
        Self::CollectorError {
            message: message.into(),
            source: None,
        }
    }

    pub fn collector_error_with_source(
        message: impl Into<String>,
        source: impl Into<Box<dyn std::error::Error + Send + Sync>>,
    ) -> Self {
        Self::CollectorError {
            message: message.into(),
            source: Some(source.into()),
        }
    }

    pub fn strategy_error(message: impl Into<String>) -> Self {
        Self::StrategyError {
            message: message.into(),
            source: None,
        }
    }

    pub fn strategy_error_with_source(
        message: impl Into<String>,
        source: impl Into<Box<dyn std::error::Error + Send + Sync>>,
    ) -> Self {
        Self::StrategyError {
            message: message.into(),
            source: Some(source.into()),
        }
    }

    pub fn executor_error(message: impl Into<String>) -> Self {
        Self::ExecutorError {
            message: message.into(),
            source: None,
        }
    }

    pub fn executor_error_with_source(
        message: impl Into<String>,
        source: impl Into<Box<dyn std::error::Error + Send + Sync>>,
    ) -> Self {
        Self::ExecutorError {
            message: message.into(),
            source: Some(source.into()),
        }
    }

    pub fn channel_error(message: impl Into<String>) -> Self {
        Self::ChannelError {
            message: message.into(),
            source: None,
        }
    }

    pub fn channel_error_with_source(
        message: impl Into<String>,
        source: impl Into<Box<dyn std::error::Error + Send + Sync>>,
    ) -> Self {
        Self::ChannelError {
            message: message.into(),
            source: Some(source.into()),
        }
    }
}
