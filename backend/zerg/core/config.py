"""Configuration classes for different application environments.

These classes define the dependencies and settings for production, testing,
and development environments while maintaining clean separation.
"""

from __future__ import annotations

import os
from abc import ABC
from abc import abstractmethod
from dataclasses import dataclass
from typing import Optional

from zerg.core.implementations import ProductionAuthProvider
from zerg.core.implementations import ProductionEventBus
from zerg.core.implementations import ProductionModelRegistry
from zerg.core.implementations import SQLAlchemyDatabase
from zerg.core.interfaces import AuthProvider
from zerg.core.interfaces import Database
from zerg.core.interfaces import EventBus
from zerg.core.interfaces import ModelRegistry
from zerg.core.test_implementations import InMemoryEventBus
from zerg.core.test_implementations import IsolatedSQLiteDatabase
from zerg.core.test_implementations import MockModelRegistry
from zerg.core.test_implementations import TestAuthProvider


@dataclass
class AppConfig(ABC):
    """Abstract base configuration for the application."""

    @abstractmethod
    def create_database(self) -> Database:
        """Create database instance."""
        pass

    @abstractmethod
    def create_auth_provider(self) -> AuthProvider:
        """Create authentication provider."""
        pass

    @abstractmethod
    def create_model_registry(self) -> ModelRegistry:
        """Create model registry."""
        pass

    @abstractmethod
    def create_event_bus(self) -> EventBus:
        """Create event bus."""
        pass


@dataclass
class ProductionConfig(AppConfig):
    """Production configuration using real infrastructure."""

    database_url: str
    jwt_secret: str
    openai_api_key: str
    cors_origins: list[str]

    @classmethod
    def from_env(cls) -> ProductionConfig:
        """Create configuration from environment variables."""
        return cls(
            database_url=os.getenv("DATABASE_URL", "sqlite:///./app.db"),
            jwt_secret=os.getenv("JWT_SECRET", "dev-secret"),
            openai_api_key=os.getenv("OPENAI_API_KEY", ""),
            cors_origins=os.getenv("ALLOWED_CORS_ORIGINS", "").split(","),
        )

    def create_database(self) -> Database:
        """Create production database."""
        return SQLAlchemyDatabase()

    def create_auth_provider(self) -> AuthProvider:
        """Create production authentication provider."""
        return ProductionAuthProvider()

    def create_model_registry(self) -> ModelRegistry:
        """Create production model registry."""
        return ProductionModelRegistry()

    def create_event_bus(self) -> EventBus:
        """Create production event bus."""
        return ProductionEventBus()


@dataclass
class TestConfig(AppConfig):
    """Test configuration using isolated, controlled infrastructure."""

    worker_id: str
    db_path: Optional[str] = None
    mock_responses: Optional[dict] = None

    @classmethod
    def for_worker(cls, worker_id: str) -> TestConfig:
        """Create test configuration for specific worker."""
        return cls(worker_id=worker_id)

    def create_database(self) -> Database:
        """Create isolated test database."""
        return IsolatedSQLiteDatabase(self.worker_id, self.db_path)

    def create_auth_provider(self) -> AuthProvider:
        """Create test authentication provider."""
        return TestAuthProvider()

    def create_model_registry(self) -> ModelRegistry:
        """Create mock model registry."""
        return MockModelRegistry()

    def create_event_bus(self) -> EventBus:
        """Create in-memory event bus."""
        return InMemoryEventBus()


@dataclass
class DevelopmentConfig(AppConfig):
    """Development configuration mixing real and mock services."""

    database_url: str = "sqlite:///./dev.db"
    use_mock_auth: bool = True
    use_mock_models: bool = True

    def create_database(self) -> Database:
        """Create development database."""
        return SQLAlchemyDatabase()

    def create_auth_provider(self) -> AuthProvider:
        """Create development authentication provider."""
        if self.use_mock_auth:
            return TestAuthProvider()
        return ProductionAuthProvider()

    def create_model_registry(self) -> ModelRegistry:
        """Create development model registry."""
        if self.use_mock_models:
            return MockModelRegistry()
        return ProductionModelRegistry()

    def create_event_bus(self) -> EventBus:
        """Create development event bus."""
        return InMemoryEventBus()  # Faster for development


def load_config() -> AppConfig:
    """Load configuration based on environment."""
    environment = os.getenv("ENVIRONMENT", "production")

    if environment == "test":
        worker_id = os.getenv("TEST_WORKER_ID", "0")
        return TestConfig.for_worker(worker_id)
    elif environment == "development":
        return DevelopmentConfig()
    else:
        return ProductionConfig.from_env()
