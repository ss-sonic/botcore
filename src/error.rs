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

#[cfg(test)]
mod tests {
    use super::*;
    use std::error::Error;
    use std::fmt;

    #[derive(Debug)]
    struct TestError(String);

    impl fmt::Display for TestError {
        fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
            write!(f, "{}", self.0)
        }
    }

    impl Error for TestError {}

    #[test]
    fn test_collector_error() {
        let error = BotError::collector_error("test error");
        assert!(matches!(error, BotError::CollectorError { message, source } 
            if message == "test error" && source.is_none()));
        
        let source = TestError("source error".to_string());
        let error = BotError::collector_error_with_source("test error", source);
        assert!(matches!(error, BotError::CollectorError { message, source: Some(_) } 
            if message == "test error"));
    }

    #[test]
    fn test_strategy_error() {
        let error = BotError::strategy_error("test error");
        assert!(matches!(error, BotError::StrategyError { message, source }
            if message == "test error" && source.is_none()));
        
        let source = TestError("source error".to_string());
        let error = BotError::strategy_error_with_source("test error", source);
        assert!(matches!(error, BotError::StrategyError { message, source: Some(_) }
            if message == "test error"));
    }

    #[test]
    fn test_executor_error() {
        let error = BotError::executor_error("test error");
        assert!(matches!(error, BotError::ExecutorError { message, source }
            if message == "test error" && source.is_none()));
        
        let source = TestError("source error".to_string());
        let error = BotError::executor_error_with_source("test error", source);
        assert!(matches!(error, BotError::ExecutorError { message, source: Some(_) }
            if message == "test error"));
    }

    #[test]
    fn test_channel_error() {
        let error = BotError::channel_error("test error");
        assert!(matches!(error, BotError::ChannelError { message, source }
            if message == "test error" && source.is_none()));
        
        let source = TestError("source error".to_string());
        let error = BotError::channel_error_with_source("test error", source);
        assert!(matches!(error, BotError::ChannelError { message, source: Some(_) }
            if message == "test error"));
    }

    #[test]
    fn test_error_conversion() {
        let anyhow_error = anyhow::anyhow!("test error");
        let error: BotError = anyhow_error.into();
        assert!(matches!(error, BotError::Other(_)));
    }

    #[test]
    fn test_error_display() {
        let error = BotError::collector_error("test error");
        assert_eq!(error.to_string(), "Collector error: test error");

        let error = BotError::strategy_error("test error");
        assert_eq!(error.to_string(), "Strategy error: test error");

        let error = BotError::executor_error("test error");
        assert_eq!(error.to_string(), "Executor error: test error");

        let error = BotError::channel_error("test error");
        assert_eq!(error.to_string(), "Channel error: test error");
    }
}
