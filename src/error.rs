use anyhow;
use thiserror::Error;

/// Represents all possible errors that can occur in the bot engine.
///
/// This enum provides structured error types for different components of the system:
/// - Collector errors (event source issues)
/// - Strategy errors (event processing issues)
/// - Executor errors (action execution issues)
/// - Channel errors (communication issues)
#[derive(Error, Debug)]
pub enum BotError {
    /// An error that occurred in a collector component.
    #[error("Collector error: {message}")]
    CollectorError {
        /// A description of what went wrong
        message: String,
        /// The underlying error that caused this failure, if any
        source: Option<Box<dyn std::error::Error + Send + Sync>>,
    },

    /// An error that occurred in a strategy component.
    #[error("Strategy error: {message}")]
    StrategyError {
        /// A description of what went wrong
        message: String,
        /// The underlying error that caused this failure, if any
        source: Option<Box<dyn std::error::Error + Send + Sync>>,
    },

    /// An error that occurred in an executor component.
    #[error("Executor error: {message}")]
    ExecutorError {
        /// A description of what went wrong
        message: String,
        /// The underlying error that caused this failure, if any
        source: Option<Box<dyn std::error::Error + Send + Sync>>,
    },

    /// An error that occurred in the channel communication system.
    #[error("Channel error: {message}")]
    ChannelError {
        /// A description of what went wrong
        message: String,
        /// The underlying error that caused this failure, if any
        source: Option<Box<dyn std::error::Error + Send + Sync>>,
    },

    /// A catch-all variant for errors that don't fit into other categories.
    #[error(transparent)]
    Other(#[from] anyhow::Error),
}

/// A specialized Result type for bot operations.
pub type Result<T> = std::result::Result<T, BotError>;

impl BotError {
    /// Creates a new collector error with just a message.
    ///
    /// # Arguments
    ///
    /// * `message` - A description of what went wrong
    pub fn collector_error(message: impl Into<String>) -> Self {
        Self::CollectorError {
            message: message.into(),
            source: None,
        }
    }

    /// Creates a new collector error with both a message and an underlying cause.
    ///
    /// # Arguments
    ///
    /// * `message` - A description of what went wrong
    /// * `source` - The underlying error that caused this failure
    pub fn collector_error_with_source(
        message: impl Into<String>,
        source: impl Into<Box<dyn std::error::Error + Send + Sync>>,
    ) -> Self {
        Self::CollectorError {
            message: message.into(),
            source: Some(source.into()),
        }
    }

    /// Creates a new strategy error with just a message.
    ///
    /// # Arguments
    ///
    /// * `message` - A description of what went wrong
    pub fn strategy_error(message: impl Into<String>) -> Self {
        Self::StrategyError {
            message: message.into(),
            source: None,
        }
    }

    /// Creates a new strategy error with both a message and an underlying cause.
    ///
    /// # Arguments
    ///
    /// * `message` - A description of what went wrong
    /// * `source` - The underlying error that caused this failure
    pub fn strategy_error_with_source(
        message: impl Into<String>,
        source: impl Into<Box<dyn std::error::Error + Send + Sync>>,
    ) -> Self {
        Self::StrategyError {
            message: message.into(),
            source: Some(source.into()),
        }
    }

    /// Creates a new executor error with just a message.
    ///
    /// # Arguments
    ///
    /// * `message` - A description of what went wrong
    pub fn executor_error(message: impl Into<String>) -> Self {
        Self::ExecutorError {
            message: message.into(),
            source: None,
        }
    }

    /// Creates a new executor error with both a message and an underlying cause.
    ///
    /// # Arguments
    ///
    /// * `message` - A description of what went wrong
    /// * `source` - The underlying error that caused this failure
    pub fn executor_error_with_source(
        message: impl Into<String>,
        source: impl Into<Box<dyn std::error::Error + Send + Sync>>,
    ) -> Self {
        Self::ExecutorError {
            message: message.into(),
            source: Some(source.into()),
        }
    }

    /// Creates a new channel error with just a message.
    ///
    /// # Arguments
    ///
    /// * `message` - A description of what went wrong
    pub fn channel_error(message: impl Into<String>) -> Self {
        Self::ChannelError {
            message: message.into(),
            source: None,
        }
    }

    /// Creates a new channel error with both a message and an underlying cause.
    ///
    /// # Arguments
    ///
    /// * `message` - A description of what went wrong
    /// * `source` - The underlying error that caused this failure
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
        assert!(
            matches!(error, BotError::CollectorError { message, source } 
            if message == "test error" && source.is_none())
        );

        let source = TestError("source error".to_string());
        let error = BotError::collector_error_with_source("test error", source);
        assert!(
            matches!(error, BotError::CollectorError { message, source: Some(_) }
            if message == "test error")
        );
    }

    #[test]
    fn test_strategy_error() {
        let error = BotError::strategy_error("test error");
        assert!(matches!(error, BotError::StrategyError { message, source }
            if message == "test error" && source.is_none()));

        let source = TestError("source error".to_string());
        let error = BotError::strategy_error_with_source("test error", source);
        assert!(
            matches!(error, BotError::StrategyError { message, source: Some(_) }
            if message == "test error")
        );
    }

    #[test]
    fn test_executor_error() {
        let error = BotError::executor_error("test error");
        assert!(matches!(error, BotError::ExecutorError { message, source }
            if message == "test error" && source.is_none()));

        let source = TestError("source error".to_string());
        let error = BotError::executor_error_with_source("test error", source);
        assert!(
            matches!(error, BotError::ExecutorError { message, source: Some(_) }
            if message == "test error")
        );
    }

    #[test]
    fn test_channel_error() {
        let error = BotError::channel_error("test error");
        assert!(matches!(error, BotError::ChannelError { message, source }
            if message == "test error" && source.is_none()));

        let source = TestError("source error".to_string());
        let error = BotError::channel_error_with_source("test error", source);
        assert!(
            matches!(error, BotError::ChannelError { message, source: Some(_) }
            if message == "test error")
        );
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
